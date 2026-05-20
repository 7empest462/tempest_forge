use bevy::prelude::*;
use bevy_voxel_world::prelude::*;
use crate::world::noise_generator::NoiseGenerator;

/// Finds the highest ground (Solid block)
pub fn find_ground_height(pos: Vec3, world: &VoxelWorld<NoiseGenerator>) -> Option<f32> {
    let gx = pos.x.floor() as i32;
    let gy = pos.y.floor() as i32;
    let gz = pos.z.floor() as i32;

    // Search range downwards and upwards (wide enough for deep valleys and high mountains)
    for y in (gy - 256..gy + 64).rev() {
        let p = IVec3::new(gx, y, gz);
        if let WorldVoxel::Solid(mat) = world.get_voxel(p) {
            // Ignore water (BlockType::Water == 1)
            if mat != 1 {
                return Some(y as f32 + 1.0);
            }
        }
    }
    None
}

/// Finds the highest "stable" ground (Grass, Dirt, Stone, etc.)
pub fn find_stable_ground_height(pos: Vec3, world: &VoxelWorld<NoiseGenerator>) -> Option<f32> {
    let gx = pos.x.floor() as i32;
    let gy = pos.y.floor() as i32;
    let gz = pos.z.floor() as i32;

    // Search range downwards and upwards (wide enough for deep valleys and high mountains)
    for y in (gy - 256..gy + 64).rev() {
        let p = IVec3::new(gx, y, gz);
        if let WorldVoxel::Solid(mat) = world.get_voxel(p) {
            if is_stable_block(mat) {
                return Some(y as f32 + 1.0);
            }
        }
    }
    None
}

fn is_stable_block(mat: u8) -> bool {
    let block = crate::voxel::chunk::BlockType::from(mat);
    matches!(block, 
        crate::voxel::chunk::BlockType::Grass | 
        crate::voxel::chunk::BlockType::Dirt | 
        crate::voxel::chunk::BlockType::Sand |
        crate::voxel::chunk::BlockType::Stone |
        crate::voxel::chunk::BlockType::Podzol |
        crate::voxel::chunk::BlockType::Limestone |
        crate::voxel::chunk::BlockType::Granite |
        crate::voxel::chunk::BlockType::Cobblestone |
        crate::voxel::chunk::BlockType::Basalt
    )
}
