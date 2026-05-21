/// Enhanced noise generation using bracket-noise
/// Provides multiple noise types for varied terrain generation

use bevy::prelude::*;
use bracket_noise::prelude::*;
use std::sync::Arc;
use bevy_voxel_world::prelude::*;
use crate::voxel::chunk::BlockType;

pub struct TerrainData {
    pub height: f32,
    pub is_desert: bool,
    pub _is_forest: bool,
}

#[derive(Resource, Clone)]
pub struct NoiseGenerator {
    pub inner: Arc<NoiseGeneratorInner>,
}

pub struct NoiseGeneratorInner {
    pub base_noise: FastNoise,
    pub detail_noise: FastNoise,
    pub cave_noise: FastNoise,
    pub moisture_noise: FastNoise,
    pub temp_noise: FastNoise,
    pub ore_noise: FastNoise,
    pub flora_noise: FastNoise,
}

impl NoiseGenerator {
    pub fn new() -> Self {
        let mut base_noise = FastNoise::seeded(1337);
        base_noise.set_noise_type(NoiseType::Perlin);
        base_noise.set_frequency(0.01);

        let mut detail_noise = FastNoise::seeded(1337 + 1);
        detail_noise.set_noise_type(NoiseType::Perlin);
        detail_noise.set_frequency(0.05);

        let mut cave_noise = FastNoise::seeded(1337 + 2);
        cave_noise.set_noise_type(NoiseType::Perlin);
        cave_noise.set_frequency(0.02);

        let mut moisture_noise = FastNoise::seeded(1337 + 3);
        moisture_noise.set_noise_type(NoiseType::Perlin);
        moisture_noise.set_frequency(0.002);

        let mut temp_noise = FastNoise::seeded(1337 + 4);
        temp_noise.set_noise_type(NoiseType::Perlin);
        temp_noise.set_frequency(0.01);

        let mut ore_noise = FastNoise::seeded(1337 + 5);
        ore_noise.set_noise_type(NoiseType::Perlin);
        ore_noise.set_frequency(0.1);

        let mut flora_noise = FastNoise::seeded(1337 + 6);
        flora_noise.set_noise_type(NoiseType::Perlin);
        flora_noise.set_frequency(0.15);

        Self {
            inner: Arc::new(NoiseGeneratorInner {
                base_noise,
                detail_noise,
                cave_noise,
                moisture_noise,
                temp_noise,
                ore_noise,
                flora_noise,
            }),
        }
    }

    pub fn get_terrain(&self, x: f32, z: f32) -> TerrainData {
        let inner = &self.inner;
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
            _is_forest: is_forest,
        }
    }

    pub fn get_adjusted_surface_height(&self, x: f32, z: f32) -> f32 {
        let terrain = self.get_terrain(x, z);
        let base = self.inner.base_noise.get_noise(x, z);

        // River Carving (Ridged noise)
        let river_val = self.inner.temp_noise.get_noise(x, z).abs();
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
        self.inner.cave_noise.get_noise3d(x, y, z)
    }

    pub fn get_ore_vein(&self, x: f32, y: f32, z: f32) -> f32 {
        self.inner.ore_noise.get_noise3d(x, y, z)
    }

    pub fn get_flora(&self, x: f32, z: f32) -> f32 {
        self.inner.flora_noise.get_noise(x, z)
    }
}

impl Default for NoiseGenerator {
    fn default() -> Self {
        Self::new()
    }
}


impl VoxelWorldConfig for NoiseGenerator {
    type MaterialIndex = u8;
    type ChunkUserBundle = ();

    fn spawning_distance(&self) -> u32 {
        14
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
        Some(("default_texture.png".into(), 13))
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
                BlockType::Brick => [3, 3, 3], // Brown/Clay
                BlockType::Concrete => [6, 6, 6], // Dark Gray
                BlockType::WoodPlanks => [8, 8, 8], // Wood
                BlockType::Glass => [4, 4, 4], // Stone-like frame for now
                _ => [4, 4, 4],
            }
        })
    }

    fn voxel_lookup_delegate(&self) -> VoxelLookupDelegate<Self::MaterialIndex> {
        let generator = self.clone();
        Box::new(move |_chunk_pos, _lod, _prev_chunk| {
            let gen_ref = generator.clone();
            Box::new(move |pos, _prev_voxel| {
                let x = pos.x as f32;
                let y = pos.y as f32;
                let z = pos.z as f32;

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
                
                // Cave System (Deeper and more structured)
                let cave_val = gen_ref.get_cave(x * 0.04, y * 0.08, z * 0.04).abs();
                if cave_val < 0.06 && depth > 25.0 {
                    return WorldVoxel::Air;
                }

                // Ore Veins
                let ore_val = gen_ref.get_ore_vein(x * 0.1, y * 0.1, z * 0.1);
                if ore_val > 0.8 && depth > 10.0 {
                    if depth < 40.0 {
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
