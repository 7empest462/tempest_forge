/// TreeMap — shared, thread-safe store of all pre-generated tree voxels.
///
/// This is the core architectural fix: bevy_voxel_world's voxel_lookup_delegate
/// is the ONLY write path that persists. We pre-compute trees once with shrubbery
/// (space colonization + voxelization) and store them here. The lookup delegate
/// reads from this map, returning Log/Leaves where a tree voxel exists.
///
/// The map is populated from the ECS (chunk_vegetation_system) and read from
/// the noise generator (which runs on worker threads — hence Arc<RwLock>).

use crate::voxel::chunk::BlockType;
use bevy::prelude::IVec3;
use hashbrown::HashMap;
use parking_lot::RwLock;
use rustc_hash::FxBuildHasher;
use triomphe::Arc;

/// Shared between NoiseGenerator (read) and chunk_vegetation_system (write).
#[derive(Clone)]
pub struct TreeMap(pub Arc<RwLock<HashMap<IVec3, BlockType, FxBuildHasher>>>);

impl Default for TreeMap {
    fn default() -> Self {
        Self(Arc::new(RwLock::new(HashMap::with_capacity_and_hasher(
            4096,
            FxBuildHasher::default(),
        ))))
    }
}

impl TreeMap {
    /// Insert a batch of tree voxels atomically.
    pub fn insert_batch(&self, voxels: impl IntoIterator<Item = (IVec3, BlockType)>) {
        let mut map = self.0.write();
        for (pos, block) in voxels {
            map.entry(pos).or_insert(block); // don't overwrite existing wood with leaves
        }
    }

    /// Query a single position — called millions of times per second from lookup delegate.
    #[inline]
    pub fn get(&self, pos: IVec3) -> Option<BlockType> {
        self.0.read().get(&pos).copied()
    }

    /// Check whether a 2D column (x, z) has any tree registered near it.
    /// Used to avoid redundant shrubbery generation on already-decorated columns.
    pub fn column_has_tree(&self, x: i32, z: i32) -> bool {
        self.0.read().keys().any(|p| p.x == x && p.z == z)
    }
}
