// Shared sky gradient function used by water reflections
// Daytime sky gradient matching Tempest Forge's peak daylight

fn get_sky_color(direction: vec3<f32>) -> vec3<f32> {
    // Map Y direction to gradient position
    let gradient_pos = clamp((direction.y + 0.5) / 1.5, 0.0, 1.0);
    
    // Daytime sky palette — blue tones appropriate for peak daylight
    let bottom_color = vec3<f32>(0.55, 0.72, 0.85);     // Pale hazy blue (near horizon)
    let horizon_color = vec3<f32>(0.35, 0.65, 0.88);     // Light sky blue (horizon)
    let mid_sky_color = vec3<f32>(0.15, 0.40, 0.80);     // Rich blue (mid sky)
    let top_color = vec3<f32>(0.05, 0.15, 0.45);         // Deep blue (zenith)
    
    var color: vec3<f32>;
    
    if gradient_pos < 0.3 {
        // Below horizon — pale haze to horizon blue
        let t = gradient_pos / 0.3;
        color = mix(bottom_color, horizon_color, smoothstep(0.0, 1.0, t));
    } else if gradient_pos < 0.7 {
        // Horizon to mid sky — light blue deepening
        let t = (gradient_pos - 0.3) / 0.4;
        color = mix(horizon_color, mid_sky_color, smoothstep(0.0, 1.0, t));
    } else {
        // Mid sky to zenith — deep blue
        let t = (gradient_pos - 0.7) / 0.3;
        color = mix(mid_sky_color, top_color, smoothstep(0.0, 1.0, t));
    }
    
    return color;
}
