use bevy::prelude::*;
use bevy::ecs::relationship::Relationship;
use crate::world::noise_generator::NoiseGenerator;
use crate::world::manager::{find_ground_height};
use bevy_voxel_world::prelude::*;
use crate::player::camera::Player;
use crate::player::combat::{Health, Hittable};
use crate::world::env::TimeOfDay;
use rand::RngExt;
use super::{Creature, CreatureData, Species, AIState};
use super::npc::NPC;

pub struct AnimalsPlugin;

impl Plugin for AnimalsPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Update, (spawn_animals, animal_ai, animal_animation));
    }
}

#[derive(Component)]
pub struct Animal;

#[derive(Component)]
pub struct Leg {
    pub side: f32, // 1 or -1
    pub front: f32, // 1 or -1
}

fn spawn_animals(
    mut commands: Commands,
    animal_query: Query<Entity, With<Animal>>,
    player_query: Query<(Entity, &Transform), With<Player>>,
    voxel_world: VoxelWorld<NoiseGenerator>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    time_of_day: Res<TimeOfDay>,
) {
    if animal_query.iter().count() >= 25 { return; }

    let player_transform = if let Some((_, t)) = player_query.iter().next() { t } else { return };
    let player_pos = player_transform.translation;

    let mut rng = rand::rng();
    
    if rng.random_bool(0.05) {
        let angle = rng.random_range(0.0..std::f32::consts::TAU);
        let dist = rng.random_range(30.0..50.0);
        let spawn_x = player_pos.x + angle.cos() * dist;
        let spawn_z = player_pos.z + angle.sin() * dist;
        
        // Search for ground starting from a high point if player is high, or standard height
        let search_start = player_pos.y.max(40.0);
        let spawn_y = find_ground_height(Vec3::new(spawn_x, search_start, spawn_z), &voxel_world).unwrap_or(20.0);
        
        let spawn_pos = Vec3::new(spawn_x, spawn_y, spawn_z);

        let is_night = time_of_day.hour < 5.0 || time_of_day.hour > 20.0;

        let (species, color, speed, size, detection, hp) = if spawn_y < player_pos.y - 10.0 {
            // Underground Monster Spawning
            match rng.random_range(0..2) {
                0 => (Species::Spider, Color::srgb(0.2, 0.0, 0.0), 1.4, 0.6, 25.0, 15.0),
                _ => (Species::Skeleton, Color::srgb(0.8, 0.8, 0.8), 0.9, 0.9, 30.0, 20.0),
            }
        } else {
            // Surface Spawning
            let roll = rng.random_range(0..8);
            match roll {
                0..=2 if is_night => (Species::Wolf, Color::srgb(0.3, 0.3, 0.3), 1.2, 0.8, 20.0, 20.0),
                3 if is_night => (Species::Spider, Color::srgb(0.2, 0.0, 0.0), 1.4, 0.6, 25.0, 15.0),
                4 if is_night => (Species::Skeleton, Color::srgb(0.8, 0.8, 0.8), 0.9, 0.9, 30.0, 20.0),
                0..=2 => (Species::Deer, Color::srgb(0.6, 0.4, 0.2), 1.5, 0.8, 15.0, 12.0), // Replace wolf with deer in day
                3 => (Species::Cow, Color::srgb(0.9, 0.9, 0.9), 0.8, 1.0, 5.0, 15.0),
                4 => (Species::Pig, Color::srgb(1.0, 0.7, 0.7), 1.0, 0.7, 10.0, 10.0),
                5 => (Species::Chicken, Color::srgb(1.0, 1.0, 1.0), 0.5, 0.3, 15.0, 5.0),
                _ => (Species::Deer, Color::srgb(0.6, 0.4, 0.2), 1.5, 0.8, 15.0, 12.0),
            }
        };

        commands.spawn((
            Animal,
            crate::world::water::WaterInteractor {
                mass: size * size * size,
                ..default()
            },
            Creature { species, state: AIState::Wandering, last_attack_time: 0.0 },
            CreatureData { speed, size, detection_radius: detection },
            Health::new(hp),
            Hittable,
            Transform::from_translation(spawn_pos),
            Visibility::default(),
            InheritedVisibility::default(),
        )).with_children(|parent| {
            if species == Species::Skeleton {
                // Humanoid Skeleton Model
                // Torso
                parent.spawn((
                    Mesh3d(meshes.add(Cuboid::new(0.3, 0.7, 0.2))),
                    MeshMaterial3d(materials.add(StandardMaterial { base_color: Color::srgb(0.9, 0.9, 0.9), ..default() })),
                    Transform::from_xyz(0.0, 0.1, 0.0),
                ));
                // Head
                parent.spawn((
                    Mesh3d(meshes.add(Cuboid::from_size(Vec3::splat(0.3)))),
                    MeshMaterial3d(materials.add(StandardMaterial { base_color: Color::srgb(0.9, 0.9, 0.9), ..default() })),
                    Transform::from_xyz(0.0, 0.6, 0.0),
                ));
                // Legs
                for side in [-1.0, 1.0] {
                    parent.spawn((
                        Leg { side, front: 0.0 },
                        Mesh3d(meshes.add(Cuboid::new(0.1, 0.6, 0.1))),
                        MeshMaterial3d(materials.add(StandardMaterial { base_color: Color::srgb(0.9, 0.9, 0.9), ..default() })),
                        Transform::from_xyz(side * 0.1, -0.45, 0.0),
                    ));
                }
                // Arms (Reusing Leg component for animation tagging)
                for side in [-1.0, 1.0] {
                    parent.spawn((
                        Leg { side, front: 1.0 }, 
                        Mesh3d(meshes.add(Cuboid::new(0.08, 0.6, 0.08))),
                        MeshMaterial3d(materials.add(StandardMaterial { base_color: Color::srgb(0.9, 0.9, 0.9), ..default() })),
                        Transform::from_xyz(side * 0.25, 0.2, 0.0),
                    ));
                }
            } else {
                // Quadruped Animal Model
                // 1. Body
                parent.spawn((
                    Mesh3d(meshes.add(Cuboid::new(size * 0.7, size * 0.6, size * 1.0))),
                    MeshMaterial3d(materials.add(StandardMaterial { base_color: color, ..default() })),
                    Transform::from_translation(Vec3::ZERO),
                ));

                // 2. Head
                parent.spawn((
                    Mesh3d(meshes.add(Cuboid::from_size(Vec3::splat(size * 0.4)))),
                    MeshMaterial3d(materials.add(StandardMaterial { base_color: color, ..default() })),
                    Transform::from_translation(Vec3::new(0.0, size * 0.3, -size * 0.6)), 
                ));

                // 3. Legs
                let leg_count = if species == Species::Spider { 8 } else { 4 };
                for i in 0..leg_count {
                    let x_side = if i % 2 == 0 { -0.4 } else { 0.4 };
                    let z_pos = if species == Species::Spider {
                        (i as f32 / 4.0 - 0.5) * 1.5
                    } else {
                        if i < 2 { -0.4 } else { 0.4 }
                    };
                    
                    parent.spawn((
                        Leg { side: x_side, front: z_pos },
                        Mesh3d(meshes.add(Cuboid::new(size * 0.1, size * 0.5, size * 0.1))),
                        MeshMaterial3d(materials.add(StandardMaterial { base_color: color, ..default() })),
                        Transform::from_translation(Vec3::new(x_side * size, -size * 0.4, z_pos * size)),
                    ));
                }
            }
        });
    }
}

fn animal_ai(
    time: Res<Time>,
    voxel_world: VoxelWorld<NoiseGenerator>,
    player_query: Query<(Entity, &Transform), (With<Player>, Without<Animal>)>,
    mut query: Query<(Entity, &mut Transform, &mut Creature, &CreatureData), (With<Animal>, Without<Player>)>,
    mut commands: Commands,
    time_of_day: Res<TimeOfDay>,
    collider_query: Query<&Transform, (With<bevy_rapier3d::prelude::Collider>, Without<Animal>)>,
    npc_query: Query<(Entity, &Transform), (With<NPC>, Without<Animal>)>,
) {
    let dt = time.delta_secs();
    let player_data = player_query.iter().next();
    let player_pos = player_data.map(|(_, t)| t.translation).unwrap_or(Vec3::ZERO);
    let player_entity = player_data.map(|(e, _)| e).unwrap_or(Entity::PLACEHOLDER);

    // 1. Collect data for behavioral interactions (Predators and Prey)
    let predators: Vec<_> = query.iter()
        .filter(|(_, _, c, _)| c.species == Species::Wolf)
        .map(|(_, t, _, _)| t.translation)
        .collect();

    let mut prey: Vec<_> = query.iter()
        .filter(|(_, _, c, _)| matches!(c.species, Species::Pig | Species::Chicken | Species::Deer))
        .map(|(e, t, _, _)| (e, t.translation))
        .collect();

    // Wolves also prey on helpless town citizens!
    for (npc_entity, npc_trans) in npc_query.iter() {
        prey.push((npc_entity, npc_trans.translation));
    }

    let cur_time = time.elapsed_secs();

    for (entity, mut transform, mut creature, data) in query.iter_mut() {
        let pos = transform.translation;

        // Cleanup: Despawn far away animals
        let dist = pos.distance(player_pos);
        let is_day = time_of_day.hour > 6.0 && time_of_day.hour < 19.0;
        let is_monster = matches!(creature.species, Species::Wolf | Species::Spider | Species::Skeleton);

        // Despawn monsters in daylight if they are not underground (y > 10.0)
        if is_day && is_monster && pos.y > 10.0 {
            if let Ok(mut cmd) = commands.get_entity(entity) {
                cmd.despawn();
            }
            continue;
        }

        if dist > 120.0 {
            if let Ok(mut cmd) = commands.get_entity(entity) {
                cmd.despawn();
            }
            continue;
        }

        // 1. Behavior State Selection
        match creature.species {
            Species::Wolf => {
                let mut closest_dist = data.detection_radius;
                let mut target = None;
                for (p_entity, p_pos) in &prey {
                    let d = pos.distance(*p_pos);
                    if d < closest_dist {
                        closest_dist = d;
                        target = Some((*p_entity, *p_pos));
                    }
                }
                
                if let Some((t_entity, t_pos)) = target {
                    creature.state = AIState::Chasing;
                    
                    // Damage logic
                    if pos.distance(t_pos) < 2.0 && cur_time - creature.last_attack_time > 1.0 {
                        if let Ok(mut cmd) = commands.get_entity(t_entity) {
                            cmd.insert(crate::player::combat::DamageEvent(5.0));
                            creature.last_attack_time = cur_time;
                            println!("{:?} bit {:?}!", creature.species, t_entity);
                        }
                    }

                    let mut dir = (t_pos - pos).normalize_or_zero();
                    
                    // Add separation from other wolves to prevent clustering
                    for &other_wolf_pos in &predators {
                        let d_other = pos.distance(other_wolf_pos);
                        if d_other < 2.5 && d_other > 0.01 {
                            dir += (pos - other_wolf_pos).normalize_or_zero() * 0.8;
                        }
                    }

                    let flat_dir = Vec3::new(dir.x, 0.0, dir.z).normalize_or_zero();
                    if flat_dir != Vec3::ZERO {
                        transform.look_to(flat_dir, Vec3::Y);
                    }
                } else {
                    creature.state = AIState::Wandering;
                }
            }
            Species::Spider | Species::Skeleton => {
                // Hostile monsters: chase player or closest NPC
                let mut target_entity = player_entity;
                let mut target_pos = player_pos;
                let mut closest_dist = pos.distance(player_pos);

                // Check if any NPC is closer
                for (npc_entity, npc_trans) in npc_query.iter() {
                    let d = pos.distance(npc_trans.translation);
                    if d < closest_dist {
                        closest_dist = d;
                        target_entity = npc_entity;
                        target_pos = npc_trans.translation;
                    }
                }

                if closest_dist < data.detection_radius {
                    creature.state = AIState::Chasing;
                    
                    // Attack target
                    if closest_dist < 2.5 && cur_time - creature.last_attack_time > 1.5 {
                        if target_entity != Entity::PLACEHOLDER {
                            commands.entity(target_entity).insert(crate::player::combat::DamageEvent(5.0));
                            creature.last_attack_time = cur_time;
                            println!("{:?} hit target {:?}!", creature.species, target_entity);
                        }
                    }

                    let mut dir = (target_pos - pos).normalize_or_zero();
                    
                    // Smart AI: Circle the target slightly
                    let time_offset = (entity.to_bits() as f32) % 10.0;
                    let circle_dir = Vec3::new(-dir.z, 0.0, dir.x);
                    let zig_zag = (cur_time * 2.0 + time_offset).sin() * 0.8;
                    dir = (dir + circle_dir * zig_zag).normalize_or_zero();

                    let flat_dir = Vec3::new(dir.x, 0.0, dir.z).normalize_or_zero();
                    if flat_dir != Vec3::ZERO {
                        transform.look_to(flat_dir, Vec3::Y);
                    }
                } else {
                    creature.state = AIState::Wandering;
                }
            }
            Species::Cow | Species::Pig | Species::Chicken | Species::Deer => {
                let mut flee_dir = Vec3::ZERO;
                for p_pos in &predators {
                    let d = pos.distance(*p_pos);
                    if d < data.detection_radius {
                        flee_dir += (pos - *p_pos).normalize_or_zero();
                    }
                }

                if flee_dir != Vec3::ZERO {
                    creature.state = AIState::Fleeing;
                    let flat_dir = Vec3::new(flee_dir.x, 0.0, flee_dir.z).normalize_or_zero();
                    if flat_dir != Vec3::ZERO {
                        transform.look_to(flat_dir, Vec3::Y);
                    }
                } else {
                    creature.state = AIState::Wandering;
                }
            }
            _ => {}
        }

        // 2. Movement
        let speed_mult = match creature.state {
            AIState::Chasing | AIState::Fleeing => 4.0, // High speed run
            _ => 1.0,
        };

        let forward = transform.forward();
        let mut movement = forward * data.speed * speed_mult * dt;

        // Universal Separation: "Give everything its own barrier"
        let mut separation = Vec3::ZERO;
        let p_dist = pos.distance(player_pos);
        if p_dist < 2.0 && p_dist > 0.01 {
            separation += (pos - player_pos).normalize_or_zero() * (2.0 - p_dist);
        }
        
        // Very fast brute-force separation check for all nearby animals
        // (Okay since max 25 animals)
        for &other_pos in predators.iter().chain(prey.iter().map(|(_, p)| p)) {
            let o_dist = pos.distance(other_pos);
            if o_dist < 1.5 && o_dist > 0.01 {
                separation += (pos - other_pos).normalize_or_zero() * (1.5 - o_dist);
            }
        }
        
        movement += separation * dt * 10.0;

        // Static Collider Collision Check: Prevent walking through procedural walls, doors, castle doors, slopes
        let next_pos = pos + movement;
        let collision_radius = (data.size * 0.5) + 0.45; // animal size radius + block radius
        let mut collides = false;
        
        for collider_transform in collider_query.iter() {
            let entity_pos = collider_transform.translation;
            let horizontal_dist = Vec2::new(entity_pos.x, entity_pos.z).distance(Vec2::new(next_pos.x, next_pos.z));
            let vertical_diff = (next_pos.y - entity_pos.y).abs();
            
            // Check if animal is horizontally and vertically intersecting the block collider
            if horizontal_dist < collision_radius && vertical_diff < 1.8 {
                collides = true;
                break;
            }
        }

        if collides {
            // Block horizontal movement
            movement.x = 0.0;
            movement.z = 0.0;
            
            // Smart AI response: turn away if we hit a wall!
            transform.rotate_y(1.2); 
        }

        transform.translation += movement;
        
        // 3. Terrain Following (3D Search)
        if let Some(ground_height) = find_ground_height(transform.translation, &voxel_world) {
            transform.translation.y = ground_height + (data.size / 2.0);
        }
        
        // 4. Random Wandering
        if creature.state == AIState::Wandering {
            if (time.elapsed_secs() as i32 + (pos.x * 10.0) as i32) % 5 == 0 {
                transform.rotate_y(0.02); // Slower turn
            }
        }
    }
}

fn animal_animation(
    time: Res<Time>,
    mut query: Query<(&mut Transform, &Leg, &ChildOf)>,
    creature_query: Query<(&Creature, &CreatureData)>,
) {
    let t = time.elapsed_secs();
    for (mut transform, leg, child_of) in query.iter_mut() {
        if let Ok((creature, _data)) = creature_query.get(Relationship::get(child_of)) {
            let speed = match creature.state {
                AIState::Chasing | AIState::Fleeing => 15.0,
                AIState::Wandering => 8.0,
                _ => 0.0,
            };

            if speed > 0.1 {
                if creature.species == Species::Skeleton {
                    // Humanoid animation: Legs and arms swing opposite
                    if leg.front > 0.5 {
                        // Arm swing
                        let offset = if leg.side > 0.0 { std::f32::consts::PI } else { 0.0 };
                        let angle = (t * speed + offset).sin() * 0.6; // Wider arm swing
                        transform.rotation = Quat::from_rotation_x(angle);
                    } else {
                        // Leg swing
                        let offset = if leg.side > 0.0 { 0.0 } else { std::f32::consts::PI };
                        let angle = (t * speed + offset).sin() * 0.5;
                        transform.rotation = Quat::from_rotation_x(angle);
                    }
                } else {
                    // Quadruped procedural walking: legs move in pairs
                    let offset = if (leg.side > 0.0 && leg.front > 0.0) || (leg.side < 0.0 && leg.front < 0.0) {
                        0.0
                    } else {
                        std::f32::consts::PI
                    };
                    
                    let angle = (t * speed + offset).sin() * 0.5;
                    transform.rotation = Quat::from_rotation_x(angle);
                }
            } else {
                transform.rotation = Quat::IDENTITY;
            }
        }
    }
}
