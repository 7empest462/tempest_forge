// Procedural Sky and Cloud Shader
#import bevy_pbr::forward_io::VertexOutput
#import bevy_pbr::mesh_view_bindings::view

struct SkyMaterial {
    color: vec4<f32>,
    time: f32,
    cloudiness: f32,
    is_alien: f32,
    sun_dir: vec4<f32>,
};

@group(3) @binding(0) var<uniform> material: SkyMaterial;

fn hash(p: vec2<f32>) -> f32 {
    return fract(sin(dot(p, vec2<f32>(127.1, 311.7))) * 43758.5453123);
}

fn noise(p: vec2<f32>) -> f32 {
    let i = floor(p);
    let f = fract(p);
    let u = f * f * (3.0 - 2.0 * f);
    return mix(
        mix(hash(i), hash(i + vec2<f32>(1.0, 0.0)), u.x),
        mix(hash(i + vec2<f32>(0.0, 1.0)), hash(i + vec2<f32>(1.0, 1.0)), u.x),
        u.y
    ) * 2.0 - 1.0;
}

fn fbm(p: vec2<f32>) -> f32 {
    var v = 0.0;
    var a = 0.5;
    let shift = vec2<f32>(100.0);
    let rot = mat2x2<f32>(vec2<f32>(1.6, 1.2), vec2<f32>(-1.2, 1.6));
    var p_mut = p;
    for (var i = 0; i < 4; i = i + 1) {
        v = v + a * noise(p_mut);
        p_mut = rot * p_mut + shift;
        a = a * 0.5;
    }
    return v;
}

@fragment
fn fragment(
    in: VertexOutput,
) -> @location(0) vec4<f32> {
    let direction = normalize(in.world_position.xyz - view.world_position);
    
    if material.is_alien > 0.5 {
        let gradient_pos = clamp((direction.y + 0.4) / 1.6, 0.0, 1.0);
        
        // Alien sky base palette
        let deep_space = vec3<f32>(0.02, 0.01, 0.08);
        let horizon_glow = vec3<f32>(0.6, 0.25, 0.45);
        let mid_sky = vec3<f32>(0.15, 0.35, 0.75);
        let zenith = vec3<f32>(0.08, 0.12, 0.45);

        var color = mix(deep_space, horizon_glow, smoothstep(0.0, 0.4, gradient_pos));
        color = mix(color, mid_sky, smoothstep(0.3, 0.7, gradient_pos));
        color = mix(color, zenith, smoothstep(0.6, 1.0, gradient_pos));

        // === Primary Large Sun ===
        let sun_dir1 = normalize(vec3<f32>(0.6, 0.8, -0.4));
        let sun_intensity1 = pow(max(0.0, dot(direction, sun_dir1)), 32.0) * 1.8;
        color += vec3<f32>(1.0, 0.75, 0.4) * sun_intensity1 * 2.2;

        // === Secondary Smaller Sun ===
        let sun_dir2 = normalize(vec3<f32>(-0.7, 0.65, 0.3));
        let sun_intensity2 = pow(max(0.0, dot(direction, sun_dir2)), 48.0) * 0.9;
        color += vec3<f32>(0.5, 0.8, 1.2) * sun_intensity2 * 1.4;

        // === Black Hole ===
        let black_hole_dir = normalize(vec3<f32>(-0.3, 0.4, 0.85));
        let bh_cos = dot(direction, black_hole_dir);
        let bh_dist = 1.0 - bh_cos;
        
        // Accretion disk radius
        let disk_radius = 0.015;
        let event_horizon = 0.003;
        
        if bh_dist < disk_radius {
            // Calculate local 2D coordinates on the plane perpendicular to black_hole_dir
            let up = vec3<f32>(0.0, 1.0, 0.0);
            let right = normalize(cross(black_hole_dir, up));
            let local_up = cross(right, black_hole_dir);
            
            let local_pos = vec2<f32>(dot(direction, right), dot(direction, local_up));
            let angle = atan2(local_pos.y, local_pos.x);
            let dist_to_center = length(local_pos);
            
            // Swirling effect: add angle to noise coordinates
            let swirl = noise(vec2<f32>(dist_to_center * 150.0 - material.time * 2.0, angle * 3.0 - material.time * 4.0));
            
            if bh_dist < event_horizon {
                // Event Horizon (completely black core)
                color = vec3<f32>(0.0, 0.0, 0.0);
            } else {
                // Accretion Disk
                let t_disk = (bh_dist - event_horizon) / (disk_radius - event_horizon);
                // Intensity falls off, but has noise detail
                let disk_glow = pow(1.0 - t_disk, 3.0) * (1.0 + 0.6 * swirl);
                
                // Gravitational lensing / Einstein ring effect at the edge of event horizon
                let lensing = pow(1.0 - smoothstep(0.0, 0.002, bh_dist - event_horizon), 4.0) * 2.5;
                
                let disk_color = vec3<f32>(1.0, 0.45, 0.15) * disk_glow * 5.0 + vec3<f32>(1.0, 0.2, 0.6) * lensing;
                
                // Mix disk color with background
                color = mix(color, disk_color, disk_glow);
                // Gravitational lensing warp (darken slightly near the horizon)
                color = mix(color, vec3<f32>(0.0, 0.0, 0.0), (1.0 - t_disk) * 0.4);
            }
        }
        // Nebula / atmospheric glow
        let nebula = noise(direction.xz * 2.5 + vec2<f32>(material.time * 0.03, 0.0)) * 0.15;
        color += vec3<f32>(0.4, 0.2, 0.8) * nebula * (direction.y * 0.6 + 0.4);

        return vec4<f32>(color, 1.0);
    }
    
    // Base sky color from uniform
    var sky_color = material.color.rgb;

    // === Procedural Daytime Sun Disc & Solar Glare in Regular Sky ===
    let sun_d = normalize(material.sun_dir.xyz);
    let sun_dot = max(0.0, dot(direction, sun_d));
    let sun_disc = pow(sun_dot, 600.0) * 5.0;
    let sun_glare = pow(sun_dot, 14.0) * 0.9;
    let sun_light_color = vec3<f32>(1.0, 0.92, 0.65) * (sun_disc + sun_glare);
    sky_color += sun_light_color * (1.0 - material.cloudiness * 0.7);
    
    // Project direction to 2D XZ for planar scrolling clouds
    let uv = direction.xz / (abs(direction.y) + 0.12);
    
    // Wind scrolling direction
    let t = material.time * 0.024;
    let wind_dir = vec2<f32>(t, t * 0.72);
    
    // Generate organic cloud texture using fBm
    let cloud_noise = fbm(uv * 0.95 + wind_dir) * 0.5 + 0.5;
    
    // Threshold shifts down as cloudiness increases (clouds expand)
    let threshold = 0.76 - material.cloudiness * 0.52;
    
    // Smooth mask for cloud borders
    let cloud_mask = smoothstep(threshold - 0.16, threshold + 0.16, cloud_noise);
    
    // Fade out clouds near and below the horizon to prevent warping
    let horizon_fade = smoothstep(-0.02, 0.16, direction.y);
    
    // Fluffy cloud colors: white/blueish, but darkens as storm approaches
    let cloud_base_color = vec3<f32>(0.92, 0.94, 0.97) * (1.0 - material.cloudiness * 0.44);
    
    // Final composite
    let final_color = mix(sky_color, cloud_base_color, cloud_mask * horizon_fade * 0.86);
    
    return vec4<f32>(final_color, 1.0);
}
