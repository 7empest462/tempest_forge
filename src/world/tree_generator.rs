use crate::player::combat::{Health, Hittable};
use crate::voxel::chunk::BlockType;
use crate::world::noise_generator::NoiseGenerator;
use crate::world::settlement::{SettlementBuilding, SettlementRegistry};
use bevy::prelude::*;
use bevy::tasks::{AsyncComputeTaskPool, Task};
use bevy_voxel_world::prelude::*;
use fastrand::Rng as FastRng;
use futures_lite::future;

#[derive(Component)]
pub struct TreeEntity;
// shrubbery: pure-Rust space-colonization library (no Bevy dep)
use shrubbery::algorithm_settings::AlgorithmSettings;
use shrubbery::attractor_generator_settings::AttractorGeneratorSettings;
use shrubbery::shape::BoxShape;
use shrubbery::shrubbery::Shrubbery;
use shrubbery::voxel::{
    BranchRootSizeIncreaser, BranchSizeSetting, LeafSetting, VoxelizeSettings, voxelize,
};

// shrubbery uses glam 0.30 — re-export its vec3 so we don't confuse the two glam versions
use shrubbery::glam::vec3 as svec3;

use bevy_procedural_tree::enums::TreeType as ProcTreeType;
use bevy_procedural_tree::settings::{
    BranchParams, BranchRecursionLevel, LeafParams, TreeMeshSettings,
};

// ─────────────────────────────────────────────────────────────────────────────
// Tree type enum — controls colonization & leaf parameters per biome
// ─────────────────────────────────────────────────────────────────────────────

#[derive(Clone, Copy, PartialEq, Eq, Debug, Hash)]
pub enum TreeType {
    Oak,      // wide rounded crown, thick trunk
    Pine,     // tall narrow cone, sparse leaves
    Birch,    // medium height, wispy crown
    Jungle,   // very tall, thin trunk, large flat canopy
    Mushroom, // Giant bioluminescent mushroom with wide umbrella cap
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
        Self {
            rng: FastRng::with_seed(1337),
        }
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
    pub branch_mesh: Mesh,
    pub leaf_mesh: Mesh,
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
    let undecorated_count = query.iter().count();
    if undecorated_count > 0 {
        trace!(
            "[VEGETATION] chunk_vegetation_system: processing {} undecorated chunks",
            undecorated_count
        );
    }

    for (entity, chunk_comp) in query.iter() {
        let chunk_key = chunk_comp.position;

        // Avoid spawning trees in settlement chunks
        let chunk_hash =
            (chunk_key.x.wrapping_mul(73856093) ^ chunk_key.z.wrapping_mul(19349663)).abs();
        let is_settlement_chunk = (chunk_hash % 10) == 0 && (chunk_key.x != 0 || chunk_key.z != 0);

        if chunk_key.x >= 156 {
            info!(
                "[VEGETATION] Processing alien chunk {:?} is_settlement_chunk={}",
                chunk_key, is_settlement_chunk
            );
        }

        let mut spawn_count = 0;
        let mut rng = FastRng::with_seed((chunk_key.x as u64) << 32 | (chunk_key.z as u64));
        let mut spawned_positions: Vec<IVec3> = Vec::new();

        for i in 0..20 {
            if is_settlement_chunk {
                continue;
            } // Clear the entire settlement chunk of trees

            let x = rng.i32(0..32);
            let z = rng.i32(0..32);
            let world_x = (chunk_key.x * 32) + x;
            let world_z = (chunk_key.z * 32) + z;

            // Enforce minimum 6.0m spacing between trees and mushrooms
            let pos_2d = Vec2::new(world_x as f32, world_z as f32);
            let too_close = spawned_positions.iter().any(|prev_pos| {
                let prev_2d = Vec2::new(prev_pos.x as f32, prev_pos.z as f32);
                pos_2d.distance_squared(prev_2d) < 36.0 // 6.0m minimum spacing
            });
            if too_close {
                continue;
            }

            let terrain = noise_gen.get_terrain(world_x as f32, world_z as f32);
            let adjusted_surface =
                noise_gen.get_adjusted_surface_height(world_x as f32, world_z as f32);

            // Check if near any house/building (within 7.5 meters)
            if is_near_settlement_building(
                Vec3::new(world_x as f32, adjusted_surface, world_z as f32),
                &registry,
            ) {
                continue;
            }

            // 1. Vertical Filtering: Only spawn if the surface belongs to THIS vertical chunk
            let surface_y = adjusted_surface.floor() as i32;
            let chunk_y = (surface_y as f32 / 32.0).floor() as i32;
            if chunk_y != chunk_key.y {
                if chunk_key.x >= 156 && i == 0 {
                    info!(
                        "[VEGETATION] Alien chunk skip: chunk_y={} != chunk_key.y={} (surface_y={})",
                        chunk_y, chunk_key.y, surface_y
                    );
                }
                continue;
            }

            // 2. Suitability: Trees grow on dry land (above sea level 15.0)
            let flora_val = noise_gen.get_flora(world_x as f32, world_z as f32);

            // Biome Density:
            // Alien dimension: High alien tree & mushroom density (12)
            // Normal Forest (> 0.4): High density
            // Normal Sparse Plains (< -0.4): Very low density
            let density_limit = if world_x >= 5000 {
                12 // Alien flora & mushroom paradise
            } else if flora_val > 0.4 {
                14 // Dense Forest
            } else if flora_val < -0.4 {
                1 // Sparse Plains
            } else {
                4 // Regular transition
            };

            if i >= density_limit {
                if chunk_key.x >= 156 && i == 0 {
                    info!(
                        "[VEGETATION] Alien chunk skip: i={} >= density_limit={} (flora_val={:.3})",
                        i, density_limit, flora_val
                    );
                }
                continue;
            }
            let min_height = if world_x >= 5000 { 3.0 } else { 16.0 };
            if adjusted_surface > min_height && adjusted_surface < 120.0 && !terrain.is_desert {
                let pos = IVec3::new(world_x, surface_y, world_z);

                // Spawn the tree task with spatial grove clustering
                let tree_type = if world_x >= 5000 {
                    let grove_val =
                        noise_gen.get_flora(world_x as f32 * 0.05, world_z as f32 * 0.05);
                    if grove_val > 0.0 {
                        TreeType::Mushroom
                    } else {
                        TreeType::Jungle
                    }
                } else if flora_val > 0.6 {
                    TreeType::Jungle // Deep forest has bigger trees
                } else if i % 5 == 0 {
                    TreeType::Oak
                } else {
                    TreeType::Pine
                };
                if world_x >= 5000 {
                    info!(
                        "[VEGETATION] Queuing alien tree spawn at {:?} (height {:.1})",
                        pos, adjusted_surface
                    );
                }
                spawned_positions.push(pos);
                commands.spawn(TreeSpawnRequest { pos, tree_type });
                spawn_count += 1;
            } else {
                if chunk_key.x >= 156 && i == 0 {
                    info!(
                        "[VEGETATION] Alien chunk skip suitability: surface={:.1}, desert={}",
                        adjusted_surface, terrain.is_desert
                    );
                }
            }
        }

        commands.entity(entity).insert(Decorated);
        if spawn_count > 0 {
            // println!("  SPAWNED {} TREE TASKS in chunk {:?}", spawn_count, chunk_key);
        }

        let candidates = if is_settlement_chunk {
            Vec::new()
        } else {
            scatter_candidates(chunk_key, 4, &mut tree_gen.rng)
        };
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
            ) else {
                continue;
            };

            // Check if near any house/building (within 7.5 meters)
            if is_near_settlement_building(
                Vec3::new(pos_2d.x as f32, height, pos_2d.y as f32),
                &registry,
            ) {
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
                BlockType::Grass => pick_grass_tree(&mut tree_gen.rng),
                BlockType::Dirt => Some(TreeType::Oak),
                BlockType::Podzol => Some(TreeType::Pine),
                BlockType::GlowingMoss => {
                    if tree_gen.rng.f32() > 0.5 {
                        Some(TreeType::Mushroom)
                    } else {
                        Some(TreeType::Jungle)
                    }
                }
                BlockType::AlienDirt => Some(TreeType::Mushroom),
                _ => None,
            };

            if let Some(tt) = tree_type {
                if pos.x >= 5000 {
                    info!(
                        "[VEGETATION] Queuing alien candidate tree spawn at {:?} (surface: {:?})",
                        pos, surface_type
                    );
                }
                commands.spawn(TreeSpawnRequest { pos, tree_type: tt });
            }
        }
    }
}

pub fn start_tree_generation(mut commands: Commands, requests: Query<(Entity, &TreeSpawnRequest)>) {
    let thread_pool = AsyncComputeTaskPool::get();
    for (entity, request) in requests.iter() {
        let pos = request.pos;
        let tree_type = request.tree_type;

        let task = thread_pool.spawn(async move { generate_tree_data(pos, tree_type) });

        commands
            .entity(entity)
            .remove::<TreeSpawnRequest>()
            .insert(TreeGenerationTask { task });
    }
}

#[derive(Default)]
pub struct TreeMaterialsCache {
    pub normal_bark: Option<Handle<StandardMaterial>>,
    pub normal_leaf: Option<Handle<StandardMaterial>>,
    pub mushroom_stalk: Option<Handle<StandardMaterial>>,
    pub mushroom_cap: Option<Handle<StandardMaterial>>,
    pub alien_trunk: Option<Handle<StandardMaterial>>,
    pub alien_leaf: Option<Handle<StandardMaterial>>,
}

pub fn complete_tree_generation(
    mut commands: Commands,
    mut tasks: Query<(Entity, &mut TreeGenerationTask)>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut mat_cache: Local<TreeMaterialsCache>,
    mut mesh_cache: Local<
        rustc_hash::FxHashMap<(TreeType, bool, u32), (Handle<Mesh>, Handle<Mesh>)>,
    >,
) {
    if mat_cache.normal_bark.is_none() {
        mat_cache.normal_bark = Some(materials.add(StandardMaterial {
            base_color: Color::srgb(0.4, 0.25, 0.1),
            perceptual_roughness: 0.9,
            ..default()
        }));
        mat_cache.normal_leaf = Some(materials.add(StandardMaterial {
            base_color: Color::srgb(0.1, 0.5, 0.1),
            ..default()
        }));
        mat_cache.mushroom_stalk = Some(materials.add(StandardMaterial {
            base_color: Color::srgb(0.8, 0.7, 0.9),
            emissive: LinearRgba::from(Color::srgb(0.2, 0.1, 0.3)),
            perceptual_roughness: 0.6,
            ..default()
        }));
        mat_cache.mushroom_cap = Some(materials.add(StandardMaterial {
            base_color: Color::srgb(0.9, 0.1, 0.6),
            emissive: LinearRgba::from(Color::srgb(0.8, 0.1, 0.5)),
            perceptual_roughness: 0.5,
            ..default()
        }));
        mat_cache.alien_trunk = Some(materials.add(StandardMaterial {
            base_color: Color::srgb(0.15, 0.1, 0.25),
            emissive: LinearRgba::from(Color::srgb(0.05, 0.02, 0.1)),
            perceptual_roughness: 0.8,
            ..default()
        }));
        mat_cache.alien_leaf = Some(materials.add(StandardMaterial {
            base_color: Color::srgb(0.0, 0.8, 0.9),
            emissive: LinearRgba::from(Color::srgb(0.0, 0.7, 0.8)),
            perceptual_roughness: 0.4,
            ..default()
        }));
    }

    for (entity, mut task_comp) in tasks.iter_mut() {
        if let Some(result) = future::block_on(future::poll_once(&mut task_comp.task)) {
            let is_alien = result.pos.x >= 5000;
            let is_mushroom = matches!(result._tree_type, TreeType::Mushroom);

            let (bark_mat, leaf_mat) = if is_alien {
                if is_mushroom {
                    (
                        mat_cache.mushroom_stalk.clone().unwrap(),
                        mat_cache.mushroom_cap.clone().unwrap(),
                    )
                } else {
                    (
                        mat_cache.alien_trunk.clone().unwrap(),
                        mat_cache.alien_leaf.clone().unwrap(),
                    )
                }
            } else {
                (
                    mat_cache.normal_bark.clone().unwrap(),
                    mat_cache.normal_leaf.clone().unwrap(),
                )
            };

            let variant_key = (
                result._tree_type,
                is_alien,
                (result.pos.x.unsigned_abs() ^ result.pos.z.unsigned_abs()) % 8,
            );

            let (branch_mesh_handle, leaf_mesh_handle) = mesh_cache
                .entry(variant_key)
                .or_insert_with(|| (meshes.add(result.branch_mesh), meshes.add(result.leaf_mesh)))
                .clone();

            trace!(
                "[VEGETATION] Spawning tree at {:?} (is_alien={})",
                result.pos, is_alien
            );

            let (parent_transform, child_transform) = if is_mushroom {
                (
                    Transform::from_translation(result.pos.as_vec3() + Vec3::new(0.0, 2.25, 0.0)),
                    Transform::from_xyz(0.0, 2.25, 0.0)
                        .with_rotation(Quat::from_rotation_z(std::f32::consts::PI)),
                )
            } else {
                (
                    Transform::from_translation(result.pos.as_vec3() + Vec3::new(0.0, 1.0, 0.0)),
                    Transform::default(),
                )
            };

            // Spawn mesh canopy
            commands
                .entity(entity)
                .remove::<TreeGenerationTask>()
                .insert((
                    parent_transform,
                    Visibility::default(),
                    InheritedVisibility::default(),
                    Mesh3d(branch_mesh_handle),
                    MeshMaterial3d(bark_mat),
                    bevy_rapier3d::prelude::Collider::cylinder(3.0, 0.4),
                    Hittable,
                    Health::new(50.0), // Trees have 50 HP
                    TreeEntity,
                ))
                .with_children(|parent| {
                    parent.spawn((
                        Mesh3d(leaf_mesh_handle),
                        MeshMaterial3d(leaf_mat),
                        child_transform,
                        Visibility::default(),
                        InheritedVisibility::default(),
                    ));
                });
        }
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// Generation Logic (runs in async task)
// ─────────────────────────────────────────────────────────────────────────────

fn generate_tree_data(base_pos: IVec3, tree_type: TreeType) -> TreeGenerationResult {
    if tree_type == TreeType::Mushroom {
        let stalk_mesh = Mesh::from(Cylinder::new(0.45, 4.5));
        let cap_mesh = Mesh::from(Cone::new(3.0, 1.2));
        return TreeGenerationResult {
            pos: base_pos,
            _tree_type: tree_type,
            branch_mesh: stalk_mesh,
            leaf_mesh: cap_mesh,
        };
    }

    // 1. Shrubbery Voxel Trunk Generation
    let (
        crown_w,
        crown_h,
        crown_offset_y,
        branch_len,
        attraction_dist,
        kill_dist,
        trunk_min_h,
        grow_iters,
        branch_thickness,
    ) = match tree_type {
        TreeType::Oak => (9.0, 7.0, 6.0, 0.6, 5.5, 0.5, 3.0, 14, 0.55),
        TreeType::Pine => (4.0, 10.0, 8.0, 0.5, 4.5, 0.45, 5.0, 10, 0.4),
        TreeType::Birch => (6.0, 6.0, 5.0, 0.55, 5.0, 0.5, 2.5, 12, 0.45),
        TreeType::Jungle | TreeType::Mushroom => (12.0, 5.0, 10.0, 0.65, 6.0, 0.55, 7.0, 16, 0.5),
    };

    let seed = (base_pos.x.unsigned_abs() as u64).wrapping_mul(2654435761)
        ^ (base_pos.z.unsigned_abs() as u64).wrapping_mul(1234567891)
        ^ ((tree_type as u64) * 999983);

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
        BoxShape {
            x: crown_w,
            y: crown_h,
            z: crown_w,
        },
    );

    shrub.build_trunk();
    for _ in 0..grow_iters {
        shrub.grow();
    }
    if matches!(tree_type, TreeType::Oak | TreeType::Birch) {
        shrub.post_process_gravity(0.8);
    }

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
        TreeType::Jungle | TreeType::Mushroom => TreeMeshSettings {
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

    let mut rng = fastrand::Rng::with_seed(seed);
    let (branch_mesh, leaf_mesh) =
        bevy_procedural_tree::meshgen::generate_tree_meshes(&canopy_settings, &mut rng)
            .expect("Failed to generate tree meshes");

    TreeGenerationResult {
        pos: base_pos,
        _tree_type: tree_type,
        branch_mesh,
        leaf_mesh,
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// Helpers
// ─────────────────────────────────────────────────────────────────────────────

fn pick_grass_tree(rng: &mut FastRng) -> Option<TreeType> {
    match rng.u8(0..10) {
        0..=4 => Some(TreeType::Oak),
        5..=7 => Some(TreeType::Birch),
        8 => Some(TreeType::Jungle),
        _ => None,
    }
}

fn scatter_candidates(chunk_2d: IVec3, count: usize, rng: &mut FastRng) -> Vec<IVec2> {
    let base_x = chunk_2d.x * 32;
    let base_z = chunk_2d.z * 32;
    let cell = 32 / count.max(1) as i32;
    (0..count)
        .map(|i| {
            let cx = base_x + (i as i32) * cell + rng.i32(0..cell.max(1));
            let cz = base_z + rng.i32(0..32);
            IVec2::new(cx, cz)
        })
        .collect()
}

fn is_near_settlement_building(pos: Vec3, registry: &SettlementRegistry) -> bool {
    const BUILDING_OFFSETS: [(f32, f32); 8] = [
        (0.0, 16.0),    // Tavern
        (22.0, 6.0),    // Shop
        (-22.0, 6.0),   // Forge
        (0.0, -26.0),   // Farm
        (18.0, -18.0),  // House 1
        (-18.0, -18.0), // House 2
        (26.0, -26.0),  // Guard Tower
        (0.0, 0.0),     // Plaza / Well
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
    let mut trees_to_despawn = rustc_hash::FxHashSet::default();

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
            entity_cmd.despawn_related::<Children>();
            entity_cmd.despawn();
        }
    }
}
