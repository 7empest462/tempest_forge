//! Enhanced noise generation using bracket-noise
//! Provides multiple noise types for varied terrain generation

use crate::voxel::chunk::BlockType;
use bevy::prelude::*;
use bevy_voxel_world::prelude::*;
use bracket_noise::prelude::*;
use std::sync::Arc;

pub struct TerrainData {
    pub height: f32,
    pub is_desert: bool,
    pub is_forest: bool,
}

#[derive(Resource, Clone)]
pub struct NoiseGenerator {
    pub inner: Arc<parking_lot::RwLock<Arc<NoiseGeneratorInner>>>,
    pub spawning_distance: u32,
}

pub struct NoiseGeneratorInner {
    pub seed: u32,
    pub base_noise: FastNoise,
    pub detail_noise: FastNoise,
    pub cave_noise: FastNoise,
    pub moisture_noise: FastNoise,
    pub temp_noise: FastNoise,
    pub ore_noise: FastNoise,
    pub flora_noise: FastNoise,
}

// Fast FBM implementation using pre-seeded noise objects to avoid allocation on the fly
fn fbm_noise_fast(x: f32, z: f32, noise1: &FastNoise, noise2: &FastNoise) -> f32 {
    let mut value = 0.0;
    value += 0.5 * noise1.get_noise(x, z);
    value += 0.25 * noise2.get_noise(x * 2.0, z * 2.0);
    value += 0.125 * noise1.get_noise(x * 4.0, z * 4.0);
    value += 0.0625 * noise2.get_noise(x * 8.0, z * 8.0);
    value
}
fn get_alien_height(x: f32, z: f32, inner: &NoiseGeneratorInner) -> f32 {
    let nx = x * 0.025;
    let nz = z * 0.025;

    // Base terrain with sharp alien peaks
    let base = fbm_noise_fast(nx, nz, &inner.base_noise, &inner.detail_noise) * 32.0;
    let peaks = fbm_noise_fast(nx * 3.0, nz * 3.0, &inner.cave_noise, &inner.moisture_noise) * 20.0;

    // Floating island bias
    let islands =
        (fbm_noise_fast(nx * 0.5, nz * 0.5, &inner.temp_noise, &inner.ore_noise) - 0.3) * 45.0;

    base + peaks + islands.max(0.0) + 12.0 // Lowered to 12.0 to allow basins to go below sea level (15.0)
}

impl NoiseGenerator {
    pub fn new(seed: u32, spawning_distance: u32) -> Self {
        let mut base_noise = FastNoise::seeded(seed as u64);
        base_noise.set_noise_type(NoiseType::Perlin);
        base_noise.set_frequency(0.01);

        let mut detail_noise = FastNoise::seeded((seed + 1) as u64);
        detail_noise.set_noise_type(NoiseType::Perlin);
        detail_noise.set_frequency(0.05);

        let mut cave_noise = FastNoise::seeded((seed + 2) as u64);
        cave_noise.set_noise_type(NoiseType::Perlin);
        cave_noise.set_frequency(0.025);

        let mut moisture_noise = FastNoise::seeded((seed + 3) as u64);
        moisture_noise.set_noise_type(NoiseType::Perlin);
        moisture_noise.set_frequency(0.002);

        let mut temp_noise = FastNoise::seeded((seed + 4) as u64);
        temp_noise.set_noise_type(NoiseType::Perlin);
        temp_noise.set_frequency(0.01);

        let mut ore_noise = FastNoise::seeded((seed + 5) as u64);
        ore_noise.set_noise_type(NoiseType::Perlin);
        ore_noise.set_frequency(0.1);

        let mut flora_noise = FastNoise::seeded((seed + 6) as u64);
        flora_noise.set_noise_type(NoiseType::Perlin);
        flora_noise.set_frequency(0.15);

        Self {
            inner: Arc::new(parking_lot::RwLock::new(Arc::new(NoiseGeneratorInner {
                seed,
                base_noise,
                detail_noise,
                cave_noise,
                moisture_noise,
                temp_noise,
                ore_noise,
                flora_noise,
            }))),
            spawning_distance,
        }
    }

    pub fn get_terrain(&self, x: f32, z: f32) -> TerrainData {
        let inner_arc = self.inner.read().clone();
        let inner = &*inner_arc;

        // Branch for Alien Dimension
        if x >= 5000.0 {
            let height = get_alien_height(x, z, inner);
            return TerrainData {
                height,
                is_desert: false,
                is_forest: false,
            };
        }

        let base = inner.base_noise.get_noise(x, z);
        let moisture_val = inner.moisture_noise.get_noise(x, z);
        let detail = inner.detail_noise.get_noise(x, z);

        // Map base (-1.0 to 1.0) to a more dramatic height range
        // We want a lot of area to be below sea level (15.0)
        let mut height = if base.abs() < 0.30 {
            // Plateau / Flat Land (Plains)
            let t = (base + 0.30) / 0.60; // 0 to 1
            24.0 + (t - 0.5) * 3.0 // Extremely gentle slope (±1.5m)
        } else if base < 0.0 {
            // Oceans and lowlands
            let t = (base + 0.30) / -0.70; // 0 to 1 as base goes from -0.30 to -1.0
            22.5 - t * 40.0
        } else {
            // Plains and mountains
            let t = (base - 0.30) / 0.70; // 0 to 1 as base goes from 0.30 to 1.0
            25.5 + t * 80.0
        };

        // Add mountain peaks (Smooth ramp)
        if base > 0.45 {
            let mountain_t = (base - 0.45) / 0.55; // 0 to 1
            height += mountain_t.powi(2) * 100.0; // Quadratic ramp for smoother peaks
        }

        // Add detail noise
        height += detail * 4.0;

        let is_desert = moisture_val < -0.3;
        let is_forest = moisture_val > 0.3;

        TerrainData {
            height,
            is_desert,
            is_forest,
        }
    }

    pub fn get_adjusted_surface_height(&self, x: f32, z: f32) -> f32 {
        let inner_arc = self.inner.read().clone();
        let inner = &*inner_arc;
        if x >= 5000.0 {
            return get_alien_height(x, z, inner);
        }

        let terrain = self.get_terrain(x, z);
        let base = inner.base_noise.get_noise(x, z);

        // River Carving (Ridged noise)
        let river_val = inner.temp_noise.get_noise(x, z).abs();
        let is_river = river_val < 0.05;

        let river_depth = if is_river {
            let target_river_height = 15.0 + (base + 0.3).max(0.0) * 15.0;
            let max_depth = (terrain.height - target_river_height).max(0.0);
            let t = river_val / 0.05;
            let carve_factor = 1.0 - t;
            max_depth * carve_factor
        } else {
            0.0
        };

        terrain.height - river_depth
    }

    pub fn get_cave(&self, x: f32, y: f32, z: f32) -> f32 {
        self.inner.read().cave_noise.get_noise3d(x, y, z)
    }

    pub fn get_ore_vein(&self, x: f32, y: f32, z: f32) -> f32 {
        self.inner.read().ore_noise.get_noise3d(x, y, z)
    }

    pub fn get_flora(&self, x: f32, z: f32) -> f32 {
        self.inner.read().flora_noise.get_noise(x, z)
    }
}

impl Default for NoiseGenerator {
    fn default() -> Self {
        Self::new(1337, 0)
    }
}

impl VoxelWorldConfig for NoiseGenerator {
    type MaterialIndex = u8;
    type ChunkUserBundle = ();

    fn spawning_distance(&self) -> u32 {
        self.spawning_distance
    }

    fn chunk_lod(&self, chunk_pos: IVec3, _prev_lod: Option<u8>, player_pos: Vec3) -> u8 {
        // Chunk size is 32 voxels. Center of the chunk in world-space:
        let chunk_world_pos = Vec3::new(
            (chunk_pos.x * 32 + 16) as f32,
            (chunk_pos.y * 32 + 16) as f32,
            (chunk_pos.z * 32 + 16) as f32,
        );
        let distance = player_pos.distance(chunk_world_pos);

        // LOD 0: within 6 chunks (192m) — maximum detail for gameplay and collision region
        // LOD 1: between 6–10 chunks (192–320m) — moderate detail, halved mesh resolution
        // LOD 2: beyond 10 chunks (320m+) — coarse backdrop, quarter mesh resolution
        if distance < 192.0 {
            0
        } else if distance < 320.0 {
            1
        } else {
            2
        }
    }

    /// Reduce voxel data resolution for distant chunks to save CPU during generation.
    /// LOD 0: full 34³ padded (32 interior + 2 padding), LOD 1: 18³, LOD 2: 10³.
    fn chunk_data_shape(&self, lod_level: u8) -> UVec3 {
        match lod_level {
            0 => padded_chunk_shape_uniform(32), // 34³ — full resolution
            1 => padded_chunk_shape_uniform(16), // 18³ — half resolution
            _ => padded_chunk_shape_uniform(8),  // 10³ — quarter resolution
        }
    }

    /// Reduce mesh resolution for distant chunks to slash vertex/triangle count.
    /// Uses the same dimensions as data_shape so no extra downsampling pass is needed.
    fn chunk_meshing_shape(&self, lod_level: u8) -> UVec3 {
        match lod_level {
            0 => padded_chunk_shape_uniform(32),
            1 => padded_chunk_shape_uniform(16),
            _ => padded_chunk_shape_uniform(8),
        }
    }

    fn voxel_texture(&self) -> Option<(String, u32)> {
        Some(("default_texture.png".into(), 14))
    }

    fn texture_index_mapper(&self) -> Arc<dyn Fn(Self::MaterialIndex) -> [u32; 3] + Send + Sync> {
        Arc::new(|vox_mat: u8| {
            let block = BlockType::from(vox_mat);
            match block {
                BlockType::Water => [11, 11, 11],
                BlockType::Sand => [1, 1, 1],
                BlockType::Grass => [2, 2, 2],
                BlockType::Dirt => [3, 3, 3],
                BlockType::Limestone => [4, 4, 4],
                BlockType::Granite => [5, 5, 5],
                BlockType::Basalt => [6, 6, 6],
                BlockType::Slate => [7, 7, 7],
                BlockType::Wood | BlockType::OakLog | BlockType::PineLog => [8, 8, 8],
                BlockType::Leaves => [9, 9, 9],
                BlockType::Flower | BlockType::Fern | BlockType::Moss => [10, 10, 10],
                BlockType::IronOre | BlockType::IronBlock => [11, 11, 11],
                BlockType::GoldOre | BlockType::GoldBlock => [12, 12, 12],
                BlockType::Brick => [3, 3, 3],
                BlockType::Concrete => [6, 6, 6],
                BlockType::WoodPlanks => [8, 8, 8],
                BlockType::Glass => [13, 13, 13],

                // Alien Blocks (indices 14 to 18 in the 19-layer array)
                BlockType::AlienStone => [14, 14, 14],
                BlockType::AlienDirt => [15, 15, 15],
                BlockType::GlowingMoss => [16, 16, 16],
                BlockType::AlienCrystal => [17, 17, 17],
                BlockType::FloatingCrystal => [18, 18, 18],

                _ => [4, 4, 4],
            }
        })
    }

    fn voxel_lookup_delegate(&self) -> VoxelLookupDelegate<Self::MaterialIndex> {
        let generator = self.clone();
        Box::new(move |_chunk_pos, _lod, _prev_chunk| {
            let inner_arc = generator.inner.read().clone();
            let gen_ref = generator.clone();
            Box::new(move |pos, _prev_voxel| {
                let x = pos.x as f32;
                let y = pos.y as f32;
                let z = pos.z as f32;

                let inner = &*inner_arc;

                // 1. Alien Portal structure
                if x >= 5000.0 {
                    let dx = (x - 10000.0).abs();
                    let dz = (z - 10050.0).abs();
                    if dx < 2.5 && dz < 0.5 {
                        let portal_y = get_alien_height(10000.0, 10050.0, inner).round();
                        let dy = y - portal_y;
                        if (0.0..=5.0).contains(&dy) {
                            if dx > 0.5 || dy == 5.0 {
                                return WorldVoxel::Solid(BlockType::FloatingCrystal as u8);
                            } else {
                                return WorldVoxel::Air;
                            }
                        }
                    }
                } else {
                    // Normal Portal structure
                    let dx = x.abs();
                    let dz = (z - 50.0).abs();
                    if dx < 2.5 && dz < 0.5 {
                        let dy = y - 42.0;
                        if (0.0..=5.0).contains(&dy) {
                            if dx > 0.5 || dy == 5.0 {
                                return WorldVoxel::Solid(BlockType::FloatingCrystal as u8);
                            } else {
                                return WorldVoxel::Air;
                            }
                        }
                    }
                }

                // Branch for Alien Dimension
                if x >= 5000.0 {
                    let adjusted_surface = get_alien_height(x, z, inner);
                    if y >= 30.0 {
                        // Floating islands in the sky
                        let island_noise = fbm_noise_fast(
                            x * 0.035,
                            z * 0.035,
                            &inner.temp_noise,
                            &inner.ore_noise,
                        );
                        if island_noise > 0.32 && y <= 45.0 {
                            return WorldVoxel::Solid(BlockType::FloatingCrystal as u8);
                        }
                        return WorldVoxel::Air;
                    }

                    if y < adjusted_surface - 6.0 {
                        return WorldVoxel::Solid(BlockType::AlienStone as u8); // Deep rock
                    } else if y < adjusted_surface - 1.0 {
                        return WorldVoxel::Solid(BlockType::AlienDirt as u8); // Middle sediment
                    } else if y < adjusted_surface {
                        return WorldVoxel::Solid(BlockType::GlowingMoss as u8); // Top green moss / grass
                    } else {
                        return WorldVoxel::Air;
                    }
                }

                let terrain = gen_ref.get_terrain(x, z);
                let adjusted_surface = gen_ref.get_adjusted_surface_height(x, z);
                let sea_level = 15.0;

                // 3. Flora & Structures (Above ground/water)
                if y > adjusted_surface {
                    return WorldVoxel::Air;
                }

                // 1. Surface Layers
                if y > adjusted_surface - 1.0 {
                    if terrain.is_desert {
                        return WorldVoxel::Solid(BlockType::Sand as u8);
                    }
                    // Beach transition
                    if y < sea_level + 0.5 {
                        return WorldVoxel::Solid(BlockType::Sand as u8);
                    }
                    return WorldVoxel::Solid(BlockType::Grass as u8);
                }

                if y > adjusted_surface - 5.0 {
                    if terrain.is_desert {
                        return WorldVoxel::Solid(BlockType::Sand as u8);
                    }
                    return WorldVoxel::Solid(BlockType::Dirt as u8);
                }

                // 2. Geological Layers (Depth based)
                let depth = adjusted_surface - y;

                // Cave System (winding tunnels with surface entrances)
                let cave_val = gen_ref.get_cave(x, y, z).abs();
                // Gradually increase cave size with depth, starting with narrow entrances at the surface
                let t = (depth / 20.0).clamp(0.0, 1.0);
                let cave_threshold = 0.035 + (0.08 - 0.035) * t;
                if cave_val < cave_threshold && depth > 2.0 {
                    return WorldVoxel::Air;
                }

                // Ore Veins (Iron and Gold, exposed on cave walls and shallow underground)
                let ore_val = gen_ref.get_ore_vein(x * 0.1, y * 0.1, z * 0.1);
                if ore_val > 0.65 && depth > 8.0 {
                    if depth < 35.0 {
                        return WorldVoxel::Solid(BlockType::IronOre as u8);
                    } else {
                        return WorldVoxel::Solid(BlockType::GoldOre as u8);
                    }
                }

                if depth < 15.0 {
                    WorldVoxel::Solid(BlockType::Limestone as u8)
                } else if depth < 40.0 {
                    WorldVoxel::Solid(BlockType::Granite as u8)
                } else if depth < 80.0 {
                    WorldVoxel::Solid(BlockType::Basalt as u8)
                } else {
                    WorldVoxel::Solid(BlockType::Slate as u8)
                }
            })
        })
    }
}
