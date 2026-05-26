use bevy::{
    prelude::*,
    render::{
        extract_resource::{ExtractResource, ExtractResourcePlugin},
        render_resource::{binding_types::uniform_buffer, *},
        renderer::{RenderDevice, RenderQueue},
        Render, RenderApp, RenderStartup, RenderSystems,
    },
};
use std::borrow::Cow;

use crate::world::water_gpu::{WaterSimParams, WATER_GRID_SIZE, WORKGROUP_SIZE};

const SHADER_ASSET_PATH: &str = "shaders/water_compute.wgsl";
pub struct WaterRenderPlugin;

impl Plugin for WaterRenderPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins((
            ExtractResourcePlugin::<WaterSimParamsUniform>::default(),
        ));

        let render_app = app.sub_app_mut(RenderApp);
            
        render_app
            .init_resource::<WaterComputeState>()
            .add_systems(RenderStartup, init_water_pipeline)
            .add_systems(
                Render,
                prepare_water_bind_group.in_set(RenderSystems::PrepareBindGroups),
            )
            .add_systems(Render, update_water_state.in_set(RenderSystems::Prepare))
            .add_systems(Render, execute_water_compute_pass.in_set(RenderSystems::Render));
    }
}

/// Physics parameters extracted to render world
#[derive(Resource, Clone, ExtractResource, ShaderType)]
struct WaterSimParamsUniform {
    pub delta_time: f32,
    pub gravity: f32,
    pub friction: f32,
    pub padding: f32,
}

impl From<WaterSimParams> for WaterSimParamsUniform {
    fn from(params: WaterSimParams) -> Self {
        Self {
            delta_time: params.delta_time,
            gravity: params.gravity,
            friction: params.friction,
            padding: 0.0,
        }
    }
}

/// Bind groups for the three compute passes
#[derive(Resource)]
struct WaterComputeBindGroups {
    flow_pass: BindGroup,
    outflow_pass: BindGroup,
    height_pass: BindGroup,
}

/// Pipeline with cached compute pipeline IDs for each pass
#[derive(Resource)]
struct WaterComputePipeline {
    bind_group_layout: BindGroupLayoutDescriptor,
    flow_pass_id: CachedComputePipelineId,
    outflow_pass_id: CachedComputePipelineId,
    height_pass_id: CachedComputePipelineId,
}

/// State machine for tracking pipeline loading
#[derive(Resource, Default)]
enum WaterComputeState {
    #[default]
    Loading,
    FlowPass,
    OutflowPass,
    HeightPass,
    Running,
}

fn init_water_pipeline(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    pipeline_cache: Res<PipelineCache>,
    render_device: Res<RenderDevice>,
) {
    // Define the bind group layout for all passes (uniform + storage buffers)
    let bind_group_layout = BindGroupLayoutDescriptor::new(
        "WaterComputeLayout",
        &BindGroupLayoutEntries::sequential(
            ShaderStages::COMPUTE,
            (uniform_buffer::<WaterSimParamsUniform>(false),),
        ),
    );

    // Load the shader from assets
    let shader = asset_server.load(SHADER_ASSET_PATH);

    // Queue the three compute pipelines
    let flow_pass_id = pipeline_cache.queue_compute_pipeline(ComputePipelineDescriptor {
        layout: vec![bind_group_layout.clone()],
        shader: shader.clone(),
        entry_point: Some(Cow::from("water_flow_pass")),
        ..default()
    });

    let outflow_pass_id = pipeline_cache.queue_compute_pipeline(ComputePipelineDescriptor {
        layout: vec![bind_group_layout.clone()],
        shader: shader.clone(),
        entry_point: Some(Cow::from("water_outflow_pass")),
        ..default()
    });

    let height_pass_id = pipeline_cache.queue_compute_pipeline(ComputePipelineDescriptor {
        layout: vec![bind_group_layout.clone()],
        shader,
        entry_point: Some(Cow::from("water_height_pass")),
        ..default()
    });

    commands.insert_resource(WaterComputePipeline {
        bind_group_layout,
        flow_pass_id,
        outflow_pass_id,
        height_pass_id,
    });

    info!("✅ Water compute pipelines initialized and queued for compilation");
}

fn prepare_water_bind_group(
    mut commands: Commands,
        pipeline: Res<WaterComputePipeline>,
    render_device: Res<RenderDevice>,
    pipeline_cache: Res<PipelineCache>,
    params: Res<WaterSimParamsUniform>,
    queue: Res<RenderQueue>,
) {
    // Create uniform buffer for physics parameters
    let mut uniform_buffer = UniformBuffer::from(params.into_inner());
    uniform_buffer.write_buffer(&render_device, &queue);

    // Get the actual BindGroupLayout from the descriptor via the cache
    let bind_group_layout = pipeline_cache.get_bind_group_layout(&pipeline.bind_group_layout);

    let flow_pass = render_device.create_bind_group(
        Some("water_flow_pass_bindgroup"),
        &bind_group_layout,
        &BindGroupEntries::sequential((&uniform_buffer,)),
    );

    let outflow_pass = render_device.create_bind_group(
        Some("water_outflow_pass_bindgroup"),
        &bind_group_layout,
        &BindGroupEntries::sequential((&uniform_buffer,)),
    );

    let height_pass = render_device.create_bind_group(
        Some("water_height_pass_bindgroup"),
        &bind_group_layout,
        &BindGroupEntries::sequential((&uniform_buffer,)),
    );

    commands.insert_resource(WaterComputeBindGroups {
        flow_pass,
        outflow_pass,
        height_pass,
    });

    info!("✅ Water compute bind groups prepared");
}

fn update_water_state(
    pipeline: Res<WaterComputePipeline>,
    pipeline_cache: Res<PipelineCache>,
    mut state: ResMut<WaterComputeState>,
) {
    // State machine to wait for each pipeline to compile before moving forward
    match *state {
        WaterComputeState::Loading => {
            match pipeline_cache.get_compute_pipeline_state(pipeline.flow_pass_id) {
                CachedPipelineState::Ok(_) => {
                    *state = WaterComputeState::FlowPass;
                    info!("✅ Flow pass pipeline compiled");
                }
                CachedPipelineState::Err(_) => {}
                _ => {}
            }
        }
        WaterComputeState::FlowPass => {
            match pipeline_cache.get_compute_pipeline_state(pipeline.outflow_pass_id) {
                CachedPipelineState::Ok(_) => {
                    *state = WaterComputeState::OutflowPass;
                    info!("✅ Outflow pass pipeline compiled");
                }
                CachedPipelineState::Err(err) => {
                    warn!("Water outflow pass compilation error: {err:?}")
                }
                _ => {}
            }
        }
        WaterComputeState::OutflowPass => {
            match pipeline_cache.get_compute_pipeline_state(pipeline.height_pass_id) {
                CachedPipelineState::Ok(_) => {
                    *state = WaterComputeState::HeightPass;
                    info!("✅ Height pass pipeline compiled");
                }
                CachedPipelineState::Err(err) => {
                    warn!("Water height pass compilation error: {err:?}")
                }
                _ => {}
            }
        }
        WaterComputeState::HeightPass => {
            *state = WaterComputeState::Running;
            info!("✅ All water compute pipelines ready - simulation starting!");
            info!("");
            info!("🌊 Water GPU Physics Active:");
            info!("   - Grid: 128×128 cells (16,384 total)");
            info!("   - Workgroups: 8×8 (16×16 dispatch)");
            info!("   - Passes: flow → outflow → height");
            info!("");
        }
        WaterComputeState::Running => {}
    }
}

fn execute_water_compute_pass(
    mut render_context: RenderContext,
    bind_groups: Res<WaterComputeBindGroups>,
    pipeline_cache: Res<PipelineCache>,
    pipeline: Res<WaterComputePipeline>,
    state: Res<WaterComputeState>,
) {
    // Only run if all pipelines are compiled and ready
    if !matches!(*state, WaterComputeState::Running) {
        return;
    }

    let mut pass = render_context
        .command_encoder()
        .begin_compute_pass(&ComputePassDescriptor::default());

    let workgroups_x = (WATER_GRID_SIZE + WORKGROUP_SIZE - 1) / WORKGROUP_SIZE;
    let workgroups_y = (WATER_GRID_SIZE + WORKGROUP_SIZE - 1) / WORKGROUP_SIZE;

    // Pass 1: Flow calculation
    if let Some(flow_pipeline) = pipeline_cache.get_compute_pipeline(pipeline.flow_pass_id) {
        pass.set_bind_group(0, &bind_groups.flow_pass, &[]);
        pass.set_pipeline(flow_pipeline);
        pass.dispatch_workgroups(workgroups_x, workgroups_y, 1);
    }

    // Pass 2: Outflow scaling
    if let Some(outflow_pipeline) = pipeline_cache.get_compute_pipeline(pipeline.outflow_pass_id) {
        pass.set_bind_group(0, &bind_groups.outflow_pass, &[]);
        pass.set_pipeline(outflow_pipeline);
        pass.dispatch_workgroups(workgroups_x, workgroups_y, 1);
    }

    // Pass 3: Height update
    if let Some(height_pipeline) = pipeline_cache.get_compute_pipeline(pipeline.height_pass_id) {
        pass.set_bind_group(0, &bind_groups.height_pass, &[]);
        pass.set_pipeline(height_pipeline);
        pass.dispatch_workgroups(workgroups_x, workgroups_y, 1);
    }
}

// Render graph node implementation for water compute pass
impl bevy::render::render_graph::Node for WaterComputeNode {
    fn update(&mut self, _world: &mut bevy::ecs::world::World) {}

    fn run(
        &self,
        _graph: &mut bevy::render::render_graph::RenderGraphContext,
        render_context: &mut RenderContext,
        world: &bevy::ecs::world::World,
    ) -> Result<(), bevy::render::render_graph::NodeRunError> {
        let bind_groups = world.resource::<WaterComputeBindGroups>();
        let pipeline_cache = world.resource::<PipelineCache>();
        let pipeline = world.resource::<WaterComputePipeline>();
        let state = world.resource::<WaterComputeState>();

        // Only run if all pipelines are compiled and ready
        if !matches!(*state, WaterComputeState::Running) {
            return Ok(());
        }

        let mut pass = render_context
            .command_encoder()
            .begin_compute_pass(&ComputePassDescriptor::default());

        let workgroups_x = (WATER_GRID_SIZE + WORKGROUP_SIZE - 1) / WORKGROUP_SIZE;
        let workgroups_y = (WATER_GRID_SIZE + WORKGROUP_SIZE - 1) / WORKGROUP_SIZE;

        // Pass 1: Flow calculation
        if let Some(flow_pipeline) = pipeline_cache.get_compute_pipeline(pipeline.flow_pass_id) {
            pass.set_bind_group(0, &bind_groups.flow_pass, &[]);
            pass.set_pipeline(flow_pipeline);
            pass.dispatch_workgroups(workgroups_x, workgroups_y, 1);
        }

        // Pass 2: Outflow scaling
        if let Some(outflow_pipeline) = pipeline_cache.get_compute_pipeline(pipeline.outflow_pass_id) {
            pass.set_bind_group(0, &bind_groups.outflow_pass, &[]);
            pass.set_pipeline(outflow_pipeline);
            pass.dispatch_workgroups(workgroups_x, workgroups_y, 1);
        }

        // Pass 3: Height update
        if let Some(height_pipeline) = pipeline_cache.get_compute_pipeline(pipeline.height_pass_id) {
            pass.set_bind_group(0, &bind_groups.height_pass, &[]);
            pass.set_pipeline(height_pipeline);
            pass.dispatch_workgroups(workgroups_x, workgroups_y, 1);
        }

        Ok(())
    }
}