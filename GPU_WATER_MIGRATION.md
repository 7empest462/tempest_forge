# Water Physics GPU Migration Guide

## Overview

You now have the infrastructure for GPU-accelerated water physics. The current system has:

✅ **Completed:**
- Compute shader implementation (`assets/shaders/water_compute.wgsl`)
- GPU buffer infrastructure (`src/world/water_gpu.rs`)
- CPU cache for gameplay queries (heights stay synchronized)
- Working water physics (CPU-side, for compatibility)

**Next Steps:** Integrate compute shader dispatch into Bevy's render pipeline

---

## Architecture

### Current Layout
```
CPU World                          GPU Rendering Context
┌──────────────────────┐          ┌──────────────────────┐
│ water_sim()          │  ┌─────→ │ WaterGpuBuffers      │
│ Updates heights      │  │       │ - height_current     │
│ (shallow water eqs)  │  │       │ - height_next        │
└──────────────────────┘  │       │ - flow_x, flow_y     │
                          │       │ - wall_mask          │
┌──────────────────────┐  │       └──────────────────────┘
│ animate_water_mesh() │  │              ↓
│ Updates mesh from    │  │       ┌──────────────────────┐
│ height cache         │  └─────── │ Readback to CPU      │
└──────────────────────┘          │ for next frame       │
                                   └──────────────────────┘
```

### Compute Shader Passes
The compute shader runs 3 sequential passes per frame:

1. **water_flow_pass** - Calculate flow velocities based on height differences
   - Input: height_current, wall_mask, params
   - Output: flow_x_next, flow_y_next
   
2. **water_outflow_pass** - Constrain total outflow to prevent instability
   - Input: height_current, flow_x_next, flow_y_next, wall_mask
   - Output: scaled flow_x_next, flow_y_next
   
3. **water_height_pass** - Update heights from flows
   - Input: height_current, flow_x_next, flow_y_next, wall_mask
   - Output: height_next

After each frame, heights are swapped: `height_next → height_current`

---

## Step-by-Step GPU Integration

### Step 1: Create Render Plugin

Add a new file `src/world/water_render_plugin.rs`:

```rust
use bevy::prelude::*;
use bevy::render::RenderApp;
use crate::world::water_gpu::WaterGpuBuffers;

pub struct WaterRenderPlugin;

impl Plugin for WaterRenderPlugin {
    fn build(&self, app: &mut App) {
        // Insert GPU buffers resource
        let render_app = app.sub_app_mut(RenderApp);
        
        render_app.init_resource::<WaterGpuBuffers>();
    }
}
```

### Step 2: Setup GPU Buffers on Startup

In your main `water.rs` WaterPlugin:

```rust
impl Plugin for WaterPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins(MaterialPlugin::<WaterMaterial>::default())
            .add_plugins(water_render_plugin::WaterRenderPlugin)
            .add_systems(Startup, (
                setup_water,
                initialize_gpu_buffers.in_set(RenderSet::Prepare),
            ))
            // ... rest of systems
    }
}

fn initialize_gpu_buffers(
    render_device: Res<RenderDevice>,
    mut commands: Commands,
) {
    let buffers = WaterGpuBuffers::new(&render_device);
    commands.insert_resource(buffers);
}
```

### Step 3: Create Compute Dispatch System

Add to `src/world/water_gpu.rs`:

```rust
use bevy::render::render_graph::{Node, RenderGraphContext, NodeRunError};
use bevy::render::renderer::RenderContext;

pub struct WaterComputeNode;

impl Node for WaterComputeNode {
    fn run(
        &self,
        graph: &mut RenderGraphContext,
        render_context: &mut RenderContext,
        world: &World,
    ) -> Result<(), NodeRunError> {
        let pipeline_cache = world.resource::<PipelineCache>();
        let water_pipeline = world.resource::<WaterComputePipeline>();
        let buffers = world.resource::<WaterGpuBuffers>();
        
        // Dispatch three compute shader passes
        // Pass 1: water_flow_pass
        // Pass 2: water_outflow_pass  
        // Pass 3: water_height_pass
        
        // Swap buffers: height_next → height_current
        
        Ok(())
    }
}
```

### Step 4: Add Render Graph Integration

Register the compute node in your render graph:

```rust
let mut render_graph = world.resource_mut::<RenderGraph>();
render_graph.add_node(WATER_COMPUTE_LABEL, water_compute_node);
render_graph.add_node_edge(WATER_COMPUTE_LABEL, bevy::render::graph::node::MAIN_PASS_DEPENDENCIES);
```

### Step 5: Readback Heights for CPU

After GPU compute, copy heights back to CPU cache:

```rust
fn readback_gpu_heights(
    render_device: Res<RenderDevice>,
    gpu_buffers: Res<WaterGpuBuffers>,
    mut water_sim: Query<&mut WaterSimData>,
) {
    // Use render_device to read GPU buffer
    // Update water_sim.height with latest values
}
```

### Step 6: Update Dispatch Parameters

Modify params each frame based on delta time:

```rust
fn update_water_params(
    time: Res<Time>,
    render_device: Res<RenderDevice>,
    gpu_buffers: Res<WaterGpuBuffers>,
) {
    let params = WaterSimParams {
        delta_time: time.delta_secs().min(0.03),
        gravity: 12.0,
        friction: 0.975,
        padding: 0.0,
    };
    
    render_device.queue_buffer_write(
        &gpu_buffers.params_buffer,
        0,
        bytemuck::cast_slice(&[params]),
    );
}
```

### Step 7: Sync Wall Mask to GPU

Update wall mask when terrain changes:

```rust
fn update_gpu_wall_mask(
    render_device: Res<RenderDevice>,
    gpu_buffers: Res<WaterGpuBuffers>,
    water_query: Query<&WaterSimData>,
) {
    for water_data in water_query.iter() {
        // Pack bool array to bits
        let packed = pack_wall_mask(&water_data.wall_mask);
        
        render_device.queue_buffer_write(
            &gpu_buffers.wall_mask,
            0,
            bytemuck::cast_slice(&packed),
        );
    }
}

fn pack_wall_mask(mask: &[bool]) -> Vec<u32> {
    let mut packed = vec![0u32; (mask.len() + 31) / 32];
    for (i, &is_wall) in mask.iter().enumerate() {
        if is_wall {
            let idx = i / 32;
            let bit = i % 32;
            packed[idx] |= 1 << bit;
        }
    }
    packed
}
```

---

## Optimization Tips

### 1. **Workgroup Size Tuning**
The compute shader uses 8x8 workgroups. For 128x128 grid:
- 16x16 workgroups (2048 threads total)
- Good balance for most GPUs
- Adjust if performance issues: try 16x16 or 32x32

### 2. **Double Buffering**
Heights use ping-pong buffers (height_current ↔ height_next) to avoid write-after-read hazards.

### 3. **Barrier Synchronization**
Between passes, use `workgroupBarrier()` in WGSL to synchronize all threads in a workgroup.

### 4. **Bank Conflicts**
If using shared memory (future optimization), pad to avoid bank conflicts:
```wgsl
// Pad to 9 elements to avoid bank conflicts
var<workgroup> shared_data: array<f32, 9>;
```

### 5. **Coalesced Memory Access**
Thread layout matches memory layout for coalesced reads:
```wgsl
// Global ID directly maps to grid position
let x = global_id.x;
let y = global_id.y;
```

---

## Performance Expectations

### Before (CPU)
- 128x128 grid: ~2-4ms per frame
- Single-threaded CPU computation
- ~60% of frame budget on water alone

### After (GPU)
- **Target:** <0.5ms per frame
- Parallelized across thousands of GPU cores
- <10% of frame budget
- **Speedup: 4-8x** expected

---

## Testing Checklist

- [ ] Compute shader compiles without errors
- [ ] GPU buffers initialize correctly
- [ ] Three dispatch passes execute in order
- [ ] Height readback updates CPU cache
- [ ] Wall mask synchronizes with terrain changes
- [ ] Interaction forces still apply correctly
- [ ] Mesh updates match GPU heights
- [ ] No memory leaks (persistent buffers)
- [ ] Performance improvement measurable

---

## Debugging Tools

### GPU Trace Analysis
Use your GPU vendor's profiling tools:
- **NVIDIA:** NSight Systems/Graphics
- **AMD:** Radeon GPU Profiler
- **Intel:** oneAPI Level-Zero Tools

### Common Issues
1. **Shader compilation fails:** Check shader syntax, ensure WGSL 1.0 compatible
2. **Buffer size mismatch:** Verify buffer byte sizes match `f32` counts
3. **No visible updates:** Check readback timing, ensure passes execute in order
4. **Performance degradation:** Profile dispatch overhead, consider batching

---

## References

- Bevy Render Documentation: https://docs.rs/bevy/latest/bevy/render/
- WGSL Specification: https://gpuweb.github.io/gpuweb/wgsl/
- Shallow Water Equations: [GPU Gems 2 Chapter 15](https://developer.nvidia.com/gpugems/gpugems2/part-ii-simulation/chapter-15-gpu-supported-instancing-and-geometry)

---

## Next Steps After GPU Migration

Once GPU compute is working:

1. **Increase Grid Resolution:** Move to 256x256 or 512x512
2. **Shared Memory Optimization:** Cache heights in workgroup memory
3. **Indirect Dispatch:** Use indirect buffers for dynamic workgroup counts
4. **Async Readback:** Pipeline readback to avoid GPU stalls
5. **Async Compute:** Run water simulation on separate queue
