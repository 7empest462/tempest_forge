// Water Material Shader — Flo-style ocean rendering
// reference https://www.shadertoy.com/view/MttfW8 (Seascape by TDM)

#import bevy_pbr::forward_io::VertexOutput
#import bevy_pbr::mesh_view_bindings::view
#import "shaders/sky_common.wgsl"::get_sky_color

// Lighting constants — warm sunset palette from Flo
const LIGHT_POSITION: vec3<f32> = vec3<f32>(20.0, 30.0, -20.0);
const LIGHT_COLOR: vec3<f32> = vec3<f32>(1.0, 0.9, 0.75);
const LIGHT_INTENSITY: f32 = 50000.0;

const PI: f32 = 3.141592;

// Seascape ocean parameters
const SEA_BASE: vec3<f32> = vec3<f32>(0.05, 0.12, 0.18);       // Dark indigo base for deep water
const SEA_WATER_COLOR: vec3<f32> = vec3<f32>(0.3, 0.6, 0.7);   // Realistic ocean water tint
const SEA_SPEED: f32 = 0.8;
const SEA_FREQ: f32 = 0.16;

// Octave rotation matrix for procedural wave layering
const OCTAVE_M: mat2x2<f32> = mat2x2<f32>(
    vec2<f32>(1.6, 1.2),
    vec2<f32>(-1.2, 1.6)
);

struct WaterMaterial {
    color: vec4<f32>,
    time: f32,
    camera_position: vec3<f32>,
    resolution: vec2<f32>,
    water_level: f32,
    grid_scale: f32,
};

@group(3) @binding(0) var<uniform> material: WaterMaterial;

// --- Lighting functions (energy-conserving, from Flo) ---

// Wrap-around diffuse: avoids fully dark areas on the back side
fn diffuse(n: vec3<f32>, l: vec3<f32>, p: f32) -> f32 {
    // dot(n,l) remapped from [-1,1] to [0.2, 1.0], then raised to power p
    return pow(dot(n, l) * 0.4 + 0.6, p);
}

// Energy-conserving specular (Blinn-Phong variant)
fn specular(n: vec3<f32>, l: vec3<f32>, e: vec3<f32>, s: f32) -> f32 {
    let nrm = (s + 8.0) / (PI * 8.0);
    return pow(max(dot(reflect(e, n), l), 0.0), s) * nrm;
}

// --- Procedural micro-wave noise for surface detail ---

fn hash_wave(p: vec2<f32>) -> f32 {
    let h = dot(p, vec2<f32>(127.1, 311.7));
    return fract(sin(h) * 43758.5453123);
}

fn noise_wave(p: vec2<f32>) -> f32 {
    let i = floor(p);
    let f = fract(p);
    let u = f * f * (3.0 - 2.0 * f);
    return -1.0 + 2.0 * mix(
        mix(hash_wave(i + vec2<f32>(0.0, 0.0)), hash_wave(i + vec2<f32>(1.0, 0.0)), u.x),
        mix(hash_wave(i + vec2<f32>(0.0, 1.0)), hash_wave(i + vec2<f32>(1.0, 1.0)), u.x),
        u.y
    );
}

// Get animated micro-wave normal perturbation
fn get_micro_wave_normal(world_pos: vec3<f32>, time: f32) -> vec3<f32> {
    let uv = world_pos.xz * SEA_FREQ;
    var uv_anim = uv;
    let t = time * SEA_SPEED;

    // Layer octaves of noise at different scales and speeds
    var wave = vec2<f32>(0.0);
    var freq = 1.0;
    var amp = 0.15;

    for (var i = 0; i < 3; i++) {
        let n1 = noise_wave(uv_anim * freq + vec2<f32>(t * 0.7, t * 0.3));
        let n2 = noise_wave(uv_anim * freq * 1.3 + vec2<f32>(-t * 0.5, t * 0.6));
        wave += vec2<f32>(n1, n2) * amp;
        uv_anim = OCTAVE_M * uv_anim;
        freq *= 1.9;
        amp *= 0.4;
    }

    // Convert wave displacement to a normal perturbation
    return normalize(vec3<f32>(wave.x, 1.0, wave.y));
}

// --- Core water color computation (ported from Flo's get_water_color) ---

fn get_water_color(
    p: vec3<f32>,           // World position of water surface
    n: vec3<f32>,           // Surface normal (already includes sim height normals)
    l: vec3<f32>,           // Light direction (normalized)
    eye: vec3<f32>,         // View direction (from surface TO camera, normalized)
    dist: vec3<f32>,        // Vector from surface to camera
    water_level: f32        // Water level for depth calculation
) -> vec3<f32> {
    // --- Fresnel reflection ---
    // At grazing angles (looking across the surface), water is highly reflective
    // Looking straight down, you see through to the refracted color
    var fresnel = clamp(1.0 - dot(n, eye), 0.0, 1.0);
    fresnel = pow(fresnel, 3.0) * 0.65;

    // --- Sky reflection ---
    let reflected = get_sky_color(reflect(-eye, n));

    // --- Refracted water body color ---
    // SEA_BASE provides the deep dark blue-green
    // A subtle diffuse term with high power (40.0) adds directional variation
    let refracted = SEA_BASE + diffuse(n, l, 40.0) * SEA_WATER_COLOR * 0.08;

    // --- Mix reflection and refraction based on Fresnel ---
    var color = mix(refracted, reflected, fresnel);

    // --- Distance attenuation ---
    let atten = max(1.0 - dot(dist, dist) * 0.0001, 0.0);

    // --- Depth-based darkening ---
    // Water gets significantly darker at troughs (negative height offsets)
    let depth_factor = smoothstep(-2.0, 0.0, p.y - water_level);
    color = mix(color * 0.3, color, depth_factor);

    // Subtle color tint at depth
    color = color + SEA_WATER_COLOR * (1.0 - depth_factor) * 0.05 * atten;

    // --- Specular sun highlights ---
    let spec = specular(n, l, eye, 80.0);
    color = color + LIGHT_COLOR * spec * 0.8;

    return color;
}

// --- Normal extraction from mesh vertex data ---

fn get_normal_from_mesh(world_normal: vec3<f32>) -> vec3<f32> {
    var normal = normalize(world_normal);
    // Ensure normal points upward (water surface convention)
    if normal.y < 0.0 {
        normal = -normal;
    }
    return normal;
}

// =====================
// FRAGMENT ENTRY POINT
// =====================

@fragment
fn fragment(in: VertexOutput) -> @location(0) vec4<f32> {
    let world_pos = in.world_position.xyz;

    // View direction: surface → camera
    let eye_dir = normalize(material.camera_position - world_pos);

    // Light direction (treat LIGHT_POSITION as directional for large scenes)
    let light_dir = normalize(LIGHT_POSITION);

    // Get the simulation-driven normal from the mesh
    var normal = get_normal_from_mesh(in.world_normal);

    // Layer procedural micro-wave detail on top of the simulation normal
    let micro_normal = get_micro_wave_normal(world_pos, material.time);
    // Blend: simulation normals dominate (0.85), micro-waves add subtle detail (0.15)
    normal = normalize(mix(normal, micro_normal, 0.15));

    // Distance vector from surface to camera
    let dist = material.camera_position - world_pos;

    // Compute ocean-style water color using Flo's Seascape-derived model
    let water_color = get_water_color(
        world_pos,
        normal,
        light_dir,
        eye_dir,
        dist,
        material.water_level
    );

    return vec4<f32>(water_color, material.color.a);
}
