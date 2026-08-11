use bracket_noise::prelude::FastNoise;
fn main() {
    let a = FastNoise::seeded(0);
    let b = a.clone();
}
