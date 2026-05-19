#import bevy_pbr::forward_io::VertexOutput
#import bevy_pbr::mesh_view_bindings::view

// Lighting data
const LIGHT_POSITION: vec3<f32> = vec3<f32>(20.0, 30.0, -20.0);
const LIGHT_COLOR: vec3<f32> = vec3<f32>(1.0, 0.85, 0.7);
const LIGHT_INTENSITY: f32 = 50000.0;

const PI: f32 = 3.141592;

struct WaterMaterial {
    color: vec4<f32>,
    time: f32,
    camera_position: vec3<f32>,
    resolution: vec2<f32>,
    water_level: f32,
    grid_scale: f32,
};

@group(3) @binding(0) var<uniform> material: WaterMaterial;

fn get_sky_color(rd: vec3<f32>) -> vec3<f32> {
    let y = max(rd.y, 0.0);
    // Gorgeous sky gradient from zenith deep blue to horizon warm cyan
    let sky_zenith = vec3<f32>(0.05, 0.15, 0.4);
    let sky_horizon = vec3<f32>(0.35, 0.65, 0.85);
    return mix(sky_horizon, sky_zenith, pow(y, 0.6));
}

fn diffuse(n: vec3<f32>, l: vec3<f32>, p: f32) -> f32 {
    return pow(dot(n, l) * 0.4 + 0.6, p);
}

fn specular(n: vec3<f32>, l: vec3<f32>, e: vec3<f32>, s: f32) -> f32 {
    let nrm = (s + 8.0) / (PI * 8.0);
    return pow(max(dot(reflect(-l, n), e), 0.0), s) * nrm;
}

fn get_normal_from_derivatives(world_normal: vec3<f32>, world_pos: vec3<f32>) -> vec3<f32> {
    var normal = normalize(world_normal);
    if normal.y < 0.0 {
        normal = -normal;
    }
    return normal;
}

@fragment
fn fragment(in: VertexOutput) -> @location(0) vec4<f32> {
    let world_pos = in.world_position.xyz;
    
    // Calculate view direction (from surface to camera)
    let eye_dir = normalize(material.camera_position - world_pos);
    
    // Light direction (normalized)
    let light_dir = normalize(LIGHT_POSITION);
    
    // Get normal from vertex data
    let normal = get_normal_from_derivatives(in.world_normal, world_pos);
    
    // Enhanced water color variation - more noticeable depth changes
    // Relative height factor (mesh base level is at y = 15.0)
    let height_factor = (world_pos.y - material.water_level + 2.0) / 4.0;
    
    // Curated rich water color palette - Vivid Sapphire / Royal Blue
    let water_deep = vec3<f32>(0.005, 0.05, 0.18);     // Deep rich navy/indigo
    let water_shallow = vec3<f32>(0.02, 0.32, 0.78);   // Gorgeous vivid sapphire blue
    
    let simple_water = mix(water_deep, water_shallow, clamp(height_factor, 0.0, 1.0));
    
    // Lighting with contrast
    let ndotl = max(dot(normal, light_dir), 0.0);
    let lit_water = simple_water * (0.4 + 0.8 * ndotl);
    
    // Depth-based darkening
    let depth_darkening = smoothstep(0.0, 0.6, 1.0 - height_factor);
    let darkened_water = mix(lit_water, lit_water * 0.25, depth_darkening);
    
    // Fresnel reflection
    let fresnel = pow(1.0 - max(dot(normal, eye_dir), 0.0), 3.0);
    let sky_color = get_sky_color(reflect(-eye_dir, normal));
    
    // Add subtle animated micro-waves using sine waves in shader
    let uv_coords = world_pos.xz * 0.2;
    let wave_osc = sin(uv_coords.x + material.time) * cos(uv_coords.y - material.time) * 0.05;
    let final_fresnel = clamp(fresnel + wave_osc, 0.0, 0.9);
    
    var final_color = mix(darkened_water, sky_color, final_fresnel * 0.45);
    
    // Add sun specular highlights
    let spec = specular(normal, light_dir, eye_dir, 120.0);
    final_color = final_color + LIGHT_COLOR * spec * 1.2;
    
    return vec4<f32>(final_color, material.color.a);
}
