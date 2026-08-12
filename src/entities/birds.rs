use super::{AIState, Creature, CreatureData, Species};
use crate::player::camera::Player;
use bevy::ecs::relationship::Relationship;
use bevy::prelude::*;
use noise::{NoiseFn, Perlin};
use rand::RngExt;

pub struct BirdsPlugin;

impl Plugin for BirdsPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Update, (spawn_birds, bird_ai, bird_animation));
    }
}

#[derive(Component)]
pub struct Bird;

#[derive(Component)]
pub struct Wing {
    pub side: f32, // 1 or -1
}

fn spawn_birds(
    mut commands: Commands,
    bird_query: Query<Entity, With<Bird>>,
    player_query: Query<&Transform, With<Player>>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    if bird_query.iter().count() >= 30 {
        return;
    }

    let player_transform = if let Some(t) = player_query.iter().next() {
        t
    } else {
        return;
    };
    let player_pos = player_transform.translation;

    let mut rng = rand::rng();

    // Attempt to spawn one bird per frame if below limit
    if rng.random_bool(0.1) {
        let angle = rng.random_range(0.0..std::f32::consts::TAU);
        let dist = rng.random_range(40.0..60.0);
        let spawn_pos = Vec3::new(
            player_pos.x + angle.cos() * dist,
            30.0 + rng.random_range(0.0..10.0),
            player_pos.z + angle.sin() * dist,
        );

        let species = match rng.random_range(0..3) {
            0 => Species::Bird,
            1 => Species::Hawk,
            _ => Species::Crow,
        };

        let (color, speed, size) = match species {
            Species::Hawk => (Color::srgb(0.5, 0.4, 0.3), 12.0, 0.5),
            Species::Crow => (Color::srgb(0.1, 0.1, 0.1), 7.0, 0.3),
            _ => (Color::srgb(0.9, 0.9, 0.9), 5.0, 0.2),
        };

        commands
            .spawn((
                Bird,
                crate::world::water::WaterInteractor {
                    mass: (size * 0.15_f32).clamp(0.02_f32, 0.15_f32),
                    ..default()
                },
                Creature {
                    species,
                    state: AIState::Flocking,
                    last_attack_time: 0.0,
                },
                CreatureData {
                    speed,
                    size,
                    detection_radius: 10.0,
                },
                Transform::from_translation(spawn_pos),
                Visibility::default(),
                InheritedVisibility::default(),
            ))
            .with_children(|parent| {
                // 1. Body
                parent.spawn((
                    Mesh3d(meshes.add(Cuboid::new(size * 0.4, size * 0.3, size * 0.7))),
                    MeshMaterial3d(materials.add(StandardMaterial {
                        base_color: color,
                        ..default()
                    })),
                    Transform::from_translation(Vec3::ZERO),
                ));

                // 2. Wings
                for side in [-1.0, 1.0] {
                    parent.spawn((
                        Wing { side },
                        Mesh3d(meshes.add(Cuboid::new(size * 0.6, size * 0.05, size * 0.4))),
                        MeshMaterial3d(materials.add(StandardMaterial {
                            base_color: color,
                            ..default()
                        })),
                        Transform::from_translation(Vec3::new(side * size * 0.4, 0.0, 0.0)),
                    ));
                }
            });
    }
}

fn bird_ai(
    time: Res<Time>,
    player_query: Query<&Transform, With<Player>>,
    mut query: Query<(&mut Transform, &CreatureData), (With<Bird>, Without<Player>)>,
) {
    let perlin = Perlin::new(42);
    let t = time.elapsed_secs() as f64;
    let player_pos = player_query
        .single()
        .ok()
        .map(|t| t.translation)
        .unwrap_or(Vec3::ZERO);

    for (mut transform, data) in query.iter_mut() {
        let pos = transform.translation;

        // Perlin gliding logic
        let noise_x = perlin.get([pos.x as f64 * 0.05, t * 0.3]);
        let noise_z = perlin.get([pos.z as f64 * 0.05, t * 0.3, 500.0]);

        let velocity = Vec3::new(noise_x as f32, 0.0, noise_z as f32) * data.speed;
        transform.translation += velocity * time.delta_secs();

        if velocity.length_squared() > 0.001 {
            transform.look_to(velocity.normalize_or_zero(), Vec3::Y);
        }

        // Proximity Cleanup: Use horizontal distance to stay near player without following altitude
        let flat_pos = Vec2::new(pos.x, pos.z);
        let flat_player_pos = Vec2::new(player_pos.x, player_pos.z);

        if flat_pos.distance(flat_player_pos) > 100.0 {
            let dir_to_player = (flat_player_pos - flat_pos).normalize_or_zero();
            transform.translation.x += dir_to_player.x * data.speed * 0.5 * time.delta_secs();
            transform.translation.z += dir_to_player.y * data.speed * 0.5 * time.delta_secs();
        }
    }
}

fn bird_animation(
    time: Res<Time>,
    mut query: Query<(&mut Transform, &Wing, &ChildOf)>,
    creature_query: Query<(&Creature, &CreatureData)>,
) {
    let t = time.elapsed_secs();
    for (mut transform, wing, child_of) in query.iter_mut() {
        if let Ok((creature, _data)) = creature_query.get(Relationship::get(child_of)) {
            let flap_speed = match creature.species {
                Species::Hawk => 4.0,
                Species::Crow => 6.0,
                _ => 10.0,
            };

            // Flapping: rotate around Z axis based on side
            let angle = (t * flap_speed).sin() * wing.side * 0.8;
            transform.rotation = Quat::from_rotation_z(angle);
        }
    }
}
