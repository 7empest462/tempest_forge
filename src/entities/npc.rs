use bevy::prelude::*;
use bevy::ecs::relationship::Relationship;
use crate::world::noise_generator::NoiseGenerator;
use crate::world::manager::{find_stable_ground_height as find_ground_height};
use bevy_voxel_world::prelude::*;
use crate::world::env::TimeOfDay;
use crate::world::settlement::{BuildingPart, Solid, Bridge};
use crate::entities::{AIState, Species, Creature};
use crate::player::camera::Player;
use crate::player::combat::{Health, Hittable, DamageEvent};
use bevy_hanabi::prelude::*;
use rand::RngExt;

pub struct NPCPlugin;

impl Plugin for NPCPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Update, (
            npc_ai,
            npc_movement,
            npc_animation,
            npc_blacksmith_sparks,
        ));
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum NPCRole {
    Farmer,
    Guard,
    Citizen,
    Merchant,
    Blacksmith,
    Barkeeper,
}

#[derive(Component)]
pub struct NPC {
    pub role: NPCRole,
    pub state: AIState,
    pub target_pos: Option<Vec3>,
    pub home_pos: Vec3,
    pub work_pos: Option<Vec3>,
    pub timer: f32,
    pub last_player_look: f32,
    pub waypoints: Vec<Vec3>,      // Cached town graph waypoints
    pub path: Vec<Vec3>,           // Active step-by-step route
    pub current_path_idx: usize,
    pub attacker: Option<Entity>,  // Attacker target entity
    pub attack_cooldown: f32,      // Strike rate limiting
}

#[derive(Component)]
pub struct NPCBody;

#[derive(Component)]
pub struct NPCLeg { pub side: f32 }

#[derive(Component)]
pub struct NPCArm { pub side: f32 }

pub fn spawn_npc(
    commands: &mut Commands,
    meshes: &mut Assets<Mesh>,
    materials: &mut Assets<StandardMaterial>,
    pos: Vec3,
    role: NPCRole,
    work_pos: Option<Vec3>,
    town_waypoints: Vec<Vec3>,
) -> Entity {
    let body_mat = match role {
        NPCRole::Farmer => materials.add(StandardMaterial { base_color: Color::srgb(0.15, 0.3, 0.55), perceptual_roughness: 0.8, ..default() }), // Denim overalls
        NPCRole::Guard => materials.add(StandardMaterial { base_color: Color::srgb(0.32, 0.33, 0.36), metallic: 0.7, perceptual_roughness: 0.45, ..default() }), // Steel armor plate (dark, textured)
        NPCRole::Citizen => materials.add(StandardMaterial { base_color: Color::srgb(0.55, 0.35, 0.25), perceptual_roughness: 0.9, ..default() }), // Brown tunic
        NPCRole::Merchant => materials.add(StandardMaterial { base_color: Color::srgb(0.45, 0.1, 0.45), perceptual_roughness: 0.5, ..default() }), // Purple tunic
        NPCRole::Blacksmith => materials.add(StandardMaterial { base_color: Color::srgb(0.35, 0.2, 0.12), perceptual_roughness: 0.9, ..default() }), // Leather brown
        NPCRole::Barkeeper => materials.add(StandardMaterial { base_color: Color::srgb(0.65, 0.5, 0.15), perceptual_roughness: 0.8, ..default() }), // Golden vest
    };

    commands.spawn((
        NPC {
            role,
            state: AIState::Wandering,
            target_pos: None,
            home_pos: pos,
            work_pos,
            timer: 0.0,
            last_player_look: 0.0,
            waypoints: town_waypoints,
            path: Vec::new(),
            current_path_idx: 0,
            attacker: None,
            attack_cooldown: 0.0,
        },
        crate::world::water::WaterInteractor {
            mass: 3.375, // NPCs scale is 1.5 (1.5^3)
            ..default()
        },
        Health::new(25.0), // A bit tougher!
        Hittable,
        Transform::from_translation(pos).with_scale(Vec3::splat(1.5)),
        Visibility::default(),
        InheritedVisibility::default(),
    )).with_children(|npc_parent| {
            // Torso / Body
            npc_parent.spawn((
                Mesh3d(meshes.add(Cuboid::new(0.4, 0.8, 0.3))),
                MeshMaterial3d(body_mat.clone()),
                Transform::from_xyz(0.0, 0.9, 0.0),
                NPCBody,
            ));
            
            // Blacksmith Leather Apron Front Flap
            if role == NPCRole::Blacksmith {
                npc_parent.spawn((
                    Mesh3d(meshes.add(Cuboid::new(0.34, 0.6, 0.02))),
                    MeshMaterial3d(materials.add(StandardMaterial { base_color: Color::srgb(0.2, 0.1, 0.05), perceptual_roughness: 0.95, ..default() })),
                    Transform::from_xyz(0.0, 0.78, -0.152),
                ));
            }

            // Guard Chest Buckle
            if role == NPCRole::Guard {
                npc_parent.spawn((
                    Mesh3d(meshes.add(Cuboid::new(0.12, 0.12, 0.02))),
                    MeshMaterial3d(materials.add(StandardMaterial { base_color: Color::srgb(1.0, 0.8, 0.0), metallic: 0.9, perceptual_roughness: 0.1, ..default() })),
                    Transform::from_xyz(0.0, 1.0, -0.152),
                ));
            }
            
            // Head with facial details
            npc_parent.spawn((
                Mesh3d(meshes.add(Cuboid::new(0.35, 0.35, 0.35))),
                MeshMaterial3d(materials.add(StandardMaterial { base_color: Color::srgb(1.0, 0.8, 0.7), perceptual_roughness: 0.9, ..default() })),
                Transform::from_xyz(0.0, 1.45, 0.0),
            )).with_children(|head| {
                // White backplates of eyes
                for eye_side in [-1.0, 1.0] {
                    head.spawn((
                        Mesh3d(meshes.add(Cuboid::new(0.06, 0.06, 0.02))),
                        MeshMaterial3d(materials.add(StandardMaterial { base_color: Color::WHITE, perceptual_roughness: 0.9, ..default() })),
                        Transform::from_xyz(eye_side * 0.08, 0.06, -0.176),
                    ));
                    // Black pupils
                    head.spawn((
                        Mesh3d(meshes.add(Cuboid::new(0.03, 0.03, 0.01))),
                        MeshMaterial3d(materials.add(StandardMaterial { base_color: Color::BLACK, perceptual_roughness: 0.9, ..default() })),
                        Transform::from_xyz(eye_side * 0.08, 0.06, -0.187),
                    ));
                }

                // Nose block
                head.spawn((
                    Mesh3d(meshes.add(Cuboid::new(0.04, 0.06, 0.04))),
                    MeshMaterial3d(materials.add(StandardMaterial { base_color: Color::srgb(0.95, 0.75, 0.65), perceptual_roughness: 0.9, ..default() })),
                    Transform::from_xyz(0.0, -0.01, -0.185),
                ));

                // Smiley mouth! Happy all the time!
                let mouth_mat = materials.add(StandardMaterial {
                    base_color: Color::srgb(0.9, 0.2, 0.2), // Cute rosy smile
                    perceptual_roughness: 0.9,
                    ..default()
                });
                // Center block
                head.spawn((
                    Mesh3d(meshes.add(Cuboid::new(0.08, 0.025, 0.015))),
                    MeshMaterial3d(mouth_mat.clone()),
                    Transform::from_xyz(0.0, -0.09, -0.178),
                ));
                // Left end block (upturned smile corner)
                head.spawn((
                    Mesh3d(meshes.add(Cuboid::new(0.025, 0.025, 0.015))),
                    MeshMaterial3d(mouth_mat.clone()),
                    Transform::from_xyz(-0.045, -0.075, -0.178),
                ));
                // Right end block (upturned smile corner)
                head.spawn((
                    Mesh3d(meshes.add(Cuboid::new(0.025, 0.025, 0.015))),
                    MeshMaterial3d(mouth_mat.clone()),
                    Transform::from_xyz(0.045, -0.075, -0.178),
                ));

                // Hats and hairstyles
                match role {
                    NPCRole::Farmer => {
                        // Wide straw hat brim
                        head.spawn((
                            Mesh3d(meshes.add(Cuboid::new(0.55, 0.03, 0.55))),
                            MeshMaterial3d(materials.add(StandardMaterial { base_color: Color::srgb(0.85, 0.75, 0.35), perceptual_roughness: 0.95, ..default() })),
                            Transform::from_xyz(0.0, 0.18, 0.0),
                        ));
                        // Straw hat crown
                        head.spawn((
                            Mesh3d(meshes.add(Cuboid::new(0.28, 0.1, 0.28))),
                            MeshMaterial3d(materials.add(StandardMaterial { base_color: Color::srgb(0.8, 0.7, 0.3), perceptual_roughness: 0.95, ..default() })),
                            Transform::from_xyz(0.0, 0.23, 0.0),
                        ));
                    }
                    NPCRole::Guard => {
                        // Beautiful open-face helmet composed of 3 parts (top cap, back guard, side cheek guards)
                        // This leaves the front face open, exposing the happy eyes and rosy smiley mouth!
                        let helmet_mat = materials.add(StandardMaterial {
                            base_color: Color::srgb(0.35, 0.36, 0.4), // Matches plate armor
                            metallic: 0.7,
                            perceptual_roughness: 0.45,
                            ..default()
                        });
                        // Top Helmet Cap
                        head.spawn((
                            Mesh3d(meshes.add(Cuboid::new(0.38, 0.12, 0.38))),
                            MeshMaterial3d(helmet_mat.clone()),
                            Transform::from_xyz(0.0, 0.15, 0.0),
                        ));
                        // Neck Guard (Back)
                        head.spawn((
                            Mesh3d(meshes.add(Cuboid::new(0.38, 0.22, 0.04))),
                            MeshMaterial3d(helmet_mat.clone()),
                            Transform::from_xyz(0.0, 0.0, 0.17),
                        ));
                        // Cheek Guards (Left/Right)
                        for helmet_side in [-1.0, 1.0] {
                            head.spawn((
                                Mesh3d(meshes.add(Cuboid::new(0.04, 0.22, 0.34))),
                                MeshMaterial3d(helmet_mat.clone()),
                                Transform::from_xyz(helmet_side * 0.17, 0.0, 0.02),
                            ));
                        }
                        // Red crest plume
                        head.spawn((
                            Mesh3d(meshes.add(Cuboid::new(0.06, 0.14, 0.34))),
                            MeshMaterial3d(materials.add(StandardMaterial { base_color: Color::srgb(0.8, 0.1, 0.1), ..default() })),
                            Transform::from_xyz(0.0, 0.25, 0.02),
                        ));
                    }
                    NPCRole::Merchant => {
                        // Velvet merchant cap
                        head.spawn((
                            Mesh3d(meshes.add(Cuboid::new(0.38, 0.15, 0.38))),
                            MeshMaterial3d(materials.add(StandardMaterial { base_color: Color::srgb(0.45, 0.1, 0.45), perceptual_roughness: 0.8, ..default() })),
                            Transform::from_xyz(0.0, 0.16, 0.0),
                        ));
                        // Gold emblem accent
                        head.spawn((
                            Mesh3d(meshes.add(Cuboid::new(0.06, 0.08, 0.06))),
                            MeshMaterial3d(materials.add(StandardMaterial { base_color: Color::srgb(1.0, 0.8, 0.0), metallic: 0.9, ..default() })),
                            Transform::from_xyz(0.18, 0.16, -0.05),
                        ));
                    }
                    NPCRole::Citizen | NPCRole::Barkeeper | NPCRole::Blacksmith => {
                        let hair_color = if role == NPCRole::Blacksmith {
                            Color::srgb(0.15, 0.12, 0.1) // Dark
                        } else {
                            Color::srgb(0.5, 0.35, 0.15) // Brown
                        };
                        // Top Hair Cap (keeps hair above eye lines, depth reduced to 0.30 and moved back to avoid clipping eyes/forehead)
                        head.spawn((
                            Mesh3d(meshes.add(Cuboid::new(0.37, 0.12, 0.30))),
                            MeshMaterial3d(materials.add(StandardMaterial { base_color: hair_color, perceptual_roughness: 0.9, ..default() })),
                            Transform::from_xyz(0.0, 0.13, 0.04),
                        ));
                        // Back Hair Block (extends down the back of the head, away from the face)
                        head.spawn((
                            Mesh3d(meshes.add(Cuboid::new(0.37, 0.22, 0.12))),
                            MeshMaterial3d(materials.add(StandardMaterial { base_color: hair_color, ..default() })),
                            Transform::from_xyz(0.0, 0.04, 0.13),
                        ));
                        // Sideburns
                        for hair_side in [-1.0, 1.0] {
                            head.spawn((
                                Mesh3d(meshes.add(Cuboid::new(0.03, 0.12, 0.15))),
                                MeshMaterial3d(materials.add(StandardMaterial { base_color: hair_color, ..default() })),
                                Transform::from_xyz(hair_side * 0.176, -0.05, -0.04),
                            ));
                        }
                    }
                }
            });

            // Legs (Grey pants)
            let pants_mat = materials.add(StandardMaterial { base_color: Color::srgb(0.2, 0.2, 0.22), perceptual_roughness: 0.9, ..default() });
            for side in [-1.0, 1.0] {
                npc_parent.spawn((
                    NPCLeg { side },
                    Mesh3d(meshes.add(Cuboid::new(0.18, 0.6, 0.18))),
                    MeshMaterial3d(pants_mat.clone()),
                    Transform::from_xyz(side * 0.12, 0.3, 0.0),
                ));
            }

            // Arms (With hand tools parented to the right arm)
            for side in [-1.0, 1.0] {
                let mut arm_cmd = npc_parent.spawn((
                    NPCArm { side },
                    Mesh3d(meshes.add(Cuboid::new(0.15, 0.6, 0.15))),
                    MeshMaterial3d(body_mat.clone()),
                    Transform::from_xyz(side * 0.3, 1.0, 0.0),
                ));

                // Parent tool to right arm (side > 0.0)
                if side > 0.0 {
                    match role {
                        NPCRole::Guard => {
                            arm_cmd.with_children(|arm| {
                                // Steel sword hilt
                                arm.spawn((
                                    Mesh3d(meshes.add(Cuboid::new(0.05, 0.2, 0.05))),
                                    MeshMaterial3d(materials.add(StandardMaterial { base_color: Color::srgb(0.3, 0.2, 0.1), ..default() })),
                                    Transform::from_xyz(0.0, -0.35, -0.05).with_rotation(Quat::from_rotation_x(1.2)),
                                )).with_children(|hilt| {
                                    // Crossguard
                                    hilt.spawn((
                                        Mesh3d(meshes.add(Cuboid::new(0.2, 0.04, 0.05))),
                                        MeshMaterial3d(materials.add(StandardMaterial { base_color: Color::srgb(0.75, 0.75, 0.8), metallic: 0.8, perceptual_roughness: 0.2, ..default() })),
                                        Transform::from_xyz(0.0, 0.1, 0.0),
                                    ));
                                    // Silver Blade
                                    hilt.spawn((
                                        Mesh3d(meshes.add(Cuboid::new(0.04, 0.7, 0.02))),
                                        MeshMaterial3d(materials.add(StandardMaterial { base_color: Color::srgb(0.85, 0.85, 0.9), metallic: 0.95, perceptual_roughness: 0.1, ..default() })),
                                        Transform::from_xyz(0.0, 0.45, 0.0),
                                    ));
                                });
                            });
                        }
                        NPCRole::Farmer => {
                            arm_cmd.with_children(|arm| {
                                // Hoe handle
                                arm.spawn((
                                    Mesh3d(meshes.add(Cuboid::new(0.03, 0.7, 0.03))),
                                    MeshMaterial3d(materials.add(StandardMaterial { base_color: Color::srgb(0.4, 0.25, 0.12), ..default() })),
                                    Transform::from_xyz(0.0, -0.3, -0.1).with_rotation(Quat::from_rotation_x(1.2)),
                                )).with_children(|handle| {
                                    // Metal blade
                                    handle.spawn((
                                        Mesh3d(meshes.add(Cuboid::new(0.12, 0.03, 0.1))),
                                        MeshMaterial3d(materials.add(StandardMaterial { base_color: Color::srgb(0.45, 0.45, 0.45), metallic: 0.7, ..default() })),
                                        Transform::from_xyz(0.0, 0.35, -0.04),
                                    ));
                                });
                            });
                        }
                        NPCRole::Blacksmith => {
                            arm_cmd.with_children(|arm| {
                                // Heavy smithing hammer handle
                                arm.spawn((
                                    Mesh3d(meshes.add(Cuboid::new(0.04, 0.45, 0.04))),
                                    MeshMaterial3d(materials.add(StandardMaterial { base_color: Color::srgb(0.3, 0.2, 0.1), ..default() })),
                                    Transform::from_xyz(0.0, -0.3, -0.05).with_rotation(Quat::from_rotation_x(1.0)),
                                )).with_children(|handle| {
                                    // Heavy metal head
                                    handle.spawn((
                                        Mesh3d(meshes.add(Cuboid::new(0.14, 0.16, 0.22))),
                                        MeshMaterial3d(materials.add(StandardMaterial { base_color: Color::srgb(0.22, 0.22, 0.26), metallic: 0.9, perceptual_roughness: 0.4, ..default() })),
                                        Transform::from_xyz(0.0, 0.225, 0.0),
                                    ));
                                });
                            });
                        }
                        _ => {}
                    }
                }
            }
        }).id()
}

/// Helper function to compute custom hub-and-spoke paths along spawned roads
fn compute_path(start: Vec3, end: Vec3, waypoints: &[Vec3]) -> Vec<Vec3> {
    if waypoints.is_empty() || start.distance(end) < 8.0 {
        return vec![end];
    }

    // Find closest waypoint to start
    let mut closest_start_idx = 0;
    let mut min_start_dist = f32::MAX;
    for (i, &wp) in waypoints.iter().enumerate() {
        let d = start.distance(wp);
        if d < min_start_dist {
            min_start_dist = d;
            closest_start_idx = i;
        }
    }

    // Find closest waypoint to end
    let mut closest_end_idx = 0;
    let mut min_end_dist = f32::MAX;
    for (i, &wp) in waypoints.iter().enumerate() {
        let d = end.distance(wp);
        if d < min_end_dist {
            min_end_dist = d;
            closest_end_idx = i;
        }
    }

    let mut path = Vec::new();
    
    // 1. Move to start waypoint
    path.push(waypoints[closest_start_idx]);

    // 2. Route through plaza center (Node 0)
    if closest_start_idx != 0 && closest_end_idx != closest_start_idx {
        path.push(waypoints[0]);
    }

    // 3. Move to end waypoint
    if closest_end_idx != 0 && closest_end_idx != closest_start_idx {
        path.push(waypoints[closest_end_idx]);
    }

    // 4. Move to final target
    path.push(end);

    // Clean up intermediate path coordinates
    let mut clean_path = Vec::new();
    let mut last_pos = start;
    for &pos in &path {
        if pos.distance(last_pos) > 0.5 {
            clean_path.push(pos);
            last_pos = pos;
        }
    }

    if clean_path.is_empty() {
        clean_path.push(end);
    }
    
    clean_path
}

/// Ray-AABB intersection helper using Slab method
fn ray_aabb_intersection(ray_origin: Vec3, ray_dir: Vec3, box_min: Vec3, box_max: Vec3) -> Option<f32> {
    let mut t_min = f32::NEG_INFINITY;
    let mut t_max = f32::INFINITY;

    for i in 0..3 {
        let origin = ray_origin[i];
        let dir = ray_dir[i];
        let b_min = box_min[i];
        let b_max = box_max[i];

        if dir.abs() < 1e-6 {
            if origin < b_min || origin > b_max {
                return None;
            }
        } else {
            let mut t1 = (b_min - origin) / dir;
            let mut t2 = (b_max - origin) / dir;
            if t1 > t2 {
                std::mem::swap(&mut t1, &mut t2);
            }
            t_min = t_min.max(t1);
            t_max = t_max.min(t2);
            if t_min > t_max {
                return None;
            }
        }
    }

    if t_max < 0.0 {
        return None;
    }

    Some(t_min.max(0.0))
}

/// Helper function to perform line-of-sight visual checks using voxels and building collision parts
fn has_line_of_sight(
    eye_pos: Vec3,
    target_pos: Vec3,
    max_dist: f32,
    forward: Dir3,
    fov_dot: f32,
    voxel_world: &VoxelWorld<NoiseGenerator>,
    solids: &[(&GlobalTransform, &BuildingPart)],
) -> bool {
    let dist = eye_pos.distance(target_pos);
    if dist > max_dist {
        return false;
    }

    let dir = (target_pos - eye_pos).normalize_or_zero();
    if dir == Vec3::ZERO {
        return true;
    }

    // Check FOV cone (direction)
    let forward_vec = Vec3::from(forward);
    if forward_vec.dot(dir) < fov_dot {
        return false;
    }

    // Check voxel occlusion
    if let Ok(dir_3) = Dir3::new(dir) {
        let ray = Ray3d::new(eye_pos, dir_3);
        if let Some(hit) = voxel_world.raycast(ray, &|(_, v): (Vec3, WorldVoxel)| v.is_solid()) {
            let hit_dist = eye_pos.distance(hit.position);
            if hit_dist < dist {
                return false; // Occluded by voxels
            }
        }
    }

    // Check building parts occlusion
    for (s_trans, s_part) in solids.iter() {
        let center = s_trans.translation();
        let half_size = s_part.0;
        let b_min = center - half_size;
        let b_max = center + half_size;

        if let Some(t) = ray_aabb_intersection(eye_pos, dir, b_min, b_max) {
            if t < dist {
                return false; // Occluded by a building wall/roof
            }
        }
    }

    true
}

fn npc_ai(
    time: Res<Time>,
    time_of_day: Res<TimeOfDay>,
    mut commands: Commands,
    mut npc_query: Query<(Entity, &mut NPC, &Transform, Option<&DamageEvent>)>,
    creature_query: Query<(Entity, &Transform, &Creature), With<Health>>,
    spark_effect_res: Option<Res<crate::particle_effects::BlacksmithSparkEffect>>,
    player_query: Query<(Entity, &Transform), With<Player>>,
    voxel_world: VoxelWorld<NoiseGenerator>,
    solid_query: Query<(&GlobalTransform, &BuildingPart), With<Solid>>,
) {
    let mut rng = rand::rng();
    let hour = time_of_day.hour;
    let solids: Vec<_> = solid_query.iter().collect();

    // Get player entity and position
    let (player_entity, player_pos) = match player_query.iter().next() {
        Some((e, t)) => (e, t.translation),
        None => (Entity::PLACEHOLDER, Vec3::ZERO),
    };

    // Pre-collect active npc combat states to avoid borrow conflict
    let active_npc_combats: Vec<(Entity, Vec3, Option<Entity>)> = npc_query.iter()
        .map(|(e, npc, t, _)| (e, t.translation, npc.attacker))
        .collect();

    for (entity, mut npc, transform, damage_event) in npc_query.iter_mut() {
        npc.timer -= time.delta_secs();
        npc.last_player_look -= time.delta_secs();

        // --- RETALIATIVE COMBAT / DEFENSIVE AI ---
        // If attacked (i.e. has a DamageEvent)
        if damage_event.is_some() {
            // First check if there is a hostile monster nearby who could be the attacker
            let mut closest_monster = None;
            let mut closest_dist = 4.0; // Melee range for monsters
            for (c_entity, c_trans, c_creature) in creature_query.iter() {
                if matches!(c_creature.species, Species::Wolf | Species::Spider | Species::Skeleton) {
                    let d = transform.translation.distance(c_trans.translation);
                    if d < closest_dist {
                        closest_dist = d;
                        closest_monster = Some(c_entity);
                    }
                }
            }

            if let Some(monster) = closest_monster {
                npc.attacker = Some(monster);
                npc.state = AIState::Chasing;
                npc.timer = 12.0; // Stay in combat mode
                npc.path.clear();
            } else if player_entity != Entity::PLACEHOLDER {
                // If no monster is nearby, it must be the Player!
                npc.attacker = Some(player_entity);
                npc.state = AIState::Chasing;
                npc.timer = 30.0; // 30 seconds of player anger cooldown!
                npc.path.clear();
                println!("{:?} NPC got mad at the Player for attacking them!", npc.role);
            }
        }

        // Alert fellow townspeople if B is not already mad at someone
        if npc.attacker.is_none() {
            let mut found_fight = false;
            for &(a_entity, a_pos, a_attacker) in &active_npc_combats {
                if a_entity == entity {
                    continue; // Skip self
                }
                // If another citizen is fighting the player
                if a_attacker == Some(player_entity) {
                    let b_pos = transform.translation;
                    let forward_dir = transform.forward();

                    let b_eye = b_pos + Vec3::new(0.0, 1.45, 0.0);
                    let a_center = a_pos + Vec3::new(0.0, 0.9, 0.0);
                    let player_center = player_pos + Vec3::new(0.0, 1.0, 0.0);

                    // Check visual sight with full building and terrain occlusion
                    let can_see_a = has_line_of_sight(b_eye, a_center, 25.0, forward_dir, 0.35, &voxel_world, &solids);
                    let can_see_player = has_line_of_sight(b_eye, player_center, 25.0, forward_dir, 0.35, &voxel_world, &solids);

                    if can_see_a || can_see_player {
                        npc.attacker = Some(player_entity);
                        npc.state = AIState::Chasing;
                        npc.timer = 30.0; // Alert cooldown
                        npc.path.clear();
                        println!("{:?} NPC saw fellow townsperson in distress and joined the fight against the Player!", npc.role);
                        found_fight = true;
                        break;
                    }
                }
            }
            
            // Guards also actively hunt down monsters within 16.0 meters
            if !found_fight && npc.role == NPCRole::Guard {
                let mut closest_monster = None;
                let mut closest_dist = 16.0;
                for (c_entity, c_trans, c_creature) in creature_query.iter() {
                    if matches!(c_creature.species, Species::Wolf | Species::Spider | Species::Skeleton) {
                        let d = transform.translation.distance(c_trans.translation);
                        if d < closest_dist {
                            closest_dist = d;
                            closest_monster = Some(c_entity);
                        }
                    }
                }
                if let Some(monster) = closest_monster {
                    npc.attacker = Some(monster);
                    npc.state = AIState::Chasing;
                    npc.timer = 8.0;
                    npc.path.clear();
                }
            }
        }

        // Process Combat state if active
        if npc.state == AIState::Chasing {
            if let Some(attacker_entity) = npc.attacker {
                let mut found = false;

                // 1. Check if the attacker is the Player
                if attacker_entity == player_entity {
                    found = true;
                    let target_pos = player_pos;
                    npc.target_pos = Some(target_pos);

                    let dist = transform.translation.distance(target_pos);
                    if dist < 2.0 {
                        npc.attack_cooldown -= time.delta_secs();
                        if npc.attack_cooldown <= 0.0 {
                            let damage = match npc.role {
                                NPCRole::Guard => 10.0,      // Guards with steel sword hit hard
                                NPCRole::Blacksmith => 6.0,  // Hammer
                                _ => 4.0,                    // Fists/Hoe
                            };
                            commands.entity(player_entity).insert(DamageEvent(damage));
                            npc.attack_cooldown = 1.0; // Attack speed limit

                            println!("{:?} NPC struck the Player!", npc.role);

                            // Visual sparks effect if blacksmith strikes player
                            if npc.role == NPCRole::Blacksmith {
                                if let Some(ref sparks) = spark_effect_res {
                                    commands.spawn((
                                        ParticleEffect {
                                            handle: sparks.0.clone(),
                                            ..default()
                                        },
                                        Transform::from_translation(player_pos + Vec3::new(0.0, 0.4, 0.0)),
                                    ));
                                }
                            }
                        }
                    }

                    // Cooldown: check if anger timer runs out
                    if npc.timer <= 0.0 {
                        npc.attacker = None;
                        npc.state = AIState::Wandering;
                        npc.path.clear();
                        println!("{:?} NPC calmed down and forgot about the Player.", npc.role);
                    }
                } else {
                    // 2. Check if the attacker is a monster
                    for (c_entity, c_trans, _) in creature_query.iter() {
                        if c_entity == attacker_entity {
                            found = true;
                            let monster_pos = c_trans.translation;
                            npc.target_pos = Some(monster_pos);

                            let dist_to_monster = transform.translation.distance(monster_pos);
                            if dist_to_monster < 2.0 {
                                npc.attack_cooldown -= time.delta_secs();
                                if npc.attack_cooldown <= 0.0 {
                                    let damage = match npc.role {
                                        NPCRole::Guard => 10.0,      // Guards with steel sword hit hard
                                        NPCRole::Blacksmith => 6.0,  // Hammer
                                        _ => 4.0,                    // Fists/Hoe
                                    };
                                    commands.entity(attacker_entity).insert(DamageEvent(damage));
                                    npc.attack_cooldown = 1.0; // Attack speed limit

                                    println!("{:?} NPC defended themselves and struck creature!", npc.role);

                                    // Visual sparks effect if blacksmith strikes monster
                                    if npc.role == NPCRole::Blacksmith {
                                        if let Some(ref sparks) = spark_effect_res {
                                            commands.spawn((
                                                ParticleEffect {
                                                    handle: sparks.0.clone(),
                                                    ..default()
                                                },
                                                Transform::from_translation(monster_pos + Vec3::new(0.0, 0.4, 0.0)),
                                            ));
                                        }
                                    }
                                }
                            }
                            break;
                        }
                    }
                }

                if !found {
                    // Target is dead or despawned
                    npc.attacker = None;
                    npc.state = AIState::Wandering;
                    npc.timer = 0.0;
                    npc.path.clear();
                }
            } else {
                npc.state = AIState::Wandering;
                npc.timer = 0.0;
                npc.path.clear();
            }
            continue;
        }

        // --- DYNAMIC AI DAILY ROUTINE SCHEDULES ---
        
        // 1. SLEEP TIME: 21.0 to 6.0
        if hour >= 21.0 || hour < 6.0 {
            let dist_to_home = transform.translation.distance(npc.home_pos);
            if dist_to_home < 1.2 {
                npc.state = AIState::Sleeping;
                npc.target_pos = None;
                npc.path.clear();
            } else {
                npc.state = AIState::Wandering;
                npc.target_pos = Some(npc.home_pos);
            }
            continue;
        }

        if npc.state == AIState::Sleeping {
            npc.state = AIState::Wandering;
            npc.timer = 0.0;
        }

        // 2. TAVERN SOCIAL HOUR: 17.0 to 21.0
        if hour >= 17.0 && hour < 21.0 {
            let plaza_center = npc.waypoints.first().copied().unwrap_or(npc.home_pos);
            let tavern_pos = npc.waypoints.get(1).copied().unwrap_or(plaza_center);
            
            let dist_to_tavern = transform.translation.distance(tavern_pos);
            if dist_to_tavern < 6.0 {
                npc.state = AIState::Sitting;
                npc.target_pos = None;
                npc.path.clear();
            } else {
                npc.state = AIState::Wandering;
                npc.target_pos = Some(tavern_pos);
            }
            continue;
        }

        if npc.state == AIState::Sitting {
            npc.state = AIState::Wandering;
            npc.timer = 0.0;
        }

        // 3. MORNING PLAZA COMMUTE: 6.0 to 8.0
        if hour >= 6.0 && hour < 8.0 {
            let plaza_center = npc.waypoints.first().copied().unwrap_or(npc.home_pos);
            if npc.timer <= 0.0 {
                npc.state = AIState::Wandering;
                let offset = Vec3::new(rng.random_range(-4.0..4.0), 0.0, rng.random_range(-4.0..4.0));
                npc.target_pos = Some(plaza_center + offset);
                npc.timer = rng.random_range(4.0..8.0);
            }
            continue;
        }

        // 4. DAY WORK HOUR: 8.0 to 17.0
        if hour >= 8.0 && hour < 17.0 {
            if npc.timer <= 0.0 {
                npc.state = AIState::Wandering;
                match npc.role {
                    NPCRole::Farmer => {
                        // Farmers work near their garden field (offset +5.5 from farm house)
                        let offset = Vec3::new(rng.random_range(3.5..7.5), 0.0, rng.random_range(-2.0..2.0));
                        npc.target_pos = Some(npc.home_pos + offset);
                        npc.timer = rng.random_range(6.0..12.0);
                    }
                    NPCRole::Blacksmith | NPCRole::Merchant | NPCRole::Barkeeper => {
                        if let Some(work) = npc.work_pos {
                            let offset = if npc.role == NPCRole::Blacksmith {
                                Vec3::new(0.5, 0.0, 0.5) // Stands next to anvil
                            } else {
                                Vec3::new(rng.random_range(-1.0..1.0), 0.0, rng.random_range(-1.0..1.0))
                            };
                            npc.target_pos = Some(work + offset);
                            npc.timer = rng.random_range(8.0..15.0);
                        } else {
                            npc.target_pos = Some(npc.home_pos);
                            npc.timer = 5.0;
                        }
                    }
                    NPCRole::Guard => {
                        // Guards patrol outposts (nodes 0, 7, 8, 9)
                        if !npc.waypoints.is_empty() {
                            let idx = rng.random_range(0..npc.waypoints.len());
                            npc.target_pos = Some(npc.waypoints[idx]);
                        } else {
                            npc.target_pos = Some(npc.home_pos);
                        }
                        npc.timer = rng.random_range(10.0..20.0);
                    }
                    NPCRole::Citizen => {
                        // Citizens browse shop or visit plaza
                        if npc.waypoints.len() > 2 {
                            let idx = rng.random_range(2..npc.waypoints.len());
                            npc.target_pos = Some(npc.waypoints[idx]);
                        } else {
                            npc.target_pos = Some(npc.home_pos);
                        }
                        npc.timer = rng.random_range(8.0..15.0);
                    }
                }
            }
        }
    }
}

fn npc_movement(
    time: Res<Time>,
    voxel_world: VoxelWorld<NoiseGenerator>,
    player_query: Query<&Transform, (With<Player>, Without<NPC>)>,
    solid_query: Query<(&GlobalTransform, &BuildingPart), With<Solid>>,
    bridge_query: Query<&Bridge>,
    mut query: Query<(Entity, &mut Transform, &mut NPC), Without<Player>>,
) {
    let solids: Vec<_> = solid_query.iter().collect();

    for (entity, mut transform, mut npc) in query.iter_mut() {
        if npc.state == AIState::Sleeping {
            // Sleep snapped to bed in home
            transform.translation = npc.home_pos + Vec3::new(0.0, 0.5, 0.0);
            
            // Lie flat rotation
            let sleep_quat = Quat::from_rotation_x(std::f32::consts::FRAC_PI_2);
            transform.rotation = transform.rotation.lerp(sleep_quat, 0.1);
            continue;
        }

        if npc.state == AIState::Sitting {
            // Sit snapped on Tavern Bench seats
            let seat_idx = (entity.to_bits() % 4) as f32;
            let side = if seat_idx < 2.0 { -1.0 } else { 1.0 };
            let offset_z = if seat_idx % 2.0 == 0.0 { -0.95 } else { 0.95 };
            let plaza_center = npc.waypoints.first().copied().unwrap_or(npc.home_pos);
            let tavern_pos = npc.waypoints.get(1).copied().unwrap_or(plaza_center);
            
            let seat_pos = tavern_pos + Vec3::new(side * 2.2, 0.45, -1.2 + offset_z);
            transform.translation = seat_pos;

            // Turn to face the tavern table center
            let table_pos = tavern_pos + Vec3::new(side * 2.2, 0.45, -1.2);
            let dir_to_table = (table_pos - seat_pos).normalize_or_zero();
            if dir_to_table != Vec3::ZERO {
                let target_quat = Quat::from_rotation_y(dir_to_table.x.atan2(dir_to_table.z) + std::f32::consts::PI);
                transform.rotation = transform.rotation.lerp(target_quat, 0.1);
            }
            continue;
        }

        // Normal movement along star-graph waypoints
        if let Some(target) = npc.target_pos {
            // Recompute waymarked path if needed
            if npc.path.is_empty() || npc.path.last().unwrap().distance(target) > 1.0 {
                npc.path = compute_path(transform.translation, target, &npc.waypoints);
                npc.current_path_idx = 0;
            }

            if npc.current_path_idx < npc.path.len() {
                let next_wp = npc.path[npc.current_path_idx];
                let dist_to_wp = transform.translation.distance(next_wp);
                
                if dist_to_wp < 0.8 {
                    npc.current_path_idx += 1;
                } else {
                    let direction = (next_wp - transform.translation).normalize_or_zero();
                    let speed = match npc.role {
                        NPCRole::Guard => 2.6,
                        _ if npc.state == AIState::Chasing => 3.2, // Run fast during combat
                        _ => 1.5,
                    };
                    let next_pos = transform.translation + direction * speed * time.delta_secs();

                    // Perform obstacle collision check against spawned building parts
                    let mut can_move = true;
                    for (s_trans, s_part) in solids.iter() {
                        let (scale, _, center) = s_trans.to_scale_rotation_translation();
                        let half_size = s_part.0 * scale;

                        let p_min = next_pos + Vec3::new(-0.35, 0.0, -0.35);
                        let p_max = next_pos + Vec3::new(0.35, 2.7, 0.35);
                        let b_min = center - half_size;
                        let b_max = center + half_size;

                        if p_min.x < b_max.x && p_max.x > b_min.x &&
                           p_min.y < b_max.y && p_max.y > b_min.y &&
                           p_min.z < b_max.z && p_max.z > b_min.z {
                            can_move = false;
                            break;
                        }
                    }

                    if can_move {
                        transform.translation = next_pos;
                    } else {
                        // If path blocked, recalculate path to steer around obstacles
                        npc.path.clear();
                    }

                    if direction.length_squared() > 0.01 {
                        let target_quat = Quat::from_rotation_y(direction.x.atan2(direction.z) + std::f32::consts::PI);
                        transform.rotation = transform.rotation.lerp(target_quat, 0.1);
                    }
                }
            }
        }

        // Social awareness: look at player if close
        let player_pos = player_query.iter().next().map(|t| t.translation).unwrap_or(Vec3::ZERO);
        if transform.translation.distance(player_pos) < 5.0 && npc.state != AIState::Chasing {
            let dir = (player_pos - transform.translation).normalize_or_zero();
            if dir != Vec3::ZERO {
                let target_quat = Quat::from_rotation_y(dir.x.atan2(dir.z) + std::f32::consts::PI);
                transform.rotation = transform.rotation.lerp(target_quat, 0.1);
            }
        }

        // Terrain or Bridge height clamping
        let mut final_y = None;
        let pos_2d = Vec3::new(transform.translation.x, 0.0, transform.translation.z);
        let npc_y = transform.translation.y;
        
        for bridge in bridge_query.iter() {
            let start_2d = Vec3::new(bridge.start.x, 0.0, bridge.start.z);
            let end_2d = Vec3::new(bridge.end.x, 0.0, bridge.end.z);
            let v = end_2d - start_2d;
            let len_sq = v.length_squared();
            if len_sq > 0.01 {
                let w = pos_2d - start_2d;
                let t = (w.dot(v) / len_sq).clamp(0.0, 1.0);
                let closest_2d = start_2d + v * t;
                let dist_2d = pos_2d.distance(closest_2d);
                
                // If within bridge width horizontal margin and vertically within 1.5m
                if dist_2d < 1.5 {
                    let interp_y = bridge.start.y + (bridge.end.y - bridge.start.y) * t;
                    if (npc_y - interp_y).abs() < 1.5 {
                        final_y = Some(interp_y);
                        break;
                    }
                }
            }
        }

        let ground_height = final_y.or_else(|| find_ground_height(transform.translation, &voxel_world));
        if let Some(gh) = ground_height {
            transform.translation.y = gh;
        }
    }
}

fn npc_animation(
    time: Res<Time>,
    mut query: Query<(&mut Transform, Option<&NPCLeg>, Option<&NPCArm>, &ChildOf), Without<NPC>>,
    npc_query: Query<&NPC>,
) {
    let t = time.elapsed_secs();
    
    for (mut transform, leg, arm, child_of) in query.iter_mut() {
        if let Ok(npc) = npc_query.get(Relationship::get(child_of)) {
            let moving = npc.target_pos.is_some() && npc.state != AIState::Sleeping && npc.state != AIState::Sitting;

            if npc.state == AIState::Sleeping {
                transform.rotation = Quat::IDENTITY;
                continue;
            }

            if npc.state == AIState::Sitting {
                if leg.is_some() {
                    // Sit legs bent forward
                    transform.rotation = Quat::from_rotation_x(std::f32::consts::FRAC_PI_2);
                } else if let Some(arm) = arm {
                    // Arms resting forward on table
                    let side_mult = if arm.side > 0.0 { -1.0 } else { 1.0 };
                    transform.rotation = Quat::from_rotation_x(0.8) * Quat::from_rotation_y(side_mult * 0.3);
                }
                continue;
            }

            if let Some(leg) = leg {
                if moving {
                    let speed = if npc.state == AIState::Chasing { 16.0 } else { 10.0 };
                    let offset = if leg.side > 0.0 { 0.0 } else { std::f32::consts::PI };
                    let angle = (t * speed + offset).sin() * 0.4;
                    transform.rotation = Quat::from_rotation_x(angle);
                } else {
                    transform.rotation = Quat::IDENTITY;
                }
            }
            
            if let Some(arm) = arm {
                // Working blacksmith swings arm up/down on anvil clock
                if npc.role == NPCRole::Blacksmith && npc.state == AIState::Wandering && !moving && arm.side > 0.0 {
                    let cycle = (t * 2.5) % 4.0; // 1.6s striking period
                    let angle = if cycle < 2.5 {
                        -(cycle / 2.5) * 1.3 // Raise slow
                    } else {
                        -1.3 + ((cycle - 2.5) / 1.5) * 1.3 // Drop rapid
                    };
                    transform.rotation = Quat::from_rotation_x(angle);
                }
                // Farmer bending slightly when gardening
                else if npc.role == NPCRole::Farmer && npc.state == AIState::Wandering && !moving {
                    transform.rotation = Quat::from_rotation_x(0.6 + (t * 2.0).sin() * 0.15);
                }
                // Fast slashing during combat
                else if npc.state == AIState::Chasing && arm.side > 0.0 {
                    let angle = (t * 12.0).sin() * 0.8;
                    transform.rotation = Quat::from_rotation_x(angle);
                }
                else if moving {
                    let speed = if npc.state == AIState::Chasing { 16.0 } else { 10.0 };
                    let offset = if arm.side > 0.0 { std::f32::consts::PI } else { 0.0 };
                    let angle = (t * speed + offset).sin() * 0.3;
                    transform.rotation = Quat::from_rotation_x(angle);
                } else {
                    transform.rotation = Quat::IDENTITY;
                }
            }
        }
    }
}

/// Spawns golden blacksmith anvil strike sparks on strike intervals
fn npc_blacksmith_sparks(
    time: Res<Time>,
    mut commands: Commands,
    npc_query: Query<(&NPC, &Transform)>,
    spark_effect_res: Option<Res<crate::particle_effects::BlacksmithSparkEffect>>,
) {
    if let Some(ref sparks) = spark_effect_res {
        let t = time.elapsed_secs();
        for (npc, _) in npc_query.iter() {
            if npc.role == NPCRole::Blacksmith && npc.state == AIState::Wandering && npc.target_pos.is_none() {
                let last_t = t - time.delta_secs();
                let last_cycle = (last_t * 2.5) % 4.0;
                let cur_cycle = (t * 2.5) % 4.0;
                
                // Strike detected on cycle wrap
                if cur_cycle < last_cycle {
                    if let Some(work) = npc.work_pos {
                        let anvil_pos = work + Vec3::new(0.5, 0.65, 0.5);
                        commands.spawn((
                            ParticleEffect {
                                handle: sparks.0.clone(),
                                ..default()
                            },
                            Transform::from_translation(anvil_pos),
                        ));
                    }
                }
            }
        }
    }
}
