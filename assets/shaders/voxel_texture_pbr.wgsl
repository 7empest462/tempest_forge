#import bevy_pbr::{
    pbr_fragment::pbr_input_from_standard_material,
    pbr_functions::alpha_discard,
    mesh_functions,
    view_transformations::position_world_to_clip
}
#import bevy_pbr::pbr_bindings
#import bevy_render::instance_index::get_instance_index

#ifdef PREPASS_PIPELINE
#import bevy_pbr::{
    prepass_io::{VertexOutput, FragmentOutput},
    pbr_deferred_functions::deferred_output,
}
#else
#import bevy_pbr::{
    forward_io::{VertexOutput, FragmentOutput},
    pbr_functions::{apply_pbr_lighting, main_pass_post_lighting_processing},
}
#endif

@group(#{MATERIAL_BIND_GROUP}) @binding(100)
var mat_array_texture: texture_2d_array<f32>;

@group(#{MATERIAL_BIND_GROUP}) @binding(101)
var mat_array_texture_sampler: sampler;

@group(#{MATERIAL_BIND_GROUP}) @binding(102)
var mat_normal_array_texture: texture_2d_array<f32>;

@group(#{MATERIAL_BIND_GROUP}) @binding(103)
var mat_normal_array_texture_sampler: sampler;

@group(#{MATERIAL_BIND_GROUP}) @binding(104)
var mat_orm_array_texture: texture_2d_array<f32>;

@group(#{MATERIAL_BIND_GROUP}) @binding(105)
var mat_orm_array_texture_sampler: sampler;

struct Vertex {
    @builtin(instance_index) instance_index: u32,
#ifdef VERTEX_POSITIONS
    @location(0) position: vec3<f32>,
#endif
#ifdef VERTEX_NORMALS
    @location(1) normal: vec3<f32>,
#endif
#ifdef VERTEX_UVS
    @location(2) uv: vec2<f32>,
#endif
#ifdef VERTEX_UVS_B
    @location(3) uv_b: vec2<f32>,
#endif
#ifdef VERTEX_TANGENTS
    @location(4) tangent: vec4<f32>,
#endif
#ifdef VERTEX_COLORS
    @location(5) color: vec4<f32>,
#endif
// #ifdef SKINNED
//     @location(6) joint_indices: vec4<u32>,
//     @location(7) joint_weights: vec4<f32>,
// #endif
#ifdef MORPH_TARGETS
    @builtin(vertex_index) index: u32,
#endif

    @location(8) tex_idx: vec3<u32>
};

struct CustomVertexOutput {
    @builtin(position) position: vec4<f32>,
    @location(0) world_position: vec4<f32>,
    @location(1) world_normal: vec3<f32>,
#ifdef VERTEX_UVS
    @location(2) uv: vec2<f32>,
#endif
#ifdef VERTEX_UVS_B
    @location(3) uv_b: vec2<f32>,
#endif
#ifdef VERTEX_TANGENTS
    @location(4) world_tangent: vec4<f32>,
#endif
#ifdef VERTEX_COLORS
    @location(5) color: vec4<f32>,
#endif
#ifdef VERTEX_OUTPUT_INSTANCE_INDEX
    @location(6) @interpolate(flat) instance_index: u32,
#endif
    @location(8) @interpolate(flat) tex_idx: vec3<u32>,
}

@vertex
fn vertex(vertex: Vertex) -> CustomVertexOutput {
    var out: CustomVertexOutput;
    var model =  mesh_functions::get_world_from_local(vertex.instance_index);

    out.world_normal = mesh_functions::mesh_normal_local_to_world(
        vertex.normal, vertex.instance_index);

    out.world_position = mesh_functions::mesh_position_local_to_world(
        model, vec4<f32>(vertex.position, 1.0));

    out.position = position_world_to_clip(out.world_position.xyz);
        
#ifdef VERTEX_UVS
    out.uv = vertex.uv;
#endif

#ifdef VERTEX_TANGENTS
    out.world_tangent = mesh_functions::mesh_tangent_local_to_world(
        model,
        vertex.tangent,
        vertex.instance_index
    );
#endif

    out.color = vertex.color;

#ifdef VERTEX_OUTPUT_INSTANCE_INDEX
    out.instance_index = vertex.instance_index;
#endif

    out.tex_idx = vertex.tex_idx;

    return out;
}

@fragment
fn fragment(
    in: CustomVertexOutput,
    @builtin(front_facing) is_front: bool,
)  -> FragmentOutput {
    var standard_in: VertexOutput;
    standard_in.position = in.position;
    standard_in.world_normal = in.world_normal;
    standard_in.world_position = in.world_position;
    standard_in.uv = in.uv;
    standard_in.color = in.color;
#ifdef VERTEX_OUTPUT_INSTANCE_INDEX
    standard_in.instance_index = in.instance_index;
#else
    standard_in.instance_index = 0u;
#endif
    var pbr_input = pbr_input_from_standard_material(standard_in, is_front);
    var tex_face = 0;

    if in.world_normal.y == 0.0 {
        tex_face = 1;
    } else if in.world_normal.y < 0.0 {
        tex_face = 2;
    }

    let texture_idx = in.tex_idx[tex_face];

    if texture_idx >= 14u {
        pbr_input.material.base_color = textureSample(mat_array_texture, mat_array_texture_sampler, in.uv, texture_idx);
    } else {
        let ao_color = max(in.color, vec4<f32>(0.35, 0.35, 0.35, 1.0));
        pbr_input.material.base_color = textureSample(mat_array_texture, mat_array_texture_sampler, in.uv, texture_idx) * ao_color;
    }
    // Sample normal map (RGB)
    let raw_normal = textureSample(mat_normal_array_texture, mat_normal_array_texture_sampler, in.uv, texture_idx).rgb;
    
    // Sample ORM map (R: Occlusion, G: Roughness, B: Metallic)
    let orm = textureSample(mat_orm_array_texture, mat_orm_array_texture_sampler, in.uv, texture_idx);

    // Analytical TBN for axis-aligned voxel faces
    var T = vec3<f32>(1.0, 0.0, 0.0);
    var B = vec3<f32>(0.0, 0.0, 1.0);
    let N = normalize(in.world_normal);

    if abs(N.y) > 0.9 {
        T = vec3<f32>(1.0, 0.0, 0.0);
        B = vec3<f32>(0.0, 0.0, 1.0);
    } else if abs(N.x) > 0.9 {
        T = vec3<f32>(0.0, 0.0, 1.0);
        B = vec3<f32>(0.0, 1.0, 0.0);
    } else { // Z axis
        T = vec3<f32>(1.0, 0.0, 0.0);
        B = vec3<f32>(0.0, 1.0, 0.0);
    }

    // Transform normal map to world space
    let tangent_normal = raw_normal * 2.0 - 1.0;
    let world_normal = normalize(T * tangent_normal.x + B * tangent_normal.y + N * tangent_normal.z);
    
    pbr_input.world_normal = world_normal;
    pbr_input.material.perceptual_roughness = orm.g;
    pbr_input.material.metallic = orm.b;
    pbr_input.diffuse_occlusion = vec3<f32>(orm.r);

    // Apply custom emissive and dielectric properties for alien blocks
    if texture_idx == 16u {
        // Glowing Moss: bio-luminescent emerald green glow
        let moss_glow = vec4<f32>(0.1, 0.85, 0.45, 1.0);
        pbr_input.material.emissive = (pbr_input.material.base_color * 0.8 + moss_glow * 0.4) * 1.5;
    } else if texture_idx == 17u {
        // Alien Crystal: vibrant cyan crystalline glow (dielectric, non-metallic for full ambient illumination)
        let crystal_glow = vec4<f32>(0.2, 0.7, 1.2, 1.0);
        pbr_input.material.emissive = (pbr_input.material.base_color * 1.5 + crystal_glow * 0.8) * 3.0;
        pbr_input.material.metallic = 0.05;
        pbr_input.material.perceptual_roughness = 0.15;
    } else if texture_idx == 18u {
        // Floating Crystal: brilliant magenta/purple pulse (dielectric, non-metallic for full ambient illumination)
        let crystal_glow = vec4<f32>(0.9, 0.3, 1.3, 1.0);
        pbr_input.material.emissive = (pbr_input.material.base_color * 1.8 + crystal_glow * 1.0) * 3.5;
        pbr_input.material.metallic = 0.05;
        pbr_input.material.perceptual_roughness = 0.1;
    }

    pbr_input.material.base_color = alpha_discard(pbr_input.material, pbr_input.material.base_color);
#ifdef PREPASS_PIPELINE
    let out = deferred_output(in, pbr_input);
#else
    var out: FragmentOutput;
    out.color = apply_pbr_lighting(pbr_input);
    out.color = main_pass_post_lighting_processing(pbr_input, out.color);
#endif

    return out;
}
