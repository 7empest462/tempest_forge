use bevy::prelude::*;
use bevy::tasks::{AsyncComputeTaskPool, Task};
use bevy_voxel_world::prelude::*;
use fastrand::Rng as FastRng;
use futures_lite::future;
use crate::voxel::chunk::BlockType;
use crate::world::noise_generator::NoiseGenerator;
use crate::player::combat::{Health, Hittable};
use crate::world::settlement::{SettlementRegistry, SettlementBuilding};

#[derive(Component)]
pub struct TreeEntity;
// shrubbery: pure-Rust space-colonization library (no Bevy dep)
use shrubbery::shrubbery::Shrubbery;
use shrubbery::algorithm_settings::AlgorithmSettings;
use shrubbery::attractor_generator_settings::AttractorGeneratorSettings;
use shrubbery::shape::BoxShape;
use shrubbery::voxel::{voxelize, VoxelizeSettings, BranchSizeSetting, BranchRootSizeIncreaser, LeafSetting};

// shrubbery uses glam 0.30 — re-export its vec3 so we don't confuse the two glam versions
use shrubbery::glam::vec3 as svec3;

use bevy_procedural_tree::Tree;
use bevy_procedural_tree::settings::{TreeMeshSettings, BranchParams, LeafParams, BranchRecursionLevel};
use bevy_procedural_tree::enums::{TreeType as ProcTreeType};

// ─────────────────────────────────────────────────────────────────────────────
// Tree type enum — controls colonization & leaf parameters per biome
// ─────────────────────────────────────────────────────────────────────────────

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum TreeType {
    Oak,    // wide rounded crown, thick trunk
    Pine,   // tall narrow cone, sparse leaves
    Birch,  // medium height, wispy crown
    Jungle, // very tall, thin trunk, large flat canopy
}

// ─────────────────────────────────────────────────────────────────────────────
// Components & Resources
// ─────────────────────────────────────────────────────────────────────────────

#[derive(Resource)]
pub struct TreeGenerator {
    pub rng: FastRng,
}

impl Default for TreeGenerator {
    fn default() -> Self {
        Self { rng: FastRng::with_seed(1337) }
    }
}

#[derive(Component)]
pub struct TreeSpawnRequest {
    pub pos: IVec3,
    pub tree_type: TreeType,
}

#[derive(Component)]
pub struct TreeGenerationTask {
    pub task: Task<TreeGenerationResult>,
}

pub struct TreeGenerationResult {
    pub pos: IVec3,
    pub _tree_type: TreeType,
    pub canopy_settings: TreeMeshSettings,
}

// ─────────────────────────────────────────────────────────────────────────────
// ECS systems
// ─────────────────────────────────────────────────────────────────────────────

#[derive(Component)]
pub struct Decorated;

pub fn chunk_vegetation_system(
    mut commands: Commands,
    mut tree_gen: ResMut<TreeGenerator>,
    query: Query<(Entity, &Chunk<NoiseGenerator>), Without<Decorated>>,
    voxel_world: VoxelWorld<NoiseGenerator>,
    noise_gen: Res<NoiseGenerator>,
    registry: Res<SettlementRegistry>,
) {
    for (entity, chunk_comp) in query.iter() {
        let chunk_key = chunk_comp.position;
        
        // Avoid spawning trees in settlement chunks
        let chunk_hash = (chunk_key.x.wrapping_mul(73856093) ^ chunk_key.z.wrapping_mul(19349663)).abs();
        let is_settlement_chunk = (chunk_hash % 10) == 0 && (chunk_key.x != 0 || chunk_key.z != 0);
        
        let mut spawn_count = 0;
        let mut rng = FastRng::with_seed((chunk_key.x as u64) << 32 | (chunk_key.z as u64));

        for i in 0..20 { 
            if is_settlement_chunk { continue; } // Clear the entire settlement chunk of trees
            
            let x = rng.i32(0..16);
            let z = rng.i32(0..16);
            let world_x = (chunk_key.x * 16) + x;
            let world_z = (chunk_key.z * 16) + z;
            
            let terrain = noise_gen.get_terrain(world_x as f32, world_z as f32);
            let adjusted_surface = noise_gen.get_adjusted_surface_height(world_x as f32, world_z as f32);
            
            // Check if near any house/building (within 7.5 meters)
            if is_near_settlement_building(Vec3::new(world_x as f32, adjusted_surface, world_z as f32), &registry) {
                continue;
            }

            // 1. Vertical Filtering: Only spawn if the surface belongs to THIS vertical chunk
            let surface_y = adjusted_surface.floor() as i32;
            let chunk_y = (surface_y as f32 / 16.0).floor() as i32;
            if chunk_y != chunk_key.y {
                continue; 
            }

            // 2. Suitability: Trees grow on dry land (above sea level 15.0)
            let flora_val = noise_gen.get_flora(world_x as f32, world_z as f32);
            
            // Biome Density: 
            // Forest (> 0.4): High density
            // Sparse Plains (< -0.4): Very low density
            // Regular: Medium density
            let density_limit = if flora_val > 0.4 {
                14 // Dense Forest
            } else if flora_val < -0.4 {
                1  // Sparse Plains
            } else {
                4 // Regular transition
            };

            if i >= density_limit { continue; }

            if adjusted_surface > 16.0 && adjusted_surface < 120.0 && !terrain.is_desert {
                let pos = IVec3::new(world_x, surface_y, world_z);
                
                // Spawn the tree task
                let tree_type = if flora_val > 0.6 {
                    TreeType::Jungle // Deep forest has bigger trees
                } else if i % 5 == 0 {
                    TreeType::Oak 
                } else { 
                    TreeType::Pine 
                };
                // println!("  QUEUING TREE at {:?} (height {:.1})", pos, adjusted_surface);
                commands.spawn(TreeSpawnRequest {
                    pos,
                    tree_type,
                });
                spawn_count += 1;
            } else if i == 0 {
                // Log why we failed at least once per chunk
                // println!("  Tree skip at {:.1}: desert={}, y_match={}", adjusted_surface, terrain.is_desert, chunk_y == chunk_key.y);
            }
        }
        
        commands.entity(entity).insert(Decorated);
        if spawn_count > 0 {
            // println!("  SPAWNED {} TREE TASKS in chunk {:?}", spawn_count, chunk_key);
        }

        let candidates = if is_settlement_chunk { Vec::new() } else { scatter_candidates(chunk_key, 4, &mut tree_gen.rng) };
        for pos_2d in candidates {
            // Apply flora moisture check to candidates to match biomes and reduce plains density
            let flora_val = noise_gen.get_flora(pos_2d.x as f32, pos_2d.y as f32);
            let spawn_chance = if flora_val > 0.4 {
                0.80 // Forest
            } else if flora_val < -0.4 {
                0.02 // Sparse Plains
            } else {
                0.15 // Regular plains / transition
            };

            if tree_gen.rng.f32() > spawn_chance {
                continue;
            }

            let Some(height) = crate::world::manager::find_stable_ground_height(
                Vec3::new(pos_2d.x as f32, 100.0, pos_2d.y as f32),
                &voxel_world,
            ) else { continue };

            // Check if near any house/building (within 7.5 meters)
            if is_near_settlement_building(Vec3::new(pos_2d.x as f32, height, pos_2d.y as f32), &registry) {
                continue;
            }

            // Only spawn if the height is within THIS chunk's vertical bounds
            if (height / 16.0).floor() as i32 != chunk_key.y {
                continue;
            }

            let pos = IVec3::new(pos_2d.x, height as i32, pos_2d.y);
            let surface = voxel_world.get_voxel(pos - IVec3::Y);
            let surface_type = match surface {
                WorldVoxel::Solid(id) => BlockType::from(id),
                _ => BlockType::Air,
            };

            let tree_type = match surface_type {
                BlockType::Grass  => pick_grass_tree(&mut tree_gen.rng),
                BlockType::Dirt   => Some(TreeType::Oak),
                BlockType::Podzol => Some(TreeType::Pine),
                _ => None,
            };

            if let Some(tt) = tree_type {
                // println!("  SPAWNING {:?} TREE at {:?}", tt, pos);
                commands.spawn(TreeSpawnRequest {
                    pos,
                    tree_type: tt,
                });
            }
        }
    }
}

pub fn start_tree_generation(
    mut commands: Commands,
    requests: Query<(Entity, &TreeSpawnRequest)>,
) {
    let thread_pool = AsyncComputeTaskPool::get();
    for (entity, request) in requests.iter() {
        let pos = request.pos;
        let tree_type = request.tree_type;

        let task = thread_pool.spawn(async move {
            generate_tree_data(pos, tree_type)
        });

        commands.entity(entity)
            .remove::<TreeSpawnRequest>()
            .insert(TreeGenerationTask { task });
    }
}

pub fn complete_tree_generation(
    mut commands: Commands,
    mut tasks: Query<(Entity, &mut TreeGenerationTask)>,
    _meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    for (entity, mut task_comp) in tasks.iter_mut() {
        if let Some(result) = future::block_on(future::poll_once(&mut task_comp.task)) {
            let bark_mat = materials.add(StandardMaterial {
                base_color: Color::srgb(0.4, 0.25, 0.1), // Brown bark
                perceptual_roughness: 0.9,
                ..default()
            });
            let leaf_mat = materials.add(StandardMaterial {
                base_color: Color::srgb(0.1, 0.5, 0.1), // Forest green
                ..default()
            });

            // Spawn mesh canopy
            commands.entity(entity)
                .remove::<TreeGenerationTask>()
                .insert((
                    Transform::from_translation(result.pos.as_vec3() + Vec3::new(0.0, 1.0, 0.0)),
                    Visibility::default(),
                    InheritedVisibility::default(),
                    Tree {
                        seed: (result.pos.x as u64 ^ result.pos.z as u64),
                        tree_mesh_settings_override: Some(result.canopy_settings),
                        bark_material_override: Some(MeshMaterial3d(bark_mat.clone())),
                        leaf_material_override: Some(MeshMaterial3d(leaf_mat)),
                    },
                    MeshMaterial3d(bark_mat),
                    bevy_rapier3d::prelude::Collider::cylinder(3.0, 0.4),
                    Hittable,
                    Health::new(50.0), // Trees have 50 HP
                    TreeEntity,
                ));
        }
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// Generation Logic (runs in async task)
// ─────────────────────────────────────────────────────────────────────────────

fn generate_tree_data(base_pos: IVec3, tree_type: TreeType) -> TreeGenerationResult {
    // 1. Shrubbery Voxel Trunk Generation
    let (
        crown_w, crown_h, crown_offset_y,
        branch_len, attraction_dist, kill_dist,
        trunk_min_h, grow_iters,
        branch_thickness,
    ) = match tree_type {
        TreeType::Oak => (9.0, 7.0, 6.0, 0.6, 5.5, 0.5, 3.0, 14, 0.55),
        TreeType::Pine => (4.0, 10.0, 8.0, 0.5, 4.5, 0.45, 5.0, 10, 0.4),
        TreeType::Birch => (6.0, 6.0, 5.0, 0.55, 5.0, 0.5, 2.5, 12, 0.45),
        TreeType::Jungle => (12.0, 5.0, 10.0, 0.65, 6.0, 0.55, 7.0, 16, 0.5),
    };

    let seed = (base_pos.x.unsigned_abs() as u64)
        .wrapping_mul(2654435761)
        ^ (base_pos.z.unsigned_abs() as u64).wrapping_mul(1234567891)
        ^ (tree_type as u64) * 999983;

    let algo = AlgorithmSettings {
        seed,
        branch_len,
        leaf_attraction_dist: attraction_dist,
        kill_distance: kill_dist,
        min_trunk_height: trunk_min_h,
    };

    let mut shrub = Shrubbery::new(
        svec3(0.0, 0.0, 0.0),
        svec3(0.0, 1.0, 0.0),
        algo,
        AttractorGeneratorSettings {
            density: 1.2,
            max_leaves: Some(600),
            min_leaves: Some(40),
        },
    );

    shrub.spawn_attractors_from_shape(
        svec3(0.0, crown_offset_y, 0.0),
        BoxShape { x: crown_w, y: crown_h, z: crown_w },
    );

    shrub.build_trunk();
    for _ in 0..grow_iters { shrub.grow(); }
    if matches!(tree_type, TreeType::Oak | TreeType::Birch) { shrub.post_process_gravity(0.8); }

    let voxelize_settings = VoxelizeSettings {
        leaf_settings: LeafSetting::None,
        branch_size_setting: BranchSizeSetting::Generation {
            distances: vec![
                branch_thickness + 0.35,
                branch_thickness + 0.20,
                branch_thickness + 0.10,
                branch_thickness,
            ],
        },
        branch_root_size_increaser: Some(BranchRootSizeIncreaser {
            height: trunk_min_h * 0.6,
            additional_size: 0.5,
        }),
    };

    let _voxels = voxelize(&shrub, &voxelize_settings);

    // 2. Procedural Mesh Canopy Settings
    let canopy_settings = match tree_type {
        TreeType::Oak => TreeMeshSettings {
            tree_type: ProcTreeType::Deciduous,
            branch: BranchParams {
                levels: BranchRecursionLevel::Two,
                length: [4.0, 3.0, 1.5, 0.0],
                angle: [0.0, 45.0, 35.0, 0.0],
                children: [4, 3, 0],
                trunk_base_radius: 1.0,
                ..default()
            },
            leaves: LeafParams {
                count: 8,
                size: 0.6,
                ..default()
            },
        },
        TreeType::Pine => TreeMeshSettings {
            tree_type: ProcTreeType::Evergreen,
            branch: BranchParams {
                levels: BranchRecursionLevel::Three,
                length: [6.0, 2.5, 1.0, 0.5],
                angle: [0.0, 80.0, 45.0, 30.0],
                children: [8, 5, 4],
                trunk_base_radius: 0.08,
                ..default()
            },
            leaves: LeafParams {
                count: 12,
                size: 0.4,
                ..default()
            },
        },
        TreeType::Birch => TreeMeshSettings {
            tree_type: ProcTreeType::Deciduous,
            branch: BranchParams {
                levels: BranchRecursionLevel::Two,
                length: [5.0, 2.5, 1.2, 0.0],
                angle: [0.0, 30.0, 25.0, 0.0],
                children: [3, 4, 0],
                trunk_base_radius: 0.07,
                ..default()
            },
            leaves: LeafParams {
                count: 5,
                size: 0.5,
                ..default()
            },
        },
        TreeType::Jungle => TreeMeshSettings {
            tree_type: ProcTreeType::Deciduous,
            branch: BranchParams {
                levels: BranchRecursionLevel::Three,
                length: [8.0, 4.0, 2.0, 1.0],
                angle: [0.0, 60.0, 45.0, 30.0],
                children: [6, 5, 4],
                trunk_base_radius: 0.12,
                ..default()
            },
            leaves: LeafParams {
                count: 10,
                size: 0.8,
                ..default()
            },
        },
    };

    TreeGenerationResult {
        pos: base_pos,
        _tree_type: tree_type,
        canopy_settings,
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// Helpers
// ─────────────────────────────────────────────────────────────────────────────

fn pick_grass_tree(rng: &mut FastRng) -> Option<TreeType> {
    match rng.u8(0..10) {
        0..=4 => Some(TreeType::Oak),
        5..=7 => Some(TreeType::Birch),
        8     => Some(TreeType::Jungle),
        _     => None,
    }
}

fn scatter_candidates(chunk_2d: IVec3, count: usize, rng: &mut FastRng) -> Vec<IVec2> {
    let base_x = chunk_2d.x * 16;
    let base_z = chunk_2d.z * 16;
    let cell = 16 / count.max(1) as i32;
    (0..count).map(|i| {
        let cx = base_x + (i as i32) * cell + rng.i32(0..cell.max(1));
        let cz = base_z + rng.i32(0..16);
        IVec2::new(cx, cz)
    }).collect()
}

fn is_near_settlement_building(pos: Vec3, registry: &SettlementRegistry) -> bool {
    const BUILDING_OFFSETS: [(f32, f32); 8] = [
        (0.0, 16.0),   // Tavern
        (22.0, 6.0),   // Shop
        (-22.0, 6.0),  // Forge
        (0.0, -26.0),  // Farm
        (18.0, -18.0), // House 1
        (-18.0, -18.0),// House 2
        (26.0, -26.0), // Guard Tower
        (0.0, 0.0),    // Plaza / Well
    ];
    
    for chunk_pos in registry.positions.iter() {
        let town_center_x = (chunk_pos.x * 16 + 8) as f32;
        let town_center_z = (chunk_pos.z * 16 + 8) as f32;
        
        for &(dx, dz) in BUILDING_OFFSETS.iter() {
            let bx = town_center_x + dx;
            let bz = town_center_z + dz;
            let dist_sq = (pos.x - bx) * (pos.x - bx) + (pos.z - bz) * (pos.z - bz);
            // 7.5 meters squared = 56.25
            if dist_sq < 56.25 {
                return true;
            }
        }
    }
    false
}

pub fn despawn_trees_near_buildings(
    mut commands: Commands,
    new_buildings: Query<&SettlementBuilding, Added<SettlementBuilding>>,
    all_buildings: Query<&SettlementBuilding>,
    new_trees: Query<(Entity, &Transform), (Added<TreeEntity>, With<TreeEntity>)>,
    all_trees: Query<(Entity, &Transform), With<TreeEntity>>,
) {
    let mut trees_to_despawn = std::collections::HashSet::new();

    // Case 1: New building spawned, despawn any nearby existing trees
    for b in new_buildings.iter() {
        for (tree_entity, tree_transform) in all_trees.iter() {
            let dist_sq = b.center.distance_squared(tree_transform.translation);
            if dist_sq < 56.25 {
                trees_to_despawn.insert(tree_entity);
            }
        }
    }

    // Case 2: New tree spawned (e.g. async generation finished), double check it is not near any building
    for (tree_entity, tree_transform) in new_trees.iter() {
        for b in all_buildings.iter() {
            let dist_sq = b.center.distance_squared(tree_transform.translation);
            if dist_sq < 56.25 {
                trees_to_despawn.insert(tree_entity);
                break;
            }
        }
    }

    for tree_entity in trees_to_despawn {
        if let Ok(mut entity_cmd) = commands.get_entity(tree_entity) {
            entity_cmd.despawn();
        }
    }
}
