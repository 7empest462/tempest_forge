use bevy::prelude::*;
use crate::player::mesh_gen::VoxelMeshBuilder;

// ========================================================
// Voxel size: Increased resolution for "Smooth" look
// 0.0125 units per voxel -> 80 voxels = 1.0 unit
// 1.8 unit total height = 144 voxels tall
// ========================================================
pub const V: f32 = 0.0125;

// ========================================================
// Palette Indices — "Tempest Forge" Specialist
// ========================================================
pub const SKIN: i8       = 0;
pub const SKIN_SH: i8    = 1;
pub const HAIR: i8       = 2;
pub const EYE_W: i8      = 3;
pub const PUPIL: i8      = 4;
pub const IRIS: i8       = 5;
pub const GOG_FR: i8     = 6;  // goggle frame (brass/copper)
pub const GOG_LN: i8     = 7;  // goggle lens (cyan glow)
pub const CLOTH_1: i8    = 8;  // main fabric (charcoal)
pub const CLOTH_2: i8    = 9;  // accent fabric (tempest blue)
pub const LEATHER: i8    = 10; // belt/straps/boots (brown)
pub const METAL: i8      = 11; // buckles/hardware (steel)
pub const MOUTH: i8      = 12;

/// Returns (base_color, metallic, emissive) for each palette index.
pub fn base_palette() -> Vec<(Color, f32, LinearRgba)> {
    vec![
        (Color::srgb(0.85, 0.65, 0.50), 0.0, LinearRgba::BLACK),  // 0  Skin
        (Color::srgb(0.70, 0.50, 0.38), 0.0, LinearRgba::BLACK),  // 1  Skin Shadow
        (Color::srgb(0.25, 0.15, 0.08), 0.0, LinearRgba::BLACK),  // 2  Hair
        (Color::srgb(0.95, 0.95, 0.95), 0.0, LinearRgba::BLACK),  // 3  Eye White
        (Color::srgb(0.05, 0.05, 0.05), 0.0, LinearRgba::BLACK),  // 4  Pupil
        (Color::srgb(0.20, 0.40, 0.70), 0.0, LinearRgba::BLACK),  // 5  Iris (blue)
        (Color::srgb(0.72, 0.45, 0.20), 0.3, LinearRgba::BLACK),  // 6  Goggle Frame (copper)
        (Color::srgb(0.00, 0.90, 1.00), 0.5, LinearRgba::new(0.0, 0.5, 0.8, 1.0)), // 7 Goggle Lens (glow)
        (Color::srgb(0.18, 0.18, 0.20), 0.0, LinearRgba::BLACK),  // 8  Cloth Charcoal
        (Color::srgb(0.15, 0.25, 0.45), 0.0, LinearRgba::BLACK),  // 9  Tempest Blue
        (Color::srgb(0.35, 0.20, 0.12), 0.0, LinearRgba::BLACK),  // 10 Leather
        (Color::srgb(0.60, 0.62, 0.65), 0.6, LinearRgba::BLACK),  // 11 Steel
        (Color::srgb(0.50, 0.30, 0.20), 0.0, LinearRgba::BLACK),  // 12 Mouth
    ]
}

pub fn mech_palette() -> Vec<(Color, f32, LinearRgba)> {
    vec![
        (Color::srgb(0.28, 0.30, 0.33), 0.85, LinearRgba::BLACK),                     // 0 Gunmetal
        (Color::srgb(0.18, 0.19, 0.22), 0.90, LinearRgba::BLACK),                     // 1 Gunmetal Dark
        (Color::srgb(0.40, 0.42, 0.45), 0.80, LinearRgba::BLACK),                     // 2 Gunmetal Light
        (Color::srgb(0.90, 0.45, 0.05), 0.30, LinearRgba::BLACK),                     // 3 Hazard Orange
        (Color::srgb(0.00, 0.85, 1.00), 0.90, LinearRgba::new(0.0, 0.6, 0.8, 1.0)),  // 4 Visor Cyan (emissive)
        (Color::srgb(1.00, 0.90, 0.40), 0.50, LinearRgba::new(0.8, 0.7, 0.2, 1.0)),  // 5 Lamp Yellow (emissive)
        (Color::srgb(0.95, 0.40, 0.05), 0.20, LinearRgba::new(0.7, 0.25, 0.0, 1.0)), // 6 Exhaust Glow (emissive)
        (Color::srgb(0.12, 0.12, 0.14), 0.95, LinearRgba::BLACK),                     // 7 Hydraulic Dark
        (Color::srgb(0.55, 0.55, 0.58), 0.90, LinearRgba::BLACK),                     // 8 Rivet Steel
    ]
}

// ========================================================
// 3D Voxel Grid Helper (Extended for SDFs)
// ========================================================
pub struct Grid3D {
    pub sx: usize,
    pub sy: usize,
    pub sz: usize,
    pub data: Vec<i8>,
}

impl Grid3D {
    pub fn new(sx: usize, sy: usize, sz: usize) -> Self {
        Self { sx, sy, sz, data: vec![-1; sx * sy * sz] }
    }

    #[inline]
    fn idx(&self, x: usize, y: usize, z: usize) -> usize {
        y * self.sz * self.sx + z * self.sx + x
    }

    pub fn set(&mut self, x: usize, y: usize, z: usize, c: i8) {
        if x < self.sx && y < self.sy && z < self.sz {
            let i = self.idx(x, y, z);
            self.data[i] = c;
        }
    }

    pub fn get(&self, x: usize, y: usize, z: usize) -> i8 {
        if x < self.sx && y < self.sy && z < self.sz {
            self.data[self.idx(x, y, z)]
        } else { -1 }
    }

    /// Fill a rounded cylinder along an axis
    pub fn draw_cylinder(&mut self, p0: Vec3, p1: Vec3, radius: f32, c: i8) {
        let v = p1 - p0;
        let v_len_sq = v.length_squared();
        for y in 0..self.sy {
            for z in 0..self.sz {
                for x in 0..self.sx {
                    let p = Vec3::new(x as f32, y as f32, z as f32);
                    let t = ((p - p0).dot(v) / v_len_sq).clamp(0.0, 1.0);
                    let projection = p0 + t * v;
                    if (p - projection).length() <= radius {
                        self.set(x, y, z, c);
                    }
                }
            }
        }
    }

    /// Fill a sphere
    pub fn draw_sphere(&mut self, center: Vec3, radius: f32, c: i8) {
        for y in 0..self.sy {
            for z in 0..self.sz {
                for x in 0..self.sx {
                    let p = Vec3::new(x as f32, y as f32, z as f32);
                    if (p - center).length() <= radius {
                        self.set(x, y, z, c);
                    }
                }
            }
        }
    }

    /// Fill a box
    pub fn fill(&mut self, x0: usize, y0: usize, z0: usize, x1: usize, y1: usize, z1: usize, c: i8) {
        for y in y0..=y1.min(self.sy - 1) {
            for z in z0..=z1.min(self.sz - 1) {
                for x in x0..=x1.min(self.sx - 1) {
                    self.set(x, y, z, c);
                }
            }
        }
    }

    /// Generate a smooth mesh from the grid
    pub fn generate_mesh(&self, voxel_size: f32, palette_colors: &[[f32; 4]]) -> Mesh {
        let mut builder = VoxelMeshBuilder::new();
        let sx = self.sx as i32;
        let sy = self.sy as i32;
        let sz = self.sz as i32;

        for y in 0..sy {
            for z in 0..sz {
                for x in 0..sx {
                    let c_idx = self.get(x as usize, y as usize, z as usize);
                    if c_idx < 0 { continue; }
                    let color = palette_colors[c_idx as usize];

                    // Check neighbors for adjacency
                    let neighbors = [
                        (x + 1, y, z, 0), // Right
                        (x - 1, y, z, 1), // Left
                        (x, y + 1, z, 2), // Top
                        (x, y - 1, z, 3), // Bottom
                        (x, y, z + 1, 4), // Front
                        (x, y, z - 1, 5), // Back
                    ];

                    for (nx, ny, nz, face_idx) in neighbors {
                        let is_exposed = if nx < 0 || ny < 0 || nz < 0 || nx >= sx || ny >= sy || nz >= sz {
                            true
                        } else {
                            self.get(nx as usize, ny as usize, nz as usize) < 0
                        };

                        if is_exposed {
                            // Centered on 0
                            let tx = x - sx / 2;
                            let ty = y - sy / 2;
                            let tz = z - sz / 2;
                            builder.add_face(tx, ty, tz, face_idx, voxel_size, color, true);
                        }
                    }
                }
            }
        }
        builder.build()
    }
}// ========================================================
// PROPORTIONATE MODEL GENERATION (1.8m Height)
// ========================================================

pub fn build_head_mesh() -> Mesh {
    // Head: ~24x28x24 voxels
    let mut g = Grid3D::new(24, 28, 24);
    let center = Vec3::new(12.0, 10.0, 12.0); // Shifted down so joint is at base
    
    // Main skull
    g.draw_sphere(center + Vec3::Y * 4.0, 10.0, SKIN);
    
    // Face Details (Now on Front side: Z < 12)
    g.draw_sphere(Vec3::new(8.5, 14.0, 4.0), 2.0, EYE_W);
    g.draw_sphere(Vec3::new(8.5, 14.0, 3.0), 1.0, IRIS);
    g.draw_sphere(Vec3::new(15.5, 14.0, 4.0), 2.0, EYE_W);
    g.draw_sphere(Vec3::new(15.5, 14.0, 3.0), 1.0, IRIS);

    // Goggles (Now on Front side)
    g.draw_cylinder(Vec3::new(5.0, 14.0, 8.0), Vec3::new(9.5, 14.0, 8.0), 1.0, GOG_FR);
    g.draw_cylinder(Vec3::new(10.5, 14.0, 8.0), Vec3::new(15.0, 14.0, 8.0), 1.0, GOG_FR);
    g.draw_sphere(Vec3::new(7.2, 14.2, 7.8), 0.6, GOG_LN);
    g.draw_sphere(Vec3::new(12.8, 14.2, 7.8), 0.6, GOG_LN);

    // Hair (Moved to Back side)
    g.draw_sphere(center + Vec3::new(0.0, 8.0, 5.0), 10.0, HAIR);

    let pal: Vec<[f32; 4]> = base_palette().iter().map(|(c, _, _)| c.to_linear().to_f32_array()).collect();
    g.generate_mesh(V, &pal)
}

pub fn build_neck_mesh() -> Mesh {
    let mut g = Grid3D::new(10, 10, 10);
    g.draw_cylinder(Vec3::new(5.0, 0.0, 5.0), Vec3::new(5.0, 10.0, 5.0), 3.5, SKIN);
    let pal: Vec<[f32; 4]> = base_palette().iter().map(|(c, _, _)| c.to_linear().to_f32_array()).collect();
    g.generate_mesh(V, &pal)
}

pub fn build_torso_mesh() -> Mesh {
    let mut g = Grid3D::new(40, 30, 24);
    // Upper Torso
    g.draw_cylinder(Vec3::new(20.0, 0.0, 12.0), Vec3::new(20.0, 28.0, 12.0), 14.0, CLOTH_1);
    // Industrial Vest
    g.draw_cylinder(Vec3::new(20.0, 5.0, 12.0), Vec3::new(20.0, 25.0, 12.0), 15.0, CLOTH_2);
    // Shoulders
    g.draw_sphere(Vec3::new(6.0, 25.0, 12.0), 6.0, CLOTH_2);
    g.draw_sphere(Vec3::new(34.0, 25.0, 12.0), 6.0, CLOTH_2);

    let pal: Vec<[f32; 4]> = base_palette().iter().map(|(c, _, _)| c.to_linear().to_f32_array()).collect();
    g.generate_mesh(V, &pal)
}

pub fn build_pelvis_mesh() -> Mesh {
    let mut g = Grid3D::new(34, 18, 20);
    // Pelvis/Hips
    g.draw_cylinder(Vec3::new(17.0, 2.0, 10.0), Vec3::new(17.0, 16.0, 10.0), 12.0, CLOTH_1);
    // Belt
    g.draw_cylinder(Vec3::new(17.0, 12.0, 10.0), Vec3::new(17.0, 16.0, 10.0), 13.0, LEATHER);
    g.draw_sphere(Vec3::new(17.0, 14.0, 3.0), 2.5, METAL); // Buckle (MINUS Z is front)

    let pal: Vec<[f32; 4]> = base_palette().iter().map(|(c, _, _)| c.to_linear().to_f32_array()).collect();
    g.generate_mesh(V, &pal)
}

pub fn build_upper_arm_mesh() -> Mesh {
    let mut g = Grid3D::new(14, 28, 14);
    g.draw_cylinder(Vec3::new(7.0, 2.0, 7.0), Vec3::new(7.0, 26.0, 7.0), 5.5, CLOTH_1);
    let pal: Vec<[f32; 4]> = base_palette().iter().map(|(c, _, _)| c.to_linear().to_f32_array()).collect();
    g.generate_mesh(V, &pal)
}

pub fn build_forearm_mesh() -> Mesh {
    let mut g = Grid3D::new(12, 26, 12);
    g.draw_cylinder(Vec3::new(6.0, 2.0, 6.0), Vec3::new(6.0, 24.0, 6.0), 4.5, SKIN);
    let pal: Vec<[f32; 4]> = base_palette().iter().map(|(c, _, _)| c.to_linear().to_f32_array()).collect();
    g.generate_mesh(V, &pal)
}

pub fn build_hand_mesh(right: bool) -> Mesh {
    let mut g = Grid3D::new(14, 14, 14);
    g.draw_sphere(Vec3::new(7.0, 7.0, 7.0), 6.0, LEATHER);
    // Thumb
    if right {
        g.draw_sphere(Vec3::new(2.0, 7.0, 10.0), 2.5, LEATHER);
    } else {
        g.draw_sphere(Vec3::new(12.0, 7.0, 10.0), 2.5, LEATHER);
    }
    let pal: Vec<[f32; 4]> = base_palette().iter().map(|(c, _, _)| c.to_linear().to_f32_array()).collect();
    g.generate_mesh(V, &pal)
}

pub fn build_thigh_mesh() -> Mesh {
    let mut g = Grid3D::new(18, 38, 18);
    g.draw_cylinder(Vec3::new(9.0, 2.0, 9.0), Vec3::new(9.0, 36.0, 9.0), 8.0, CLOTH_1);
    let pal: Vec<[f32; 4]> = base_palette().iter().map(|(c, _, _)| c.to_linear().to_f32_array()).collect();
    g.generate_mesh(V, &pal)
}

pub fn build_calf_mesh() -> Mesh {
    let mut g = Grid3D::new(16, 36, 16);
    g.draw_cylinder(Vec3::new(8.0, 10.0, 8.0), Vec3::new(8.0, 34.0, 8.0), 7.0, CLOTH_1);
    // Boot upper
    g.draw_cylinder(Vec3::new(8.0, 2.0, 8.0), Vec3::new(8.0, 12.0, 8.0), 7.5, LEATHER);
    let pal: Vec<[f32; 4]> = base_palette().iter().map(|(c, _, _)| c.to_linear().to_f32_array()).collect();
    g.generate_mesh(V, &pal)
}

pub fn build_foot_mesh() -> Mesh {
    let mut g = Grid3D::new(16, 10, 24);
    // Heel to toe
    g.draw_cylinder(Vec3::new(8.0, 4.0, 6.0), Vec3::new(8.0, 4.0, 18.0), 6.5, LEATHER);
    g.draw_sphere(Vec3::new(8.0, 4.0, 18.0), 7.0, LEATHER); // Toe box
    let pal: Vec<[f32; 4]> = base_palette().iter().map(|(c, _, _)| c.to_linear().to_f32_array()).collect();
    g.generate_mesh(V, &pal)
}

pub fn build_body_mesh() -> Mesh {
    // 0.6m torso -> 48 voxels
    let mut g = Grid3D::new(40, 50, 24);
    let _center = Vec3::new(20.0, 25.0, 12.0);

    // Main Torso (SDF cylinder base)
    g.draw_cylinder(Vec3::new(20.0, 5.0, 12.0), Vec3::new(20.0, 45.0, 12.0), 14.0, CLOTH_1);
    
    // Shoulders
    g.draw_sphere(Vec3::new(6.0, 42.0, 12.0), 6.0, CLOTH_2);
    g.draw_sphere(Vec3::new(34.0, 42.0, 12.0), 6.0, CLOTH_2);

    // Industrial Vest
    g.draw_cylinder(Vec3::new(20.0, 15.0, 12.0), Vec3::new(20.0, 40.0, 12.0), 15.0, CLOTH_2);
    // Shoulder straps
    g.fill(5, 40, 4, 10, 48, 20, LEATHER);
    g.fill(30, 40, 4, 35, 48, 20, LEATHER);
    
    // Belt
    g.draw_cylinder(Vec3::new(20.0, 8.0, 12.0), Vec3::new(20.0, 12.0, 12.0), 15.5, LEATHER);
    g.draw_sphere(Vec3::new(20.0, 10.0, 2.0), 2.5, METAL); // Buckle

    let pal: Vec<[f32; 4]> = base_palette().iter().map(|(c, _, _)| c.to_linear().to_f32_array()).collect();
    g.generate_mesh(V, &pal)
}

pub fn build_arm_mesh(right: bool) -> Mesh {
    // 0.7m arm -> 56 voxels
    let mut g = Grid3D::new(14, 58, 14);
    let center_x = 7.0;
    
    // Upper arm (Sleeve)
    g.draw_cylinder(Vec3::new(center_x, 56.0, 7.0), Vec3::new(center_x, 32.0, 7.0), 5.5, CLOTH_1);
    
    // Forearm (Skin)
    g.draw_cylinder(Vec3::new(center_x, 32.0, 7.0), Vec3::new(center_x, 10.0, 7.0), 4.0, SKIN);
    
    // Glove
    g.draw_sphere(Vec3::new(center_x, 6.0, 7.0), 5.0, LEATHER);
    if right {
        g.draw_sphere(Vec3::new(2.0, 4.0, 7.0), 1.5, SKIN); // Thumb
    } else {
        g.draw_sphere(Vec3::new(12.0, 4.0, 7.0), 1.5, SKIN);
    }

    let pal: Vec<[f32; 4]> = base_palette().iter().map(|(c, _, _)| c.to_linear().to_f32_array()).collect();
    g.generate_mesh(V, &pal)
}

pub fn build_leg_mesh() -> Mesh {
    // 0.95m leg -> 76 voxels
    let mut g = Grid3D::new(18, 78, 18);
    let center_x = 9.0;

    // Thigh (Pants)
    g.draw_cylinder(Vec3::new(center_x, 76.0, 9.0), Vec3::new(center_x, 35.0, 9.0), 7.5, CLOTH_1);
    
    // Calf (Pants)
    g.draw_cylinder(Vec3::new(center_x, 35.0, 9.0), Vec3::new(center_x, 15.0, 9.0), 6.5, CLOTH_1);
    
    // Boot
    g.draw_cylinder(Vec3::new(center_x, 15.0, 9.0), Vec3::new(center_x, 2.0, 9.0), 7.8, LEATHER);
    g.draw_sphere(Vec3::new(center_x, 2.0, 16.0), 5.0, LEATHER); // Toe
    let pal: Vec<[f32; 4]> = base_palette().iter().map(|(c, _, _)| c.to_linear().to_f32_array()).collect();
    g.generate_mesh(V, &pal)
}

// ========================================================
// MECH — EXISTING PIECES (Keep for now, rescale later)
// ========================================================

pub fn mech_helmet() -> Vec<(Vec3, Vec3, usize)> {
    vec![
        (Vec3::new(0.0, 0.025, 0.0), Vec3::new(0.35, 0.35, 0.35), 0),   // Main helmet frame
        (Vec3::new(0.0, 0.15, 0.0),  Vec3::new(0.20, 0.10, 0.30), 1),   // Top detail
        (Vec3::new(-0.16, 0.025, 0.0), Vec3::new(0.06, 0.15, 0.20), 1), // Ear piece L
        (Vec3::new(0.16, 0.025, 0.0),  Vec3::new(0.06, 0.15, 0.20), 1), // Ear piece R
        (Vec3::new(0.0, 0.04, -0.16),  Vec3::new(0.28, 0.10, 0.05), 4), // Visor
        (Vec3::new(0.0, -0.12, -0.10), Vec3::new(0.25, 0.08, 0.15), 2), // Jaw plate
    ]
}

pub fn mech_chest() -> Vec<(Vec3, Vec3, usize)> {
    vec![
        (Vec3::new(0.0, 0.0, 0.0),    Vec3::new(0.48, 0.65, 0.42), 0), // Main chest plate
        (Vec3::new(0.0, 0.05, -0.18), Vec3::new(0.38, 0.45, 0.10), 2), // Front reinforced plate
        (Vec3::new(0.0, 0.15, -0.22), Vec3::new(0.25, 0.12, 0.04), 3), // Chest logo/hazard
    ]
}

pub fn mech_reactor() -> Vec<(Vec3, Vec3, usize)> {
    vec![
        (Vec3::new(0.0, 0.0, 0.22),     Vec3::new(0.35, 0.45, 0.20), 1), // Core housing
        (Vec3::new(0.0, 0.18, 0.25),    Vec3::new(0.38, 0.08, 0.22), 0), // Upper cowl
        (Vec3::new(-0.12, -0.15, 0.28), Vec3::new(0.12, 0.15, 0.12), 7), // Hydraulic L
        (Vec3::new(0.12, -0.15, 0.28),  Vec3::new(0.12, 0.15, 0.12), 7), // Hydraulic R
    ]
}

pub fn mech_shoulder_left() -> Vec<(Vec3, Vec3, usize)> {
    vec![(Vec3::new(-0.1, 0.25, 0.0), Vec3::new(0.45, 0.18, 0.42), 0)]
}

pub fn mech_shoulder_right() -> Vec<(Vec3, Vec3, usize)> {
    vec![(Vec3::new(0.1, 0.25, 0.0), Vec3::new(0.45, 0.18, 0.42), 0)]
}

pub fn mech_gauntlet() -> Vec<(Vec3, Vec3, usize)> {
    vec![(Vec3::new(0.0, -0.5, 0.0), Vec3::new(0.38, 0.55, 0.38), 0)]
}

pub fn mech_leg_armor() -> Vec<(Vec3, Vec3, usize)> {
    vec![
        (Vec3::new(0.0, 0.05, -0.08),  Vec3::new(0.24, 0.45, 0.10), 0), // Shin guard
        (Vec3::new(0.0, -0.15, -0.12), Vec3::new(0.22, 0.18, 0.14), 2), // Knee plating
    ]
}

