use super::water::{WaterImpulseEvent, WaterInteractor};
use crate::entities::animals::Animal;
use crate::entities::npc::NPC;
use crate::player::camera::Player;
use crate::player::combat::{DamageEvent, Health, SafeInsert, TornadoDamaged};
use crate::world::noise_generator::NoiseGenerator;
use crate::world::settlement::SettlementBuilding;
use crate::world::tree_generator::TreeEntity;
use bevy::prelude::*;
use bevy_hanabi::prelude::{
    Attribute, ColorOverLifetimeModifier, EffectAsset, EffectSpawner, Gradient as HanabiGradient,
    Module, ParticleEffect, SetAttributeModifier, SetPositionCircleModifier,
    SetVelocitySphereModifier, ShapeDimension, SizeOverLifetimeModifier, SpawnerSettings,
};
use bevy_voxel_world::prelude::*;
use rand::RngExt;
use std::f32::consts::PI;

pub struct WeatherPlugin;

impl Plugin for WeatherPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins(MaterialPlugin::<TornadoMaterial>::default())
            .insert_resource(WeatherManager::default())
            .add_systems(Startup, (setup_rain_effect, setup_tornado_effect))
            .add_systems(
                Update,
                (
                    transition_weather_system,
                    manage_weather_particles,
                    simulate_rain_impacts,
                    spawn_and_update_tornadoes,
                    animate_tornado_funnel,
                ),
            );
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Reflect)]
pub enum WeatherState {
    #[default]
    Sunny,
    Cloudy,
    Rainy,
    Stormy,
}

#[derive(Resource)]
pub struct WeatherManager {
    pub current: WeatherState,
    pub target: WeatherState,
    pub state_timer: Timer,
    pub cloudiness: f32, // Smooth interpolation from 0.0 to 1.0
    pub wetness: f32,    // Smooth interpolation from 0.0 to 1.0
}

impl Default for WeatherManager {
    fn default() -> Self {
        Self {
            current: WeatherState::Sunny,
            target: WeatherState::Sunny,
            state_timer: Timer::from_seconds(45.0, TimerMode::Repeating), // Transition every 45-90s
            cloudiness: 0.0,
            wetness: 0.0,
        }
    }
}

#[derive(Resource, Clone)]
pub struct RainEffect(pub Handle<EffectAsset>);

#[derive(Resource, Clone)]
pub struct TornadoEffect(pub Handle<EffectAsset>);

#[derive(Component)]
pub struct RainSpawner;

#[derive(Component)]
pub struct Tornado {
    pub change_timer: Timer,
    pub movement_dir: Vec2,
    pub sound_entity: Option<Entity>,
}

#[derive(Component)]
pub struct TornadoFunnelPart {
    pub base_scale: Vec3,
    pub height_offset: f32,
    pub spin_speed: f32,
    pub phase: f32,
}

use bevy::render::render_resource::AsBindGroup;
use bevy::shader::ShaderRef;

#[derive(Asset, TypePath, AsBindGroup, Debug, Clone)]
pub struct TornadoMaterial {
    #[uniform(0)]
    pub color: LinearRgba,
    #[uniform(0)]
    pub time: f32,
}

impl Material for TornadoMaterial {
    fn fragment_shader() -> ShaderRef {
        "shaders/tornado.wgsl".into()
    }

    fn alpha_mode(&self) -> AlphaMode {
        AlphaMode::Blend
    }
}

// Particle asset setup
fn setup_rain_effect(mut commands: Commands, mut effects: ResMut<Assets<EffectAsset>>) {
    let mut module = Module::default();

    // Spawn in a wide circle above the player camera
    let init_pos = SetPositionCircleModifier {
        center: module.lit(Vec3::ZERO),
        axis: module.lit(Vec3::Y),
        radius: module.lit(45.0),
        dimension: ShapeDimension::Volume,
    };

    // Fall downwards rapidly
    let init_vel = SetAttributeModifier::new(
        Attribute::VELOCITY,
        module.lit(Vec3::new(-2.2, -38.0, -1.1)),
    );

    let init_lifetime = SetAttributeModifier::new(Attribute::LIFETIME, module.lit(0.72));

    let color_gradient = HanabiGradient::from_keys([
        (0.0, Vec4::new(0.85, 0.90, 0.98, 0.85)),
        (1.0, Vec4::new(0.65, 0.70, 0.78, 0.35)),
    ]);

    let size_gradient = HanabiGradient::constant(Vec3::new(0.02, 0.40, 0.02));

    let spawner = SpawnerSettings::rate(3800.0.into()).with_starts_active(false);

    let effect = EffectAsset::new(32768, spawner, module)
        .with_name("ambient_rain")
        .init(init_pos)
        .init(init_vel)
        .init(init_lifetime)
        .render(ColorOverLifetimeModifier {
            gradient: color_gradient,
            ..default()
        })
        .render(SizeOverLifetimeModifier {
            gradient: size_gradient,
            screen_space_size: false,
        });

    commands.insert_resource(RainEffect(effects.add(effect)));
}

fn setup_tornado_effect(mut commands: Commands, mut effects: ResMut<Assets<EffectAsset>>) {
    let mut module = Module::default();

    // Spawns in a cylinder at the base of the tornado
    let init_pos = SetPositionCircleModifier {
        center: module.lit(Vec3::ZERO),
        axis: module.lit(Vec3::Y),
        radius: module.lit(7.0),
        dimension: ShapeDimension::Volume,
    };

    // Shoots upward and outward
    let init_vel = SetVelocitySphereModifier {
        center: module.lit(Vec3::new(0.0, -3.5, 0.0)),
        speed: module.lit(9.0),
    };

    let init_lifetime = SetAttributeModifier::new(Attribute::LIFETIME, module.lit(2.8));

    // Dusty brown/grey particle gradient
    let color_gradient = HanabiGradient::from_keys([
        (0.0, Vec4::new(0.24, 0.22, 0.22, 0.52)),
        (0.5, Vec4::new(0.18, 0.16, 0.16, 0.35)),
        (1.0, Vec4::new(0.12, 0.10, 0.10, 0.0)),
    ]);

    // Grow significantly as it rises
    let size_gradient = HanabiGradient::from_keys([
        (0.0, Vec3::splat(0.4)),
        (0.5, Vec3::splat(2.5)),
        (1.0, Vec3::splat(5.5)),
    ]);

    let spawner = SpawnerSettings::rate(350.0.into()).with_starts_active(true);

    let effect = EffectAsset::new(8192, spawner, module)
        .with_name("tornado_dust")
        .init(init_pos)
        .init(init_vel)
        .init(init_lifetime)
        .render(ColorOverLifetimeModifier {
            gradient: color_gradient,
            ..default()
        })
        .render(SizeOverLifetimeModifier {
            gradient: size_gradient,
            screen_space_size: false,
        });

    commands.insert_resource(TornadoEffect(effects.add(effect)));
}

// Weather transition logic
fn transition_weather_system(
    time: Res<Time>,
    mut weather: ResMut<WeatherManager>,
    keyboard_input: Res<ButtonInput<KeyCode>>,
) {
    // Tick the weather duration timer
    weather.state_timer.tick(time.delta());

    let mut force_next = false;
    let mut force_storm = false;
    if keyboard_input.just_pressed(KeyCode::KeyK) {
        force_next = true;
        info!("Debug key pressed: forcing weather transition!");
    }
    if keyboard_input.just_pressed(KeyCode::KeyT) {
        force_storm = true;
        info!("Debug key pressed: forcing Stormy weather!");
    }

    if weather.state_timer.just_finished() || force_next || force_storm {
        let next_state = if force_storm {
            WeatherState::Stormy
        } else {
            let mut rng = rand::rng();
            // Randomly transition to next weather state (Sunny 40%, Cloudy 30%, Rainy 20%, Stormy 10%)
            let roll = rng.random_range(0..100);
            match roll {
                0..=39 => WeatherState::Sunny,
                40..=69 => WeatherState::Cloudy,
                70..=89 => WeatherState::Rainy,
                _ => WeatherState::Stormy,
            }
        };

        weather.target = next_state;

        // Random state duration between 30 and 60 seconds
        let mut rng = rand::rng();
        let duration = rng.random_range(30.0..60.0);
        weather
            .state_timer
            .set_duration(std::time::Duration::from_secs_f32(duration));
        weather.state_timer.reset();

        info!(
            "Weather transitioning from {:?} to {:?}",
            weather.current, next_state
        );
    }

    // Set targets
    let (target_cloudiness, target_wetness) = match weather.target {
        WeatherState::Sunny => (0.0, 0.0),
        WeatherState::Cloudy => (0.6, 0.0),
        WeatherState::Rainy => (0.8, 0.65),
        WeatherState::Stormy => (1.0, 1.0),
    };

    // Interpolate values smoothly
    let speed = 0.22 * time.delta_secs(); // Slow, natural transitions
    weather.cloudiness += (target_cloudiness - weather.cloudiness) * speed;
    weather.wetness += (target_wetness - weather.wetness) * speed;

    // Once transition is close, update current state
    if (weather.cloudiness - target_cloudiness).abs() < 0.05
        && (weather.wetness - target_wetness).abs() < 0.05
    {
        weather.current = weather.target;
    }
}

// Handle spawning and active state of rain particle system
fn manage_weather_particles(
    mut commands: Commands,
    player_query: Query<&Transform, With<Player>>,
    mut rain_spawner_query: Query<(Entity, &mut Transform), (With<RainSpawner>, Without<Player>)>,
    mut effect_spawner_query: Query<&mut EffectSpawner>,
    weather: Res<WeatherManager>,
    rain_effect: Option<Res<RainEffect>>,
) {
    let Some(rain) = rain_effect else { return };

    let Ok(player_transform) = player_query.single() else {
        return;
    };

    if rain_spawner_query.iter().count() == 0 {
        // Spawn the rain particle spawner attached to the player
        commands.spawn((
            Name::new("AmbientRainSpawner"),
            RainSpawner,
            ParticleEffect {
                handle: rain.0.clone(),
                ..default()
            },
            Transform::from_translation(player_transform.translation + Vec3::Y * 15.0),
            bevy::camera::visibility::NoFrustumCulling,
        ));
        return;
    }

    let Ok((spawner_entity, mut spawner_transform)) = rain_spawner_query.single_mut() else {
        return;
    };

    // Keep rain emitter centered on player XZ, but fixed high on Y
    spawner_transform.translation = Vec3::new(
        player_transform.translation.x,
        player_transform.translation.y + 16.0,
        player_transform.translation.z,
    );

    // Control rain spawner active state based on wetness
    if let Ok(mut spawner) = effect_spawner_query.get_mut(spawner_entity) {
        spawner.active = weather.wetness > 0.05;
    }
}

// Spawn, movement, sound loop and water interaction of tornadoes
fn spawn_and_update_tornadoes(
    mut commands: Commands,
    time: Res<Time>,
    weather: Res<WeatherManager>,
    mut tornado_query: Query<
        (Entity, &mut Transform, &mut Tornado),
        (
            With<Tornado>,
            Without<Player>,
            Without<Animal>,
            Without<NPC>,
            Without<TreeEntity>,
            Without<SettlementBuilding>,
            Without<
                bevy_voxel_world::prelude::VoxelWorldCamera<super::noise_generator::NoiseGenerator>,
            >,
        ),
    >,
    mut player_query: Query<
        (Entity, &mut Transform),
        (
            With<Player>,
            Without<Tornado>,
            Without<Animal>,
            Without<NPC>,
            Without<TreeEntity>,
            Without<SettlementBuilding>,
        ),
    >,
    tornado_effect: Option<Res<TornadoEffect>>,
    asset_server: Res<AssetServer>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut tornado_materials: ResMut<Assets<TornadoMaterial>>,
    mut voxel_world: VoxelWorld<NoiseGenerator>,
    tree_query: Query<
        (Entity, &Transform, &Health),
        (
            With<TreeEntity>,
            Without<Tornado>,
            Without<Player>,
            Without<Animal>,
            Without<NPC>,
            Without<SettlementBuilding>,
        ),
    >,
    mut creature_query: Query<
        (Entity, &mut Transform, Option<&Animal>, Option<&NPC>),
        (
            Or<(With<Animal>, With<NPC>)>,
            Without<Tornado>,
            Without<Player>,
            Without<TreeEntity>,
            Without<SettlementBuilding>,
        ),
    >,
    funnel_parts: Query<&MeshMaterial3d<TornadoMaterial>>,
    building_query: Query<
        (Entity, &Transform, &SettlementBuilding),
        (
            With<SettlementBuilding>,
            Without<Tornado>,
            Without<Player>,
            Without<Animal>,
            Without<NPC>,
            Without<TreeEntity>,
        ),
    >,
) {
    let Ok((player_entity, mut player_transform)) = player_query.single_mut() else {
        return;
    };
    let player_pos = player_transform.translation;

    // Manage Tornado Entity Lifetime based on Stormy weather
    if weather.current == WeatherState::Stormy || weather.target == WeatherState::Stormy {
        if tornado_query.iter().count() == 0 {
            let Some(effect) = tornado_effect else { return };

            // Spawn tornado at a random position offset from the player
            let mut rng = rand::rng();
            let angle = rng.random_range(0.0..2.0 * PI);
            let dist = rng.random_range(25.0..45.0);
            let spawn_pos = player_pos + Vec3::new(angle.cos() * dist, 0.0, angle.sin() * dist);
            let spawn_pos = Vec3::new(spawn_pos.x, 15.1, spawn_pos.z); // Center at water surface

            // Loop spatial sound effect using tornado_sound.wav
            let sound_entity = commands
                .spawn((
                    AudioPlayer::new(asset_server.load("tornado_sound.wav")),
                    PlaybackSettings {
                        mode: bevy::audio::PlaybackMode::Loop,
                        speed: 1.0, // Real tornado sound already has a deep rumble
                        ..default()
                    },
                    Transform::from_translation(spawn_pos),
                ))
                .id();

            // Parent sound and create tornado entity
            let mut tornado = commands.spawn((
                Name::new("StormTornado"),
                Tornado {
                    change_timer: Timer::from_seconds(5.0, TimerMode::Repeating),
                    movement_dir: Vec2::new(angle.cos(), angle.sin()),
                    sound_entity: Some(sound_entity),
                },
                WaterInteractor {
                    mass: 8.5, // Increased mass for massive water displacement!
                    last_position: spawn_pos,
                    is_player: false,
                },
                Transform::from_translation(spawn_pos),
                Visibility::default(),
                InheritedVisibility::default(),
            ));

            let tornado_id = tornado.id();

            // Attach Hanabi dust/debris emitter at the base of the tornado
            tornado.with_children(|parent| {
                parent.spawn((
                    ParticleEffect {
                        handle: effect.0.clone(),
                        ..default()
                    },
                    Transform::from_xyz(0.0, 0.0, 0.0),
                ));
            });

            // Construct the Volumetric Funnel Mesh Hierarchy
            // Create volumetric tornado material
            let funnel_mat = tornado_materials.add(TornadoMaterial {
                color: LinearRgba::new(0.12, 0.13, 0.16, 0.85),
                time: 0.0,
            });

            // Stack multiple rotating/wobbling cylinders to form a dynamic funnel
            let cylinder_mesh = meshes.add(Cylinder::new(1.0, 1.0));

            let segment_height = 4.5;
            let radii = [
                2.8, 3.5, 4.4, 5.5, 6.8, 8.2, 10.0, 12.0, 14.5, 17.2, 20.2, 23.5, 27.2, 31.2, 35.5,
                40.0,
            ];

            commands.entity(tornado_id).with_children(|parent| {
                for (i, &r) in radii.iter().enumerate() {
                    let y = (i as f32) * segment_height + segment_height / 2.0;

                    parent.spawn((
                        Mesh3d(cylinder_mesh.clone()),
                        MeshMaterial3d(funnel_mat.clone()),
                        Transform {
                            translation: Vec3::new(0.0, y, 0.0),
                            scale: Vec3::new(r, segment_height, r),
                            ..default()
                        },
                        TornadoFunnelPart {
                            base_scale: Vec3::new(r, segment_height, r),
                            height_offset: y,
                            spin_speed: 3.5 - (i as f32 * 0.2), // Lower parts spin faster!
                            phase: (i as f32) * 0.42,
                        },
                    ));
                }
            });

            info!("Tornado spawned at: {:?}", spawn_pos);
        } else {
            // Update existing tornado movement
            let Ok((_entity, mut transform, mut tornado)) = tornado_query.single_mut() else {
                return;
            };
            let mut pos = transform.translation;

            // Tick movement change direction timer
            tornado.change_timer.tick(time.delta());
            if tornado.change_timer.just_finished() {
                let mut rng = rand::rng();
                let angle = rng.random_range(0.0..2.0 * PI);
                tornado.movement_dir = Vec2::new(angle.cos(), angle.sin());
            }

            // Move tornado
            let speed = 6.5;
            pos.x += tornado.movement_dir.x * speed * time.delta_secs();
            pos.z += tornado.movement_dir.y * speed * time.delta_secs();

            // Constrain / steer tornado to stay within view of the player (keep distance < 150m)
            let to_player = player_pos - pos;
            let dist = to_player.length();
            if dist > 140.0 {
                let steer_dir = to_player.normalize();
                pos.x += steer_dir.x * speed * 1.5 * time.delta_secs();
                pos.z += steer_dir.z * speed * 1.5 * time.delta_secs();
            }

            // Lock Y to water level
            pos.y = 15.1;

            // Update translation and sound position
            transform.translation = pos;
            if let Some(sound_entity) = tornado.sound_entity {
                commands
                    .entity(sound_entity)
                    .insert(Transform::from_translation(pos));
            }

            // Animate shader time for the scrolling noise texture on the GPU
            let elapsed = time.elapsed_secs();
            for mat_handle in funnel_parts.iter() {
                if let Some(mat) = tornado_materials.get_mut(mat_handle) {
                    mat.time = elapsed;
                }
            }

            // ─────────────────────────────────────────────────────────────────────────
            // TORNADO DESTRUCTION AND PICKUP INTERACTIONS
            // ─────────────────────────────────────────────────────────────────────────

            // 1. Voxel Destruction: Destroy solid blocks (houses, walls, terrain) in path
            let tornado_pos_ivec = pos.as_ivec3();
            let destroy_radius = 4;
            for dx in -destroy_radius..=destroy_radius {
                for dz in -destroy_radius..=destroy_radius {
                    if dx * dx + dz * dz > destroy_radius * destroy_radius {
                        continue;
                    }
                    // Destroy a vertical column of blocks above the water level (Y=15)
                    for dy in 0..20 {
                        let block_pos =
                            IVec3::new(tornado_pos_ivec.x + dx, 15 + dy, tornado_pos_ivec.z + dz);
                        if let WorldVoxel::Solid(_) = voxel_world.get_voxel(block_pos) {
                            voxel_world.set_voxel(block_pos, WorldVoxel::Air);
                        }
                    }
                }
            }

            // 2. Tree Destruction: Deal continuous damage to nearby trees
            for (tree_entity, tree_transform, _tree_health) in tree_query.iter() {
                let to_tree = tree_transform.translation - pos;
                let dist_2d = Vec2::new(to_tree.x, to_tree.z).length();
                if dist_2d < 15.0 {
                    // Deal 100.0 damage per second (chopped down in 0.5s of contact)
                    commands.queue(SafeInsert {
                        entity: tree_entity,
                        component: DamageEvent(100.0 * time.delta_secs()),
                    });
                }
            }

            // 2.5 Building Destruction: Destroy nearby buildings (houses, shops, etc.)
            for (building_entity, building_transform, building) in building_query.iter() {
                let to_building = building_transform.translation - pos;
                let dist_2d = Vec2::new(to_building.x, to_building.z).length();
                if dist_2d < 24.0 {
                    commands
                        .entity(building_entity)
                        .despawn_related::<Children>();
                    commands.entity(building_entity).despawn();
                    info!(
                        "Building of type {:?} destroyed by tornado!",
                        building.building_type
                    );
                }
            }

            // 3. Picking up the Player
            let to_player = player_pos - pos;
            let dist_player_2d = Vec2::new(to_player.x, to_player.z).length();
            if dist_player_2d < 20.0 {
                let tangent = Vec3::new(-to_player.z, 0.0, to_player.x).normalize();
                let swirl_speed = 22.0;
                let lift_speed = 8.5;
                let pull_speed = -4.0;
                let velocity = tangent * swirl_speed
                    + Vec3::Y * lift_speed
                    + to_player.normalize() * pull_speed;
                player_transform.translation += velocity * time.delta_secs();

                // Deal light damage to the player
                commands.queue(SafeInsert {
                    entity: player_entity,
                    component: DamageEvent(3.0 * time.delta_secs()),
                });
            }

            // 4. Picking up creatures (Animals and NPCs)
            for (target_entity, mut target_transform, _opt_animal, _opt_npc) in
                creature_query.iter_mut()
            {
                let to_target = target_transform.translation - pos;
                let dist_2d = Vec2::new(to_target.x, to_target.z).length();
                if dist_2d < 20.0 {
                    let mut new_pos = target_transform.translation;

                    let angular_speed = 4.0 * time.delta_secs();
                    let relative_pos = target_transform.translation - pos;
                    let rotated_relative = Vec3::new(
                        relative_pos.x * angular_speed.cos() - relative_pos.z * angular_speed.sin(),
                        relative_pos.y,
                        relative_pos.x * angular_speed.sin() + relative_pos.z * angular_speed.cos(),
                    );

                    // Pull 6% closer to the center per frame
                    let pull_factor = 0.94;
                    new_pos.x = pos.x + rotated_relative.x * pull_factor;
                    new_pos.z = pos.z + rotated_relative.z * pull_factor;

                    // Lift up in the air
                    new_pos.y += 14.0 * time.delta_secs();

                    // Cap maximum height
                    new_pos.y = new_pos.y.min(pos.y + 45.0);

                    target_transform.translation = new_pos;

                    // Deal damage to creatures in the vortex
                    commands.queue(SafeInsert {
                        entity: target_entity,
                        component: DamageEvent(8.0 * time.delta_secs()),
                    });
                    commands.queue(SafeInsert {
                        entity: target_entity,
                        component: TornadoDamaged,
                    });
                }
            }
        }
    } else if tornado_query.iter().count() > 0 {
        // Despawn Tornado when storm is over
        for (entity, _, tornado) in tornado_query.iter() {
            commands.entity(entity).despawn_related::<Children>();
            commands.entity(entity).despawn();
            if let Some(sound_entity) = tornado.sound_entity {
                commands.entity(sound_entity).despawn();
            }
        }
        info!("Tornado despawned.");
    }
}

// Animate the stacked funnel cylinders to spin and wobble dynamically
fn animate_tornado_funnel(
    time: Res<Time>,
    mut part_query: Query<(&mut Transform, &TornadoFunnelPart)>,
) {
    let elapsed = time.elapsed_secs();

    for (mut transform, part) in part_query.iter_mut() {
        // Spin around Y axis
        transform.rotate_y(part.spin_speed * time.delta_secs());

        // Wobble translation left/right and front/back based on sine waves
        // Lower parts wobble less, higher parts sway violently
        let intensity = 1.0 + (part.height_offset * 0.08);
        let wobble_x = (elapsed * 2.2 + part.phase).sin() * 0.8 * intensity;
        let wobble_z = (elapsed * 1.6 + part.phase).cos() * 0.8 * intensity;

        transform.translation = Vec3::new(wobble_x, part.height_offset, wobble_z);

        // Perturb scale to look organic
        let pulse = 1.0 + (elapsed * 3.5 + part.phase).sin() * 0.12;
        transform.scale = part.base_scale * pulse;
    }
}

fn simulate_rain_impacts(
    mut commands: Commands,
    _time: Res<Time>,
    weather: Res<WeatherManager>,
    player_query: Query<&Transform, With<Player>>,
    noise_generator: Option<Res<super::noise_generator::NoiseGenerator>>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut local_assets: Local<Option<(Handle<Mesh>, Handle<StandardMaterial>)>>,
    mut impulse_writer: MessageWriter<WaterImpulseEvent>,
) {
    if weather.wetness <= 0.05 {
        return;
    }

    let Ok(player_transform) = player_query.single() else {
        return;
    };
    let Some(ref noise_gen) = noise_generator else {
        return;
    };

    let (splash_mesh, soil_mat) = local_assets
        .get_or_insert_with(|| {
            (
                meshes.add(Cuboid::from_size(Vec3::splat(0.016))),
                // Translucent grey/white impact splash for dry land
                materials.add(StandardMaterial {
                    base_color: Color::srgba(0.8, 0.84, 0.9, 0.30),
                    alpha_mode: AlphaMode::Blend,
                    ..default()
                }),
            )
        })
        .clone();

    let mut rng = rand::rng();

    // Scale impact density with wetness (between 2 and 8 splashes per frame)
    let count = (2.0 + weather.wetness * 6.0) as usize;
    let player_pos = player_transform.translation;

    for _ in 0..count {
        // Spawn randomly in a circle around the player
        let radius = rng.random_range(1.5..26.0);
        let angle = rng.random_range(0.0..std::f32::consts::TAU);
        let x = player_pos.x + angle.cos() * radius;
        let z = player_pos.z + angle.sin() * radius;

        // Get ground height from noise generator
        let land_h = noise_gen.get_adjusted_surface_height(x, z);
        let is_water = land_h < 15.0;
        let surface_h = land_h.max(15.0);
        if is_water {
            // 1. Water Ripple: Send impulse to GPU simulation!
            impulse_writer.write(WaterImpulseEvent {
                position: Vec3::new(x, surface_h, z),
                force: rng.random_range(0.8..2.2), // small ripples
                radius: rng.random_range(0.25..0.45),
            });
        } else {
            // 2. Ground splash (soil/grass)
            let p_pos = Vec3::new(x, surface_h + 0.05, z);
            let p_vel = Vec3::new(
                rng.random_range(-0.6..0.6),
                rng.random_range(0.9..2.4),
                rng.random_range(-0.6..0.6),
            );
            commands.spawn((
                Mesh3d(splash_mesh.clone()),
                MeshMaterial3d(soil_mat.clone()),
                Transform::from_translation(p_pos),
                crate::player::interaction::Particle {
                    velocity: p_vel,
                    lifetime: Timer::from_seconds(rng.random_range(0.14..0.3), TimerMode::Once),
                },
            ));
        }
    }
}
