use super::npc::NPC;
use super::{AIState, Creature, CreatureData, Species};
use crate::player::camera::Player;
use crate::player::combat::{Health, Hittable};
use crate::voxel::chunk::BlockType;
use crate::world::env::TimeOfDay;
use crate::world::manager::find_ground_height;
use crate::world::noise_generator::NoiseGenerator;
use bevy::ecs::relationship::Relationship;
use bevy::prelude::*;
use bevy_voxel_world::prelude::*;
use rand::RngExt;

pub struct AnimalsPlugin;

impl Plugin for AnimalsPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(
            Update,
            (
                spawn_animals,
                animal_ai,
                animal_animation,
                animate_glb_creatures,
                animate_triangaroo_hop,
            ),
        );
    }
}

#[derive(Component)]
pub struct Animal;

#[derive(Component)]
pub struct SwimTarget {
    pub target: Vec3,
    pub timer: f32,
    pub is_water: bool,
}

#[derive(Component)]
pub struct Leg {
    pub side: f32,  // 1 or -1
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
    asset_server: Res<AssetServer>,
) {
    if animal_query.iter().count() >= 25 {
        return;
    }

    let player_transform = if let Some((_, t)) = player_query.iter().next() {
        t
    } else {
        return;
    };
    let player_pos = player_transform.translation;

    let mut rng = rand::rng();

    if rng.random_bool(0.05) {
        let angle = rng.random_range(0.0..std::f32::consts::TAU);
        let dist = rng.random_range(30.0..50.0);
        let spawn_x = player_pos.x + angle.cos() * dist;
        let spawn_z = player_pos.z + angle.sin() * dist;

        // Search for ground starting from a high point if player is high, or standard height
        let search_start = player_pos.y.max(40.0);
        let spawn_y = find_ground_height(Vec3::new(spawn_x, search_start, spawn_z), &voxel_world)
            .unwrap_or(20.0);

        // Prevent spawning land animals directly in water
        let is_water = spawn_y < 15.2
            && matches!(
                voxel_world.get_voxel(IVec3::new(
                    spawn_x.round() as i32,
                    15,
                    spawn_z.round() as i32
                )),
                WorldVoxel::Air
            );
        if is_water {
            return;
        }

        let spawn_pos = Vec3::new(spawn_x, spawn_y, spawn_z);

        let is_day = time_of_day.hour > 6.0 && time_of_day.hour < 19.0;
        let is_night = !is_day;

        let (species, color, speed, size, detection, hp) = if spawn_y <= 12.0 {
            // Underground Monster Spawning
            match rng.random_range(0..3) {
                0 => (
                    Species::Spider,
                    Color::srgb(0.2, 0.0, 0.0),
                    1.4,
                    0.6,
                    25.0,
                    15.0,
                ),
                1 => (Species::Cyclops, Color::WHITE, 0.5, 3.0, 35.0, 50.0),
                _ => (
                    Species::Skeleton,
                    Color::srgb(0.8, 0.8, 0.8),
                    0.9,
                    0.9,
                    30.0,
                    20.0,
                ),
            }
        } else if is_night {
            // Surface Spawning at Night (Only monsters / night creatures)
            match rng.random_range(0..5) {
                0 => (
                    Species::Wolf,
                    Color::srgb(0.3, 0.3, 0.3),
                    1.2,
                    0.8,
                    20.0,
                    20.0,
                ),
                1 => (
                    Species::Spider,
                    Color::srgb(0.2, 0.0, 0.0),
                    1.4,
                    0.6,
                    25.0,
                    15.0,
                ),
                2 => (
                    Species::Skeleton,
                    Color::srgb(0.8, 0.8, 0.8),
                    0.9,
                    0.9,
                    30.0,
                    20.0,
                ),
                3 => (Species::Cyclops, Color::WHITE, 0.5, 3.0, 35.0, 50.0),
                _ => (Species::Triangaroo, Color::WHITE, 0.4, 3.2, 30.0, 40.0),
            }
        } else {
            // Surface Spawning during Day (Only passive / day creatures)
            match rng.random_range(0..6) {
                0 => (
                    Species::Deer,
                    Color::srgb(0.6, 0.4, 0.2),
                    1.5,
                    0.8,
                    15.0,
                    12.0,
                ),
                1 => (
                    Species::Cow,
                    Color::srgb(0.9, 0.9, 0.9),
                    0.8,
                    1.0,
                    5.0,
                    15.0,
                ),
                2 => (
                    Species::Pig,
                    Color::srgb(1.0, 0.7, 0.7),
                    1.0,
                    0.7,
                    10.0,
                    10.0,
                ),
                3 => (
                    Species::Chicken,
                    Color::srgb(1.0, 1.0, 1.0),
                    0.5,
                    0.3,
                    15.0,
                    5.0,
                ),
                4 => (
                    Species::Deer,
                    Color::srgb(0.6, 0.4, 0.2),
                    1.5,
                    0.8,
                    15.0,
                    12.0,
                ),
                _ => (
                    Species::Cow,
                    Color::srgb(0.9, 0.9, 0.9),
                    0.8,
                    1.0,
                    5.0,
                    15.0,
                ),
            }
        };

        commands
            .spawn((
                Animal,
                crate::world::water::WaterInteractor {
                    mass: size * size * size,
                    ..default()
                },
                Creature {
                    species,
                    state: AIState::Wandering,
                    last_attack_time: 0.0,
                },
                CreatureData {
                    speed,
                    size,
                    detection_radius: detection,
                },
                Health::new(hp),
                Hittable,
                Transform::from_translation(spawn_pos),
                Visibility::default(),
                InheritedVisibility::default(),
            ))
            .with_children(|parent| {
                if species == Species::Triangaroo {
                    parent.spawn((
                        SceneRoot(asset_server.load("059_Triangaroo_Art.glb#Scene0")),
                        Transform::from_translation(Vec3::new(0.0, -size * 0.5, 0.0))
                            .with_rotation(Quat::from_rotation_y(std::f32::consts::PI))
                            .with_scale(Vec3::splat(size)),
                    ));
                } else if species == Species::Cyclops {
                    parent.spawn((
                        SceneRoot(asset_server.load("060_Polypug_Art.glb#Scene0")),
                        Transform::from_translation(Vec3::new(0.0, -size * 0.5, 0.0))
                            .with_rotation(Quat::from_rotation_y(std::f32::consts::PI))
                            .with_scale(Vec3::splat(size)),
                    ));
                } else if species == Species::Skeleton {
                    // Humanoid Skeleton Model
                    // Torso
                    parent.spawn((
                        Mesh3d(meshes.add(Cuboid::new(0.3, 0.7, 0.2))),
                        MeshMaterial3d(materials.add(StandardMaterial {
                            base_color: Color::srgb(0.9, 0.9, 0.9),
                            ..default()
                        })),
                        Transform::from_xyz(0.0, 0.1, 0.0),
                    ));
                    // Head
                    parent.spawn((
                        Mesh3d(meshes.add(Cuboid::from_size(Vec3::splat(0.3)))),
                        MeshMaterial3d(materials.add(StandardMaterial {
                            base_color: Color::srgb(0.9, 0.9, 0.9),
                            ..default()
                        })),
                        Transform::from_xyz(0.0, 0.6, 0.0),
                    ));
                    // Legs
                    for side in [-1.0, 1.0] {
                        parent.spawn((
                            Leg { side, front: 0.0 },
                            Mesh3d(meshes.add(Cuboid::new(0.1, 0.6, 0.1))),
                            MeshMaterial3d(materials.add(StandardMaterial {
                                base_color: Color::srgb(0.9, 0.9, 0.9),
                                ..default()
                            })),
                            Transform::from_xyz(side * 0.1, -0.45, 0.0),
                        ));
                    }
                    // Arms (Reusing Leg component for animation tagging)
                    for side in [-1.0, 1.0] {
                        parent.spawn((
                            Leg { side, front: 1.0 },
                            Mesh3d(meshes.add(Cuboid::new(0.08, 0.6, 0.08))),
                            MeshMaterial3d(materials.add(StandardMaterial {
                                base_color: Color::srgb(0.9, 0.9, 0.9),
                                ..default()
                            })),
                            Transform::from_xyz(side * 0.25, 0.2, 0.0),
                        ));
                    }
                } else {
                    // Quadruped Animal Model
                    let mut body_shape = Cuboid::new(size * 0.7, size * 0.6, size * 1.0);
                    let mut head_shape = Cuboid::from_size(Vec3::splat(size * 0.4));
                    let mut head_pos = Vec3::new(0.0, size * 0.3, -size * 0.6);
                    let mut leg_shape = Cuboid::new(size * 0.1, size * 0.5, size * 0.1);
                    let mut leg_y = -size * 0.4;
                    let mut leg_x_offset = size;

                    if species == Species::Deer {
                        body_shape = Cuboid::new(size * 0.5, size * 0.5, size * 1.1);
                        head_shape =
                            Cuboid::from_size(Vec3::new(size * 0.3, size * 0.35, size * 0.3));
                        head_pos = Vec3::new(0.0, size * 0.4, -size * 0.65);
                        leg_shape = Cuboid::new(size * 0.07, size * 0.7, size * 0.07);
                        leg_y = -size * 0.5;
                        leg_x_offset = size * 0.75;
                    } else if species == Species::Pig {
                        body_shape = Cuboid::new(size * 0.8, size * 0.6, size * 0.9);
                        head_shape = Cuboid::from_size(Vec3::splat(size * 0.45));
                        head_pos = Vec3::new(0.0, size * 0.1, -size * 0.55);
                        leg_shape = Cuboid::new(size * 0.12, size * 0.35, size * 0.12);
                        leg_y = -size * 0.35;
                    }

                    // 1. Body
                    parent.spawn((
                        Mesh3d(meshes.add(body_shape)),
                        MeshMaterial3d(materials.add(StandardMaterial {
                            base_color: color,
                            ..default()
                        })),
                        Transform::from_translation(Vec3::ZERO),
                    ));

                    // 2. Head
                    parent.spawn((
                        Mesh3d(meshes.add(head_shape)),
                        MeshMaterial3d(materials.add(StandardMaterial {
                            base_color: color,
                            ..default()
                        })),
                        Transform::from_translation(head_pos),
                    ));

                    // 3. Antlers (Deer only)
                    if species == Species::Deer {
                        let antler_mat = materials.add(StandardMaterial {
                            base_color: Color::srgb(0.85, 0.8, 0.75), // Bone/antler color
                            perceptual_roughness: 0.9,
                            ..default()
                        });
                        // Left Antler Base
                        parent.spawn((
                            Mesh3d(meshes.add(Cuboid::new(size * 0.05, size * 0.35, size * 0.05))),
                            MeshMaterial3d(antler_mat.clone()),
                            Transform::from_translation(Vec3::new(
                                -size * 0.1,
                                size * 0.65,
                                -size * 0.65,
                            )),
                        ));
                        // Left Antler Branch
                        parent.spawn((
                            Mesh3d(meshes.add(Cuboid::new(size * 0.15, size * 0.05, size * 0.05))),
                            MeshMaterial3d(antler_mat.clone()),
                            Transform::from_translation(Vec3::new(
                                -size * 0.18,
                                size * 0.75,
                                -size * 0.65,
                            )),
                        ));
                        // Right Antler Base
                        parent.spawn((
                            Mesh3d(meshes.add(Cuboid::new(size * 0.05, size * 0.35, size * 0.05))),
                            MeshMaterial3d(antler_mat.clone()),
                            Transform::from_translation(Vec3::new(
                                size * 0.1,
                                size * 0.65,
                                -size * 0.65,
                            )),
                        ));
                        // Right Antler Branch
                        parent.spawn((
                            Mesh3d(meshes.add(Cuboid::new(size * 0.15, size * 0.05, size * 0.05))),
                            MeshMaterial3d(antler_mat),
                            Transform::from_translation(Vec3::new(
                                size * 0.18,
                                size * 0.75,
                                -size * 0.65,
                            )),
                        ));
                    }

                    // 4. Legs
                    let leg_count = if species == Species::Spider { 8 } else { 4 };
                    for i in 0..leg_count {
                        let x_side = if i % 2 == 0 { -0.4 } else { 0.4 };
                        let z_pos = if species == Species::Spider {
                            (i as f32 / 4.0 - 0.5) * 1.5
                        } else {
                            if i < 2 { -0.4 } else { 0.4 }
                        };

                        parent.spawn((
                            Leg {
                                side: x_side,
                                front: z_pos,
                            },
                            Mesh3d(meshes.add(leg_shape)),
                            MeshMaterial3d(materials.add(StandardMaterial {
                                base_color: color,
                                ..default()
                            })),
                            Transform::from_translation(Vec3::new(
                                x_side * leg_x_offset,
                                leg_y,
                                z_pos * size,
                            )),
                        ));
                    }
                }
            });
    }
}

fn find_nearby_water_pos(
    origin: Vec3,
    voxel_world: &VoxelWorld<NoiseGenerator>,
    radius: f32,
) -> Option<Vec3> {
    let mut rng = rand::rng();
    for _ in 0..15 {
        let angle = rng.random_range(0.0..std::f32::consts::TAU);
        let r = rng.random_range(5.0..radius);
        let x = origin.x + angle.cos() * r;
        let z = origin.z + angle.sin() * r;

        let pos_y15 = IVec3::new(x.round() as i32, 15, z.round() as i32);
        let pos_y14 = IVec3::new(x.round() as i32, 14, z.round() as i32);

        let vox_y15 = voxel_world.get_voxel(pos_y15);
        let vox_y14 = voxel_world.get_voxel(pos_y14);

        let is_y15_air = matches!(vox_y15, WorldVoxel::Air);
        let is_y14_air_or_sand = matches!(vox_y14, WorldVoxel::Air)
            || matches!(vox_y14, WorldVoxel::Solid(mat) if mat == BlockType::Sand as u8);

        if is_y15_air && is_y14_air_or_sand {
            return Some(Vec3::new(x, 15.1, z));
        }
    }
    None
}

fn find_nearby_land_pos(
    origin: Vec3,
    voxel_world: &VoxelWorld<NoiseGenerator>,
    radius: f32,
) -> Option<Vec3> {
    let mut rng = rand::rng();
    for _ in 0..15 {
        let angle = rng.random_range(0.0..std::f32::consts::TAU);
        let r = rng.random_range(5.0..radius);
        let x = origin.x + angle.cos() * r;
        let z = origin.z + angle.sin() * r;

        let pos = Vec3::new(x, 25.0, z);
        if let Some(gh) = find_ground_height(pos, voxel_world)
            && gh >= 15.2
        {
            return Some(Vec3::new(x, gh, z));
        }
    }
    None
}

fn animal_ai(
    time: Res<Time>,
    voxel_world: VoxelWorld<NoiseGenerator>,
    player_query: Query<(Entity, &Transform), (With<Player>, Without<Animal>)>,
    mut query: Query<
        (Entity, &mut Transform, &mut Creature, &CreatureData),
        (With<Animal>, Without<Player>),
    >,
    mut commands: Commands,
    time_of_day: Res<TimeOfDay>,
    collider_query: Query<&Transform, (With<bevy_rapier3d::prelude::Collider>, Without<Animal>)>,
    npc_query: Query<(Entity, &Transform), (With<NPC>, Without<Animal>)>,
    mut swim_target_query: Query<&mut SwimTarget>,
) {
    let dt = time.delta_secs();
    let player_data = player_query.iter().next();
    let player_pos = player_data
        .map(|(_, t)| t.translation)
        .unwrap_or(Vec3::ZERO);
    let player_entity = player_data.map(|(e, _)| e).unwrap_or(Entity::PLACEHOLDER);

    // 1. Collect data for behavioral interactions (Predators and Prey)
    let predators: Vec<_> = query
        .iter()
        .filter(|(_, _, c, _)| c.species == Species::Wolf)
        .map(|(_, t, _, _)| t.translation)
        .collect();

    let mut prey: Vec<_> = query
        .iter()
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
        let is_in_water = pos.y < 15.2;
        let mut rng = rand::rng();
        let mut opt_swim_target = swim_target_query.get_mut(entity).ok();
        let mut target_dir = None;

        let can_swim = matches!(
            creature.species,
            Species::Deer | Species::Cow | Species::Pig | Species::Wolf
        );

        if is_in_water {
            // Land creatures in water must swim to land
            let needs_new_land_target = match opt_swim_target {
                Some(ref st) => st.is_water || st.timer <= 0.0,
                None => true,
            };

            if needs_new_land_target {
                if let Some(land_pos) = find_nearby_land_pos(pos, &voxel_world, 75.0) {
                    commands.entity(entity).insert(SwimTarget {
                        target: land_pos,
                        timer: 10.0,
                        is_water: false,
                    });
                    target_dir = Some((land_pos - pos).normalize_or_zero());
                }
            } else if let Some(ref mut st) = opt_swim_target {
                st.timer -= dt;
                target_dir = Some((st.target - pos).normalize_or_zero());
            }
        } else {
            // On land
            if let Some(ref mut st) = opt_swim_target {
                if !st.is_water {
                    // Reached land, clear target
                    commands.entity(entity).remove::<SwimTarget>();
                } else {
                    // Seeking water
                    st.timer -= dt;
                    if st.timer <= 0.0 {
                        commands.entity(entity).remove::<SwimTarget>();
                    } else {
                        target_dir = Some((st.target - pos).normalize_or_zero());
                    }
                }
            } else if can_swim {
                // Slower rate of entering water: 0.005% chance per frame (approx. once every 5.5 minutes)
                let swim_decide_chance = (0.00005 * dt * 60.0).clamp(0.0, 1.0);
                if rng.random_bool(swim_decide_chance as f64)
                    && let Some(water_pos) = find_nearby_water_pos(pos, &voxel_world, 45.0)
                {
                    commands.entity(entity).insert(SwimTarget {
                        target: water_pos,
                        timer: 15.0,
                        is_water: true,
                    });
                    target_dir = Some((water_pos - pos).normalize_or_zero());
                }
            }
        }

        // Cleanup: Despawn far away animals
        let dist = pos.distance(player_pos);
        let is_day = time_of_day.hour > 6.0 && time_of_day.hour < 19.0;
        let is_monster = matches!(
            creature.species,
            Species::Wolf
                | Species::Spider
                | Species::Skeleton
                | Species::Cyclops
                | Species::Triangaroo
        );

        // Despawn day-only surface animals at night if they are not underground (y > 10.0)
        let is_day_animal = matches!(
            creature.species,
            Species::Deer | Species::Cow | Species::Pig | Species::Chicken
        );
        if !is_day && is_day_animal && pos.y > 10.0 {
            if let Ok(mut cmd) = commands.get_entity(entity) {
                cmd.despawn();
            }
            continue;
        }

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
                    if pos.distance(t_pos) < 2.0
                        && cur_time - creature.last_attack_time > 1.0
                        && let Ok(mut cmd) = commands.get_entity(t_entity)
                    {
                        cmd.insert(crate::player::combat::DamageEvent(5.0));
                        creature.last_attack_time = cur_time;
                        println!("{:?} bit {:?}!", creature.species, t_entity);
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
            Species::Spider | Species::Skeleton | Species::Cyclops | Species::Triangaroo => {
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
                    let attack_cooldown = if creature.species == Species::Cyclops {
                        2.0
                    } else {
                        1.5
                    };
                    let attack_damage = if creature.species == Species::Cyclops {
                        12.0
                    } else {
                        5.0
                    };
                    let attack_range = if creature.species == Species::Cyclops {
                        3.0
                    } else {
                        2.5
                    };

                    if closest_dist < attack_range
                        && cur_time - creature.last_attack_time > attack_cooldown
                        && target_entity != Entity::PLACEHOLDER
                    {
                        commands
                            .entity(target_entity)
                            .insert(crate::player::combat::DamageEvent(attack_damage));
                        creature.last_attack_time = cur_time;
                        println!("{:?} hit target {:?}!", creature.species, target_entity);
                    }

                    let mut dir = (target_pos - pos).normalize_or_zero();

                    // Smart AI: Circle the target slightly (only for Spider/Skeleton/Triangaroo)
                    if creature.species != Species::Cyclops {
                        let time_offset = (entity.to_bits() as f32) % 10.0;
                        let circle_dir = Vec3::new(-dir.z, 0.0, dir.x);
                        let zig_zag = (cur_time * 2.0 + time_offset).sin() * 0.8;
                        dir = (dir + circle_dir * zig_zag).normalize_or_zero();
                    }

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

        if creature.state == AIState::Wandering
            && let Some(dir) = target_dir
        {
            let flat_dir = Vec3::new(dir.x, 0.0, dir.z).normalize_or_zero();
            if flat_dir != Vec3::ZERO {
                transform.look_to(flat_dir, Vec3::Y);
            }
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
            let horizontal_dist =
                Vec2::new(entity_pos.x, entity_pos.z).distance(Vec2::new(next_pos.x, next_pos.z));
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

        // 3. Terrain Following & Swimming (3D Search)
        if let Some(ground_height) = find_ground_height(transform.translation, &voxel_world) {
            let water_level = 15.1;
            if ground_height < water_level {
                // Swim/float at water surface if deep enough, otherwise wade on the bottom
                let feet_swim_y = water_level - (data.size * 0.7);
                let feet_y = ground_height.max(feet_swim_y);
                transform.translation.y = feet_y + (data.size / 2.0);
            } else {
                transform.translation.y = ground_height + (data.size / 2.0);
            }
        }

        // 4. Random Wandering
        if creature.state == AIState::Wandering && target_dir.is_none() {
            let mut rng = rand::rng();
            // Turn chance: 1.5% chance per frame (approx. once every 1.1 seconds)
            let turn_chance = (0.015 * dt * 60.0).clamp(0.0, 1.0);
            if rng.random_bool(turn_chance as f64) {
                let turn_angle = rng.random_range(-1.2..1.2);
                transform.rotate_y(turn_angle);
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
                        let offset = if leg.side > 0.0 {
                            std::f32::consts::PI
                        } else {
                            0.0
                        };
                        let angle = (t * speed + offset).sin() * 0.6; // Wider arm swing
                        transform.rotation = Quat::from_rotation_x(angle);
                    } else {
                        // Leg swing
                        let offset = if leg.side > 0.0 {
                            0.0
                        } else {
                            std::f32::consts::PI
                        };
                        let angle = (t * speed + offset).sin() * 0.5;
                        transform.rotation = Quat::from_rotation_x(angle);
                    }
                } else {
                    // Quadruped procedural walking: legs move in pairs
                    let offset = if (leg.side > 0.0 && leg.front > 0.0)
                        || (leg.side < 0.0 && leg.front < 0.0)
                    {
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

#[derive(Component)]
pub struct DefaultBoneRotation(pub Quat);

fn animate_glb_creatures(
    mut commands: Commands,
    time: Res<Time>,
    child_of_query: Query<&ChildOf>,
    creature_query: Query<(&Creature, &CreatureData)>,
    mut bone_query: Query<(Entity, &mut Transform, &Name, Option<&DefaultBoneRotation>)>,
) {
    let t = time.elapsed_secs();

    for (entity, mut transform, name, opt_default) in bone_query.iter_mut() {
        // Get or initialize default rotation from the loaded GLTF model bind pose
        let default_rot = if let Some(def) = opt_default {
            def.0
        } else {
            let rot = transform.rotation;
            commands.entity(entity).insert(DefaultBoneRotation(rot));
            rot
        };

        // Walk up parents using ChildOf to find the root Creature
        let mut root_creature = None;
        let mut curr = entity;
        while let Ok(child_of) = child_of_query.get(curr) {
            let parent_entity = Relationship::get(child_of);
            if let Ok((creature, data)) = creature_query.get(parent_entity) {
                root_creature = Some((creature, data));
                break;
            }
            curr = parent_entity;
        }

        let Some((creature, _data)) = root_creature else {
            continue;
        };

        // Animating speed
        let speed = match creature.state {
            AIState::Chasing | AIState::Fleeing => 12.0,
            AIState::Wandering => 6.0,
            _ => 0.0,
        };

        let bone_name = name.as_str();

        if speed > 0.1 {
            match creature.species {
                Species::Triangaroo => {
                    // Kangaroo hopping leg motion
                    // UpLeg.L and UpLeg.R swing together (in-phase)
                    if bone_name == "UpLeg.L" || bone_name == "UpLeg.R" {
                        let angle = (t * speed).sin() * 0.4 - 0.2;
                        transform.rotation = default_rot * Quat::from_rotation_x(angle);
                    } else if bone_name == "Leg.L" || bone_name == "Leg.R" {
                        let angle = (t * speed + 1.0).sin() * 0.3 + 0.3;
                        transform.rotation = default_rot * Quat::from_rotation_x(angle);
                    } else if bone_name == "Foot.L" || bone_name == "Foot.R" {
                        // Dynamic counter-rotation to keep feet flat/parallel to ground
                        let angle = -(t * speed + 1.0).sin() * 0.3 - 0.15;
                        transform.rotation = default_rot * Quat::from_rotation_x(angle);
                    } else if bone_name == "Tail"
                        || bone_name == "Tail.001"
                        || bone_name == "Tail.002"
                    {
                        let angle = (t * speed * 2.0).cos() * 0.15;
                        transform.rotation = default_rot * Quat::from_rotation_x(angle);
                    } else if bone_name == "Spine" || bone_name == "Spine.001" {
                        let angle = (t * speed).cos() * 0.08;
                        transform.rotation = default_rot * Quat::from_rotation_x(angle);
                    }
                }
                Species::Cyclops => {
                    // Quadruped walking motion: alternating back legs, front legs out of phase
                    if bone_name == "thigh.L" {
                        let angle = (t * speed).sin() * 0.5;
                        transform.rotation = default_rot * Quat::from_rotation_x(angle);
                    } else if bone_name == "thigh.R" {
                        let angle = -(t * speed).sin() * 0.5;
                        transform.rotation = default_rot * Quat::from_rotation_x(angle);
                    } else if bone_name == "shin.L" {
                        let angle = (t * speed + 1.5).cos() * 0.3 + 0.2;
                        transform.rotation = default_rot * Quat::from_rotation_x(angle);
                    } else if bone_name == "shin.R" {
                        let angle = -(t * speed + 1.5).cos() * 0.3 + 0.2;
                        transform.rotation = default_rot * Quat::from_rotation_x(angle);
                    } else if bone_name == "front_thigh.L" {
                        let angle = -(t * speed).sin() * 0.4;
                        transform.rotation = default_rot * Quat::from_rotation_x(angle);
                    } else if bone_name == "front_thigh.R" {
                        let angle = (t * speed).sin() * 0.4;
                        transform.rotation = default_rot * Quat::from_rotation_x(angle);
                    } else if bone_name == "front_shin.L" {
                        let angle = -(t * speed + 1.5).cos() * 0.3 + 0.2;
                        transform.rotation = default_rot * Quat::from_rotation_x(angle);
                    } else if bone_name == "front_shin.R" {
                        let angle = (t * speed + 1.5).cos() * 0.3 + 0.2;
                        transform.rotation = default_rot * Quat::from_rotation_x(angle);
                    } else if bone_name == "shoulder.L" {
                        let angle = -(t * speed).sin() * 0.15;
                        transform.rotation = default_rot * Quat::from_rotation_y(angle);
                    } else if bone_name == "shoulder.R" {
                        let angle = (t * speed).sin() * 0.15;
                        transform.rotation = default_rot * Quat::from_rotation_y(angle);
                    }
                }
                _ => {}
            }
        } else {
            // Idle breathing / relaxation - return to default bind pose
            match creature.species {
                Species::Triangaroo => {
                    if bone_name == "Tail" || bone_name == "Tail.001" || bone_name == "Tail.002" {
                        transform.rotation =
                            default_rot * Quat::from_rotation_x((t * 2.0).sin() * 0.05);
                    } else if bone_name == "Spine" || bone_name == "Spine.001" {
                        transform.rotation =
                            default_rot * Quat::from_rotation_x((t * 1.5).sin() * 0.02);
                    } else if bone_name == "UpLeg.L"
                        || bone_name == "UpLeg.R"
                        || bone_name == "Leg.L"
                        || bone_name == "Leg.R"
                        || bone_name == "Foot.L"
                        || bone_name == "Foot.R"
                    {
                        transform.rotation = default_rot;
                    }
                }
                Species::Cyclops
                    if (bone_name.contains("thigh")
                        || bone_name.contains("shin")
                        || bone_name.contains("shoulder")) =>
                {
                    transform.rotation = default_rot;
                }
                _ => {}
            }
        }
    }
}

fn animate_triangaroo_hop(
    time: Res<Time>,
    creature_query: Query<(&Creature, &CreatureData)>,
    mut scene_query: Query<(&mut Transform, &ChildOf), With<SceneRoot>>,
) {
    let t = time.elapsed_secs();
    for (mut transform, child_of) in scene_query.iter_mut() {
        let parent_entity = Relationship::get(child_of);
        if let Ok((creature, data)) = creature_query.get(parent_entity) {
            if creature.species == Species::Triangaroo {
                let speed = match creature.state {
                    AIState::Chasing | AIState::Fleeing => 12.0,
                    AIState::Wandering => 6.0,
                    _ => 0.0,
                };
                if speed > 0.1 {
                    // Hop height bobbing: sine wave maxed at 0 to stay above ground
                    let hop_height = (t * speed).sin().max(0.0) * 0.8 * data.size;
                    transform.translation.y = -data.size * 0.5 + hop_height;
                } else {
                    transform.translation.y = -data.size * 0.5;
                }
            } else if creature.species == Species::Cyclops {
                let speed = match creature.state {
                    AIState::Chasing | AIState::Fleeing => 12.0,
                    AIState::Wandering => 6.0,
                    _ => 0.0,
                };
                if speed > 0.1 {
                    let walk_bob = (t * speed * 2.0).sin().abs() * 0.15 * data.size;
                    transform.translation.y = -data.size * 0.5 + walk_bob;
                } else {
                    transform.translation.y = -data.size * 0.5;
                }
            }
        }
    }
}
