#import bevy_pbr::forward_io::VertexOutput
#import bevy_pbr::mesh_view_bindings::view

struct TornadoMaterial {
    color: vec4<f32>,
    time: f32,
};

@group(3) @binding(0) var<uniform> material: TornadoMaterial;

// Simple 2D pseudo-random noise
fn hash(p: vec2<f32>) -> f32 {
    return fract(sin(dot(p, vec2<f32>(127.1, 311.7))) * 43758.5453123);
}

fn noise(p: vec2<f32>) -> f32 {
    let i = floor(p);
    let f = fract(p);
    let u = f * f * (3.0 - 2.0 * f);
    
    return mix(
        mix(hash(i + vec2<f32>(0.0, 0.0)), hash(i + vec2<f32>(1.0, 0.0)), u.x),
        mix(hash(i + vec2<f32>(0.0, 1.0)), hash(i + vec2<f32>(1.0, 1.0)), u.x),
        u.y
    );
}

fn fbm(p: vec2<f32>) -> f32 {
    var v = 0.0;
    var a = 0.5;
    var shift = vec2<f32>(100.0);
    var p_temp = p;
    for (var i = 0; i < 4; i = i + 1) {
        v = v + a * noise(p_temp);
        p_temp = p_temp * 2.0 + shift;
        a = a * 0.5;
    }
    return v;
}

@fragment
fn fragment(in: VertexOutput) -> @location(0) vec4<f32> {
    // Scroll the UVs over time to simulate upward and rotational wind flow
    let uv_scroll = vec2<f32>(
        in.uv.x * 3.0 + material.time * 2.5,
        in.uv.y * 1.5 - material.time * 1.8
    );
    
    // Compute two layers of FBM noise scrolling at different speeds
    let n1 = fbm(uv_scroll);
    let n2 = fbm(uv_scroll * 1.7 - material.time * vec2<f32>(-1.1, 0.9));
    let combined_noise = mix(n1, n2, 0.45);
    
    // Soften the edges of the cylinder using Fresnel (view-dependent transparency)
    // This makes the cylinder look fluffy and cloud-like instead of a hard-edged tube
    let view_dir = normalize(view.world_position.xyz - in.world_position.xyz);
    let normal = normalize(in.world_normal);
    let edge_fade = pow(1.0 - abs(dot(normal, view_dir)), 1.8);
    
    // Apply noise to the alpha transparency
    let alpha = material.color.a * edge_fade * smoothstep(0.15, 0.85, combined_noise);
    
    // Dark storm cloud color with highlights based on height
    let height_gradient = smoothstep(-10.0, 60.0, in.world_position.y);
    let final_color = mix(material.color.rgb, material.color.rgb * 0.4, height_gradient);
    
    return vec4<f32>(final_color, alpha);
}
