// Probe script for Voxel Forge Noise
use bracket_noise::prelude::*;

fn main() {
    let mut base_noise = FastNoise::new();
    base_noise.set_seed(1337);
    base_noise.set_noise_type(NoiseType::Perlin);
    base_noise.set_frequency(0.006);

    let test_points = vec![
        (0.0, 0.0),
        (16.0, 0.0),
        (256.0, 0.0),
        (512.0, 0.0),
        (-16.0, 0.0),
        (-256.0, 0.0),
        (-512.0, 0.0),
    ];

    println!("--- NOISE PROBE (Seed 1337) ---");
    for (x, z) in test_points {
        let mut cont_val = base_noise.get_noise(x, z);
        let dist_sq = x*x + z*z;
        let radius = 640.0;
        let bias = (-(dist_sq) / (radius * radius)).exp(); 
        let biased_val = cont_val * (1.0 - bias) + 0.4 * bias;
        
        println!("At ({: >5}, {: >5}): DistSq: {: >8.0} | Bias: {:.4} | Orig: {:.4} | Final: {:.4}", 
                 x, z, dist_sq, bias, cont_val, biased_val);
    }
}
