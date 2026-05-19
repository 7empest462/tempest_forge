use bevy::prelude::*;
use bevy::ecs::relationship::Relationship;
use crate::world::noise_generator::NoiseGenerator;
use crate::world::manager::{find_stable_ground_height as find_ground_height};
use bevy_voxel_world::prelude::*;
use crate::world::env::TimeOfDay;
use crate::world::settlement::{BuildingPart, Solid};
use crate::entities::AIState;
use crate::player::camera::Player;
use crate::player::combat::{Health, Hittable};
use rand::RngExt;

pub struct NPCPlugin;

impl Plugin for NPCPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Update, (npc_ai, npc_movement, npc_animation));
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
    pub last_player_look: f32, // Timer to look at player
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
) -> Entity {
    let color = match role {
        NPCRole::Farmer => Color::srgb(0.2, 0.6, 0.2), // Green
        NPCRole::Guard => Color::srgb(0.2, 0.2, 0.8),  // Blue
        NPCRole::Citizen => Color::srgb(0.6, 0.4, 0.2), // Brown
        NPCRole::Merchant => Color::srgb(0.7, 0.2, 0.7), // Purple
        NPCRole::Blacksmith => Color::srgb(0.3, 0.1, 0.05), // Dark Brown/Rust
        NPCRole::Barkeeper => Color::srgb(0.8, 0.6, 0.1), // Golden/Yellow
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
        },
        Health::new(20.0),
        Hittable,
        Transform::from_translation(pos),
        Visibility::default(),
        InheritedVisibility::default(),
    )).with_children(|npc_parent| {
            // Detailed human-like model
            // Body (Scaling up to ~1.8m total height)
            npc_parent.spawn((
                Mesh3d(meshes.add(Cuboid::new(0.4, 0.8, 0.3))),
                MeshMaterial3d(materials.add(StandardMaterial { base_color: color, ..default() })),
                Transform::from_xyz(0.0, 0.9, 0.0), // Body center at 0.9m
                NPCBody,
            ));
            
            // Head
            npc_parent.spawn((
                Mesh3d(meshes.add(Cuboid::new(0.35, 0.35, 0.35))),
                MeshMaterial3d(materials.add(StandardMaterial { base_color: Color::srgb(1.0, 0.8, 0.7), ..default() })),
                Transform::from_xyz(0.0, 1.45, 0.0),
            ));

            // Legs
            for side in [-1.0, 1.0] {
                npc_parent.spawn((
                    NPCLeg { side },
                    Mesh3d(meshes.add(Cuboid::new(0.18, 0.6, 0.18))),
                    MeshMaterial3d(materials.add(StandardMaterial { base_color: Color::srgb(0.2, 0.2, 0.2), ..default() })),
                    Transform::from_xyz(side * 0.12, 0.3, 0.0),
                ));
            }

            // Arms
            for side in [-1.0, 1.0] {
                npc_parent.spawn((
                    NPCArm { side },
                    Mesh3d(meshes.add(Cuboid::new(0.15, 0.6, 0.15))),
                    MeshMaterial3d(materials.add(StandardMaterial { base_color: color, ..default() })),
                    Transform::from_xyz(side * 0.3, 1.0, 0.0),
                ));
            }
        }).id()
}

fn npc_ai(
    time: Res<Time>,
    time_of_day: Res<TimeOfDay>,
    mut query: Query<(&mut NPC, &Transform)>,
) {
    let mut rng = rand::rng();
    let is_night = time_of_day.hour > 19.0 || time_of_day.hour < 6.0;

    for (mut npc, transform) in query.iter_mut() {
        npc.timer -= time.delta_secs();
        npc.last_player_look -= time.delta_secs();

        if is_night {
            let dist_to_home = transform.translation.distance(npc.home_pos);
            if dist_to_home < 1.0 {
                npc.state = AIState::Sleeping;
                npc.target_pos = None;
            } else {
                npc.state = AIState::Wandering;
                npc.target_pos = Some(npc.home_pos);
            }
            continue;
        }

        if npc.state == AIState::Sleeping && !is_night {
            npc.state = AIState::Wandering;
            npc.timer = 0.0; // Force re-decision
        }

        if npc.timer <= 0.0 {
            // Decision making
            match npc.role {
                NPCRole::Farmer => {
                    // Farmers stay near home (their farm)
                    if rng.random_bool(0.7) {
                        npc.state = AIState::Wandering;
                        let offset = Vec3::new(rng.random_range(-5.0..5.0), 0.0, rng.random_range(-5.0..5.0));
                        npc.target_pos = Some(npc.home_pos + offset);
                        npc.timer = rng.random_range(3.0..8.0);
                    } else {
                        npc.state = AIState::Wandering;
                        npc.target_pos = None;
                        npc.timer = rng.random_range(2.0..5.0);
                    }
                }
                NPCRole::Merchant | NPCRole::Blacksmith | NPCRole::Barkeeper => {
                    // Business owners stay at their work position during the day
                    if let Some(work) = npc.work_pos {
                        npc.state = AIState::Wandering;
                        let offset = Vec3::new(rng.random_range(-1.5..1.5), 0.0, rng.random_range(-1.5..1.5));
                        npc.target_pos = Some(work + offset);
                        npc.timer = rng.random_range(5.0..15.0);
                    } else {
                        npc.state = AIState::Wandering;
                        npc.target_pos = Some(npc.home_pos);
                        npc.timer = 5.0;
                    }
                }
                NPCRole::Guard => {
                    // Guards patrol further out
                    npc.state = AIState::Wandering;
                    let offset = Vec3::new(rng.random_range(-15.0..15.0), 0.0, rng.random_range(-15.0..15.0));
                    npc.target_pos = Some(npc.home_pos + offset);
                    npc.timer = rng.random_range(5.0..12.0);
                }
                NPCRole::Citizen => {
                    npc.state = AIState::Wandering;
                    let offset = Vec3::new(rng.random_range(-10.0..10.0), 0.0, rng.random_range(-10.0..10.0));
                    npc.target_pos = Some(npc.home_pos + offset);
                    npc.timer = rng.random_range(4.0..10.0);
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
    mut query: Query<(&mut Transform, &NPC), Without<Player>>,
) {
    let solids: Vec<_> = solid_query.iter().collect();

    for (mut transform, npc) in query.iter_mut() {
        if let Some(target) = npc.target_pos {
            let direction = (target - transform.translation).normalize_or_zero();
            let distance = transform.translation.distance(target);
            
            if distance > 0.5 {
                let speed = match npc.role {
                    NPCRole::Guard => 2.5,
                    _ => 1.5,
                };
                let next_pos = transform.translation + direction * speed * time.delta_secs();
                
                // Collision check (Simplified for NPCs)
                let mut can_move = true;
                for (s_trans, s_part) in solids.iter() {
                    let center = s_trans.translation();
                    let half_size = s_part.0;
                    
                    let p_min = next_pos + Vec3::new(-0.3, 0.0, -0.3);
                    let p_max = next_pos + Vec3::new(0.3, 1.8, 0.3);
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
                }
                
                // Look towards movement
                if direction.length_squared() > 0.01 {
                    let target_quat = Quat::from_rotation_y(direction.z.atan2(direction.x) + std::f32::consts::FRAC_PI_2);
                    transform.rotation = transform.rotation.lerp(target_quat, 0.1);
                }
            }
        }

        // Social awareness: look at player if close
        let player_pos = player_query.iter().next().map(|t| t.translation).unwrap_or(Vec3::ZERO);
        if transform.translation.distance(player_pos) < 5.0 {
            let dir_to_player = (player_pos - transform.translation).normalize_or_zero();
            if dir_to_player != Vec3::ZERO {
                let target_quat = Quat::from_rotation_y(dir_to_player.z.atan2(dir_to_player.x) + std::f32::consts::FRAC_PI_2);
                transform.rotation = transform.rotation.lerp(target_quat, 0.1);
            }
        }

        // Terrain clamping
        if npc.state != AIState::Sleeping {
            if let Some(ground_height) = find_ground_height(transform.translation, &voxel_world) {
                transform.translation.y = ground_height;
            }
        } else {
            // Snap to home altitude + small offset for bed
            transform.translation.y = npc.home_pos.y;
            
            // Lying down rotation
            let sleep_quat = Quat::from_rotation_x(std::f32::consts::FRAC_PI_2);
            transform.rotation = transform.rotation.lerp(sleep_quat, 0.1);
        }
    }
}

pub fn npc_animation(
    time: Res<Time>,
    mut query: Query<(&mut Transform, Option<&NPCLeg>, Option<&NPCArm>, &ChildOf), Without<NPC>>,
    npc_query: Query<&NPC>,
) {
    let t = time.elapsed_secs();
    
    for (mut transform, leg, arm, child_of) in query.iter_mut() {
        if let Ok(npc) = npc_query.get(Relationship::get(child_of)) {
            let moving = npc.target_pos.is_some();

            if let Some(leg) = leg {
                if moving {
                    let speed = 10.0;
                    let offset = if leg.side > 0.0 { 0.0 } else { std::f32::consts::PI };
                    let angle = (t * speed + offset).sin() * 0.4;
                    transform.rotation = Quat::from_rotation_x(angle);
                } else {
                    transform.rotation = Quat::IDENTITY;
                }
            }
            
            if let Some(arm) = arm {
                if moving {
                    let speed = 10.0;
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
