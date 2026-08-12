use bevy::{
    asset::RenderAssetUsages, mesh::Indices, prelude::*, render::render_resource::PrimitiveTopology,
};
use rustc_hash::FxHashMap;

/// A specialized mesh generator for voxel-based characters
/// that supports smooth shading by averaging normals at vertex boundaries.
pub struct VoxelMeshBuilder {
    positions: Vec<[f32; 3]>,
    normals: Vec<[f32; 3]>,
    colors: Vec<[f32; 4]>,
    indices: Vec<u32>,
    /// Maps (x,y,z) position to vertex index to allow vertex sharing
    /// and normal accumulation for smoothing.
    vertex_map: FxHashMap<(i32, i32, i32, usize), u32>,
}

impl Default for VoxelMeshBuilder {
    fn default() -> Self {
        Self::new()
    }
}

impl VoxelMeshBuilder {
    pub fn new() -> Self {
        Self {
            positions: Vec::new(),
            normals: Vec::new(),
            colors: Vec::new(),
            indices: Vec::new(),
            vertex_map: FxHashMap::default(),
        }
    }

    /// Adds a face to the mesh.
    pub fn add_face(
        &mut self,
        x: i32,
        y: i32,
        z: i32,
        face_idx: usize,
        voxel_size: f32,
        color: [f32; 4],
        smooth: bool,
    ) {
        let face_normals = [
            [1.0, 0.0, 0.0],  // 0: Right (X+)
            [-1.0, 0.0, 0.0], // 1: Left (X-)
            [0.0, 1.0, 0.0],  // 2: Top (Y+)
            [0.0, -1.0, 0.0], // 3: Bottom (Y-)
            [0.0, 0.0, 1.0],  // 4: Front (Z+)
            [0.0, 0.0, -1.0], // 5: Back (Z-)
        ];

        let face_verts = [
            [
                [1.0, 0.0, 1.0],
                [1.0, 0.0, 0.0],
                [1.0, 1.0, 0.0],
                [1.0, 1.0, 1.0],
            ],
            [
                [0.0, 0.0, 0.0],
                [0.0, 0.0, 1.0],
                [0.0, 1.0, 1.0],
                [0.0, 1.0, 0.0],
            ],
            [
                [0.0, 1.0, 1.0],
                [1.0, 1.0, 1.0],
                [1.0, 1.0, 0.0],
                [0.0, 1.0, 0.0],
            ],
            [
                [0.0, 0.0, 0.0],
                [1.0, 0.0, 0.0],
                [1.0, 0.0, 1.0],
                [0.0, 0.0, 1.0],
            ],
            [
                [0.0, 0.0, 1.0],
                [1.0, 0.0, 1.0],
                [1.0, 1.0, 1.0],
                [0.0, 1.0, 1.0],
            ],
            [
                [1.0, 0.0, 0.0],
                [0.0, 0.0, 0.0],
                [0.0, 1.0, 0.0],
                [1.0, 1.0, 0.0],
            ],
        ];

        let mut face_indices = Vec::new();
        let normal = face_normals[face_idx];

        for &local_v in &face_verts[face_idx] {
            let vx = (x as f32 + local_v[0]) * voxel_size;
            let vy = (y as f32 + local_v[1]) * voxel_size;
            let vz = (z as f32 + local_v[2]) * voxel_size;

            let color_key = ((color[0] * 255.0) as i32)
                | (((color[1] * 255.0) as i32) << 8)
                | (((color[2] * 255.0) as i32) << 16);

            let key = if smooth {
                (
                    (vx * 1000.0) as i32,
                    (vy * 1000.0) as i32,
                    (vz * 1000.0) as i32,
                    color_key as usize,
                )
            } else {
                (self.positions.len() as i32, 0, 0, 0)
            };

            let idx = if let Some(&existing_idx) = self.vertex_map.get(&key) {
                self.normals[existing_idx as usize][0] += normal[0];
                self.normals[existing_idx as usize][1] += normal[1];
                self.normals[existing_idx as usize][2] += normal[2];
                existing_idx
            } else {
                let new_idx = self.positions.len() as u32;
                self.positions.push([vx, vy, vz]);
                self.normals.push(normal);
                self.colors.push(color);
                if smooth {
                    self.vertex_map.insert(key, new_idx);
                }
                new_idx
            };
            face_indices.push(idx);
        }

        self.indices.push(face_indices[0]);
        self.indices.push(face_indices[1]);
        self.indices.push(face_indices[2]);
        self.indices.push(face_indices[2]);
        self.indices.push(face_indices[3]);
        self.indices.push(face_indices[0]);
    }

    pub fn build(mut self) -> Mesh {
        // Normalize accumulated normals
        for n in &mut self.normals {
            let len = (n[0] * n[0] + n[1] * n[1] + n[2] * n[2]).sqrt();
            if len > 0.0 {
                n[0] /= len;
                n[1] /= len;
                n[2] /= len;
            }
        }

        let mut mesh = Mesh::new(
            PrimitiveTopology::TriangleList,
            RenderAssetUsages::default(),
        );
        mesh.insert_attribute(Mesh::ATTRIBUTE_POSITION, self.positions);
        mesh.insert_attribute(Mesh::ATTRIBUTE_NORMAL, self.normals);
        mesh.insert_attribute(Mesh::ATTRIBUTE_COLOR, self.colors);
        mesh.insert_indices(Indices::U32(self.indices));
        mesh
    }
}
