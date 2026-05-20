use bevy::prelude::*;
use crate::world::tree_generator::TreeEntity;
use crate::player::camera::Player;
use crate::player::interaction::Inventory;
use crate::voxel::BlockType;
use crate::world::noise_generator::NoiseGenerator;
use bevy_voxel_world::prelude::*;
use crate::ui::UiState;

#[derive(Message, Default, Clone)]
pub struct LaserHitEvent {
    pub position: Vec3,
    pub _normal: Vec3,
}

#[derive(Component)]
pub struct LaserBeam;

pub struct CombatPlugin;

impl Plugin for CombatPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<WeaponState>();
        app.init_resource::<LaserHeat>();
        app.add_systems(Startup, setup_combat_audio);
        app.add_message::<LaserHitEvent>();
        app.add_systems(Update, (
            weapon_select,
            fire_bow,
            melee_attack,
            fire_laser,
            weapon_model_sync,
            projectile_update,
            health_death,
            update_laser_heat,
        ));
    }
}

/// Marker for the currently visible weapon model attached to camera
#[derive(Component)]
pub struct EquippedWeaponModel;

#[derive(Resource)]
pub struct LaserAudio {
    pub sound: Handle<AudioSource>,
    pub playing_entity: Option<Entity>,
}

#[derive(Resource, Default)]
pub struct LaserHeat {
    pub current: f32,
    pub overheated: bool,
}

fn setup_combat_audio(mut commands: Commands, asset_server: Res<AssetServer>) {
    commands.insert_resource(LaserAudio {
        sound: asset_server.load("laser_hum_final.wav"),
        playing_entity: None,
    });
}

/// Health component for any damageable entity
#[derive(Component)]
pub struct Health {
    pub hp: f32,
    pub max_hp: f32,
}

impl Health {
    pub fn new(max: f32) -> Self {
        Self { hp: max, max_hp: max }
    }
}

/// Active weapon selection
#[derive(Resource, Default, PartialEq, Eq, Clone, Copy, Debug)]
pub enum WeaponState {
    #[default]
    NoWeapon,
    Pickaxe,
    Axe,
    Sword,
    Bow,
    Laser,
}

/// A flying projectile (arrow)
#[derive(Component)]
pub struct Projectile {
    pub velocity: Vec3,
    pub damage: f32,
    pub lifetime: Timer,
}

/// Marker for entities that can be hit by projectiles
#[derive(Component)]
pub struct Hittable;

/// Pending damage marker
#[derive(Component)]
pub struct DamageEvent(pub f32);

/// Weapon selection system
fn weapon_select(
    keys: Res<ButtonInput<KeyCode>>,
    mut weapon: ResMut<WeaponState>,
    inventory: Res<Inventory>,
    gamepads: Query<&Gamepad>,
    ui_state: Res<UiState>,
) {
    if ui_state.show_inventory || ui_state.show_pause_menu {
        return;
    }

    if keys.just_pressed(KeyCode::Digit1) {
        if inventory.has_gold_pickaxe || inventory.has_iron_pickaxe || inventory.has_pickaxe {
            *weapon = WeaponState::Pickaxe;
        } else {
            *weapon = WeaponState::NoWeapon;
        }
    }
    if keys.just_pressed(KeyCode::Digit2) {
        if inventory.has_gold_axe || inventory.has_iron_axe || inventory.has_axe {
            *weapon = WeaponState::Axe;
        } else {
            *weapon = WeaponState::NoWeapon;
        }
    }
    if keys.just_pressed(KeyCode::Digit3) {
        if inventory.has_gold_sword || inventory.has_iron_sword || inventory.has_sword {
            *weapon = WeaponState::Sword;
        } else {
            *weapon = WeaponState::NoWeapon;
        }
    }
    if keys.just_pressed(KeyCode::Digit4) {
        *weapon = WeaponState::Laser;
    }
    if keys.just_pressed(KeyCode::Digit5) && inventory.has_bow {
        *weapon = WeaponState::Bow;
    }

    for gamepad in gamepads.iter() {
        if gamepad.just_pressed(GamepadButton::RightTrigger) {
            // Cycle weapons forward: NoWeapon -> Pickaxe -> Axe -> Sword -> Laser -> Bow -> NoWeapon
            let mut next = *weapon;
            loop {
                next = match next {
                    WeaponState::NoWeapon => WeaponState::Pickaxe,
                    WeaponState::Pickaxe => WeaponState::Axe,
                    WeaponState::Axe => WeaponState::Sword,
                    WeaponState::Sword => WeaponState::Laser,
                    WeaponState::Laser => WeaponState::Bow,
                    WeaponState::Bow => WeaponState::NoWeapon,
                };
                
                // Check if we have the selected weapon/tool
                match next {
                    WeaponState::NoWeapon | WeaponState::Laser => break,
                    WeaponState::Pickaxe => {
                        if inventory.has_gold_pickaxe || inventory.has_iron_pickaxe || inventory.has_pickaxe {
                            break;
                        }
                    }
                    WeaponState::Axe => {
                        if inventory.has_gold_axe || inventory.has_iron_axe || inventory.has_axe {
                            break;
                        }
                    }
                    WeaponState::Sword => {
                        if inventory.has_gold_sword || inventory.has_iron_sword || inventory.has_sword {
                            break;
                        }
                    }
                    WeaponState::Bow => {
                        if inventory.has_bow {
                            break;
                        }
                    }
                }
            }
            *weapon = next;
        }
        
        if gamepad.just_pressed(GamepadButton::LeftTrigger) {
            // Cycle weapons backward: NoWeapon -> Bow -> Laser -> Sword -> Axe -> Pickaxe -> NoWeapon
            let mut prev = *weapon;
            loop {
                prev = match prev {
                    WeaponState::NoWeapon => WeaponState::Bow,
                    WeaponState::Bow => WeaponState::Laser,
                    WeaponState::Laser => WeaponState::Sword,
                    WeaponState::Sword => WeaponState::Axe,
                    WeaponState::Axe => WeaponState::Pickaxe,
                    WeaponState::Pickaxe => WeaponState::NoWeapon,
                };
                
                // Check if we have the selected weapon/tool
                match prev {
                    WeaponState::NoWeapon | WeaponState::Laser => break,
                    WeaponState::Pickaxe => {
                        if inventory.has_gold_pickaxe || inventory.has_iron_pickaxe || inventory.has_pickaxe {
                            break;
                        }
                    }
                    WeaponState::Axe => {
                        if inventory.has_gold_axe || inventory.has_iron_axe || inventory.has_axe {
                            break;
                        }
                    }
                    WeaponState::Sword => {
                        if inventory.has_gold_sword || inventory.has_iron_sword || inventory.has_sword {
                            break;
                        }
                    }
                    WeaponState::Bow => {
                        if inventory.has_bow {
                            break;
                        }
                    }
                }
            }
            *weapon = prev;
        }
    }
}

fn fire_bow(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mouse_input: Res<ButtonInput<MouseButton>>,
    weapon: Res<WeaponState>,
    mut inventory: ResMut<Inventory>,
    player_query: Query<&Transform, With<Player>>,
    camera_query: Query<&GlobalTransform, With<Camera3d>>,
    ui_state: Res<UiState>,
    gamepads: Query<&Gamepad>,
) {
    if ui_state.show_inventory || ui_state.show_pause_menu { return; }
    
    let mut is_firing = mouse_input.just_pressed(MouseButton::Left);
    for gamepad in gamepads.iter() {
        if gamepad.just_pressed(GamepadButton::RightTrigger2) { is_firing = true; }
    }
    
    if *weapon != WeaponState::Bow || !is_firing { return; }

    let wood_count = inventory.resources.get(&BlockType::Wood).copied().unwrap_or(0);
    if wood_count == 0 { return; }

    *inventory.resources.entry(BlockType::Wood).or_insert(0) -= 1;

    if let Ok(player_transform) = player_query.single() {
        if let Ok(camera_transform) = camera_query.single() {
            let spawn_pos = player_transform.translation + Vec3::new(0.0, 1.5, 0.0);
            let forward = camera_transform.forward();
            let velocity = Vec3::from(forward) * 40.0;

            commands.spawn((
                Mesh3d(meshes.add(Cuboid::new(0.05, 0.05, 0.4))),
                MeshMaterial3d(materials.add(StandardMaterial { base_color: Color::srgb(0.4, 0.25, 0.1), ..default() })),
                Transform::from_translation(spawn_pos).looking_to(forward, Vec3::Y),
                Projectile {
                    velocity,
                    damage: 5.0,
                    lifetime: Timer::from_seconds(5.0, TimerMode::Once),
                },
            ));
        }
    }
}

fn projectile_update(
    mut commands: Commands,
    time: Res<Time>,
    mut projectiles: Query<(Entity, &mut Transform, &mut Projectile)>,
    hittable_query: Query<(Entity, &GlobalTransform, &Health), With<Hittable>>,
) {
    let dt = time.delta_secs();
    for (arrow_entity, mut arrow_transform, mut projectile) in projectiles.iter_mut() {
        projectile.lifetime.tick(time.delta());
        if projectile.lifetime.is_finished() {
            commands.entity(arrow_entity).despawn();
            continue;
        }

        projectile.velocity.y -= 9.8 * dt;
        arrow_transform.translation += projectile.velocity * dt;

        for (target_entity, target_transform, _) in hittable_query.iter() {
            // Increased hit radius to 2.0 to account for enemy height (origin is at their feet)
            if arrow_transform.translation.distance(target_transform.translation()) < 2.0 {
                if let Ok(mut cmd) = commands.get_entity(target_entity) {
                    cmd.insert(DamageEvent(projectile.damage));
                }
                commands.entity(arrow_entity).despawn();
                break;
            }
        }
    }
}

fn health_death(
    mut commands: Commands,
    mut query: Query<(Entity, &mut Health, &DamageEvent, Option<&Player>, Option<&mut Transform>, Option<&TreeEntity>)>,
    mut inventory: ResMut<Inventory>,
) {
    for (entity, mut health, damage, is_player, mut opt_transform, is_tree) in query.iter_mut() {
        health.hp -= damage.0;
        commands.entity(entity).remove::<DamageEvent>();

        if health.hp <= 0.0 {
            if is_player.is_some() {
                // Respawn player instead of despawning (which would destroy the VoxelWorldCamera)
                health.hp = health.max_hp;
                if let Some(ref mut transform) = opt_transform {
                    transform.translation = Vec3::new(0.0, 150.0, 0.0);
                }
            } else {
                if is_tree.is_some() {
                    // Tree dies, drop wood
                    *inventory.resources.entry(BlockType::Wood).or_insert(0) += 5;
                    println!("Tree chopped down! Gained 5 Wood. (Total: {})", inventory.resources[&BlockType::Wood]);
                }
                commands.entity(entity).despawn();
            }
        }
    }
}

fn weapon_model_sync(
    mut commands: Commands,
    weapon: Res<WeaponState>,
    inventory: Res<Inventory>,
    model_query: Query<Entity, With<EquippedWeaponModel>>,
    camera_query: Query<Entity, With<Camera3d>>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    if weapon.is_changed() {
        for entity in model_query.iter() {
            commands.entity(entity).despawn();
        }

        if let Ok(camera) = camera_query.single() {
            let model = match *weapon {
                WeaponState::Pickaxe => {
                    let color = if inventory.has_gold_pickaxe { Color::srgb(1.0, 0.84, 0.0) } else { Color::srgb(0.5, 0.35, 0.05) };
                    Some(commands.spawn((
                        EquippedWeaponModel,
                        Mesh3d(meshes.add(Cone { radius: 0.1, height: 0.6 })),
                        MeshMaterial3d(materials.add(StandardMaterial { base_color: color, ..default() })),
                        Transform::from_xyz(0.5, -0.4, -0.6).with_rotation(Quat::from_rotation_x(1.5)),
                    )).id())
                },
                WeaponState::Sword => {
                    Some(commands.spawn((
                        EquippedWeaponModel,
                        Mesh3d(meshes.add(Cuboid::new(0.1, 0.9, 0.05))),
                        MeshMaterial3d(materials.add(StandardMaterial { base_color: Color::WHITE, ..default() })),
                        Transform::from_xyz(0.5, -0.2, -0.7).with_rotation(Quat::from_rotation_z(0.2)),
                    )).id())
                },
                WeaponState::Laser => {
                    Some(commands.spawn((
                        EquippedWeaponModel,
                        Mesh3d(meshes.add(Cylinder::new(0.08, 0.5))),
                        MeshMaterial3d(materials.add(StandardMaterial { 
                            base_color: Color::srgb(0.0, 0.5, 0.5),
                            emissive: LinearRgba::from(Color::srgb(0.0, 2.0, 2.0)),
                            ..default() 
                        })),
                        Transform::from_xyz(0.5, -0.3, -0.4).with_rotation(Quat::from_rotation_x(1.5)),
                    )).id())
                },
                _ => None,
            };

            if let Some(m) = model {
                commands.entity(camera).add_child(m);
            }
        }
    }
}

fn melee_attack(
    mut commands: Commands,
    mouse_input: Res<ButtonInput<MouseButton>>,
    weapon: Res<WeaponState>,
    camera_query: Query<&GlobalTransform, With<Camera3d>>,
    hittable_query: Query<(Entity, &GlobalTransform), With<Hittable>>,
    ui_state: Res<UiState>,
    gamepads: Query<&Gamepad>,
) {
    let mut is_attacking = mouse_input.just_pressed(MouseButton::Left);
    for gamepad in gamepads.iter() {
        if gamepad.just_pressed(GamepadButton::RightTrigger2) { is_attacking = true; }
    }

    if !is_attacking || ui_state.show_inventory || ui_state.show_pause_menu { return; }

    let damage = match *weapon {
        WeaponState::Sword => 10.0,
        WeaponState::Pickaxe | WeaponState::Axe => 5.0,
        _ => 2.0,
    };

    if let Ok(camera_transform) = camera_query.single() {
        let shoot_pos = camera_transform.translation();
        let forward = camera_transform.forward();

        for (target_entity, target_transform) in hittable_query.iter() {
            let to_target = target_transform.translation() - shoot_pos;
            // Increased melee range to 5.0 and widened cone to 0.5 (~60 degrees) 
            // so looking at their torso still hits them even if their origin is at their feet.
            if to_target.length() < 5.0 && Vec3::from(forward).dot(to_target.normalize()) > 0.5 {
                commands.entity(target_entity).insert(DamageEvent(damage));
            }
        }
    }
}

pub fn fire_laser(
    mouse_input: Res<ButtonInput<MouseButton>>,
    weapon: Res<WeaponState>,
    mut commands: Commands,
    camera_query: Query<(&GlobalTransform, &Camera3d)>,
    mut voxel_world: VoxelWorld<NoiseGenerator>,
    mut hittable_query: Query<(Entity, &GlobalTransform, &mut Health), (With<Hittable>, Without<DamageEvent>, Without<Player>)>,
    mut hit_events: MessageWriter<LaserHitEvent>,
    time: Res<Time>,
    ui_state: Res<UiState>,
    mut beam_query: Query<(Entity, &mut Transform), With<LaserBeam>>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut laser_audio: ResMut<LaserAudio>,
    laser_heat: Res<LaserHeat>,
    gamepads: Query<&Gamepad>,
) {
    let mut is_firing = mouse_input.pressed(MouseButton::Left);
    for gamepad in gamepads.iter() {
        if gamepad.pressed(GamepadButton::RightTrigger2) { is_firing = true; }
    }

    if !is_firing || *weapon != WeaponState::Laser || ui_state.show_inventory || ui_state.show_pause_menu || laser_heat.overheated {
        for (entity, _) in beam_query.iter() {
            commands.entity(entity).despawn();
        }
        if let Some(audio_entity) = laser_audio.playing_entity {
            commands.entity(audio_entity).despawn();
            laser_audio.playing_entity = None;
        }
        return;
    }

    if laser_audio.playing_entity.is_none() {
        println!("LASER AUDIO STARTING...");
        laser_audio.playing_entity = Some(commands.spawn((
            AudioPlayer::new(laser_audio.sound.clone()),
            PlaybackSettings::LOOP, // Default volume to avoid compilation error
        )).id());
    }

    if let Ok((camera_transform, _)) = camera_query.single() {
        let shoot_pos = camera_transform.translation();
        let forward = camera_transform.forward();
        
        // Offset the beam start point to the player's right hand position
        let right = camera_transform.right();
        let hand_offset = Vec3::from(right) * 0.5 + Vec3::from(camera_transform.down()) * 0.3 + Vec3::from(forward) * 0.5;
        let beam_start = shoot_pos + hand_offset;
        
        let mut hit_pos = shoot_pos + Vec3::from(forward) * 25.0;

        for (target_entity, target_transform, mut health) in hittable_query.iter_mut() {
            let to_target = target_transform.translation() - shoot_pos;
            let forward_vec = Vec3::from(forward);
            let t = to_target.dot(forward_vec);
            
            // If the target is in front of the player and within 25 meters
            if t > 0.0 && t < 25.0 {
                let closest_point = shoot_pos + forward_vec * t;
                
                // Height-aware Cylinder check: hit anything within 1.5m radius and 5.0m height
                let horizontal_dist = Vec2::new(target_transform.translation().x, target_transform.translation().z)
                    .distance(Vec2::new(closest_point.x, closest_point.z));
                let vertical_diff = closest_point.y - target_transform.translation().y;

                if horizontal_dist < 1.5 && vertical_diff >= 0.0 && vertical_diff < 15.0 {
                    health.hp -= 40.0 * time.delta_secs(); // Increased damage for faster tree chopping
                    hit_pos = closest_point; // Set hit point to the point on the ray
                    hit_events.write(LaserHitEvent { position: hit_pos, _normal: -forward_vec });
                    if health.hp <= 0.0 { 
                        // Death is handled in health_death now, but we add DamageEvent to trigger it
                        commands.entity(target_entity).insert(DamageEvent(health.hp.abs() + 1.0));
                    }
                }
            }
        }
        
        let ray = Ray3d::new(shoot_pos, forward);
        if let Some(hit) = voxel_world.raycast(ray, &|(_, v): (Vec3, WorldVoxel)| v.is_solid()) {
            hit_pos = hit.position;
            hit_events.write(LaserHitEvent { position: hit_pos, _normal: hit.normal.unwrap_or(Vec3::Y) });
            voxel_world.set_voxel(hit.voxel_pos(), WorldVoxel::Air);
        }

        let beam_len = beam_start.distance(hit_pos);
        let beam_center = beam_start + Vec3::from(forward) * (beam_len * 0.5);
        
        if let Ok((_, mut transform)) = beam_query.single_mut() {
            transform.translation = beam_center;
            transform.scale = Vec3::new(1.0, beam_len, 1.0);
            transform.look_to(forward, Vec3::Y);
            transform.rotate_local_x(std::f32::consts::FRAC_PI_2);
        } else {
            commands.spawn((
                LaserBeam,
                Mesh3d(meshes.add(Cylinder::new(0.05, 1.0))),
                MeshMaterial3d(materials.add(StandardMaterial {
                    base_color: Color::NONE,
                    emissive: LinearRgba::from(Color::srgb(0.0, 5.0, 5.0)),
                    ..default()
                })),
                Transform::from_translation(beam_center).with_scale(Vec3::new(1.0, beam_len, 1.0)),
            ));
        }
    }
}

fn update_laser_heat(
    time: Res<Time>,
    mouse_input: Res<ButtonInput<MouseButton>>,
    weapon: Res<WeaponState>,
    mut laser_heat: ResMut<LaserHeat>,
    gamepads: Query<&Gamepad>,
) {
    let dt = time.delta_secs();
    let mut is_firing = mouse_input.pressed(MouseButton::Left);
    for gamepad in gamepads.iter() {
        if gamepad.pressed(GamepadButton::RightTrigger2) { is_firing = true; }
    }
    
    if *weapon == WeaponState::Laser && is_firing && !laser_heat.overheated {
        // Heating up - slower rate (approx. 6.7 seconds before overheating)
        laser_heat.current += 15.0 * dt;
        if laser_heat.current >= 100.0 {
            laser_heat.overheated = true;
            laser_heat.current = 100.0;
        }
    } else {
        // Cooling down - faster rate (approx. 4.0 seconds to fully cool down)
        laser_heat.current -= 25.0 * dt;
        if laser_heat.current <= 0.0 {
            laser_heat.current = 0.0;
            laser_heat.overheated = false;
        }
    }
}
