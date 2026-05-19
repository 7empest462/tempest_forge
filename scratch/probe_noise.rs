use bracket_noise::prelude::*;

fn main() {
    let mut base_noise = FastNoise::new();
    base_noise.set_seed(1337);
    base_noise.set_noise_type(NoiseType::Perlin);
    base_noise.set_frequency(0.01);
    base_noise.set_fractal_type(FractalType::FBM);
    base_noise.set_fractal_octaves(4);
    
    let mut detail_noise = FastNoise::new();
    detail_noise.set_seed(1337);
    detail_noise.set_noise_type(NoiseType::Perlin);
    detail_noise.set_frequency(0.05);

    println!("Noise Probe (Seed 1337):");
    for z in (0..512).step_by(32) {
        for x in (0..512).step_by(32) {
            let cont_val = base_noise.get_noise(x as f32, z as f32);
            let detail = detail_noise.get_noise(x as f32 * 0.5, z as f32 * 0.5);
            let height = (detail + 1.0) * 0.5 * 60.0 + 30.0; // Approximation of mountain weight
            if cont_val > 0.5 {
                print!("X "); // Likely High land
            } else if cont_val > -0.4 {
                print!(". "); // Plains/Hills
            } else {
                print!("_ "); // Ocean/Void
            }
        }
        println!(" | z={}", z);
    }
}
