use crate::player::camera::{CameraMode, Player};
use crate::player::interaction::Inventory;
use crate::ui::UiState;
use crate::voxel::BlockType;
use crate::world::noise_generator::NoiseGenerator;
use crate::world::tree_generator::TreeEntity;
use bevy::prelude::*;
use bevy_voxel_world::prelude::*;

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
        app.init_resource::<GunCooldowns>();
        app.init_resource::<RecoilState>();
        app.init_resource::<AmmoState>();
        app.add_systems(Startup, setup_combat_audio);
        app.add_message::<LaserHitEvent>();
        app.add_systems(
            Update,
            (
                weapon_select,
                fire_bow,
                melee_attack,
                fire_laser,
                fire_guns,
                update_recoil,
                update_weapon_sway,
                weapon_model_sync,
                update_weapon_visibilities,
                projectile_update,
                health_death,
                update_laser_heat,
            ),
        );
    }
}

/// Marker for any visible weapon model (both first and third person)
#[derive(Component)]
pub struct EquippedWeaponModel;

/// Marker for first-person visible weapon model attached to camera
#[derive(Component)]
pub struct FirstPersonWeaponModel;

/// Marker for third-person visible weapon model attached to player right hand
#[derive(Component)]
pub struct ThirdPersonWeaponModel;

#[derive(Component)]
pub struct WeaponSway {
    pub current_offset: Vec3,
    pub target_offset: Vec3,
    pub current_rotation: Quat,
    pub target_rotation: Quat,
}

impl Default for WeaponSway {
    fn default() -> Self {
        Self {
            current_offset: Vec3::ZERO,
            target_offset: Vec3::ZERO,
            current_rotation: Quat::IDENTITY,
            target_rotation: Quat::IDENTITY,
        }
    }
}

#[derive(Resource, Default)]
pub struct GunCooldowns {
    pub pistol_timer: f32,
    pub revolver_timer: f32,
    pub rifle_timer: f32,
    pub sniper_timer: f32,
}

#[derive(Resource, Default)]
pub struct RecoilState {
    pub amount: f32,
    pub current: f32,
}

#[derive(Resource)]
pub struct LaserAudio {
    pub sound: Handle<AudioSource>,
    pub playing_entity: Option<Entity>,
}

#[derive(Resource)]
pub struct GunSounds {
    pub pistol_shoot: Handle<AudioSource>,
    pub revolver_shoot: Handle<AudioSource>,
    pub rifle_shoot: Handle<AudioSource>,
    pub sniper_shoot: Handle<AudioSource>,
    pub reload: Handle<AudioSource>,
}

#[derive(Resource, Clone, serde::Serialize, serde::Deserialize)]
pub struct AmmoState {
    pub pistol_ammo: u32,
    pub revolver_ammo: u32,
    pub rifle_ammo: u32,
    pub sniper_ammo: u32,
    pub reload_timer: f32,
    pub reloading_weapon: Option<WeaponState>,
}

impl Default for AmmoState {
    fn default() -> Self {
        Self {
            pistol_ammo: 12,
            revolver_ammo: 6,
            rifle_ammo: 30,
            sniper_ammo: 5,
            reload_timer: 0.0,
            reloading_weapon: None,
        }
    }
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
    commands.insert_resource(GunSounds {
        pistol_shoot: asset_server.load("pistol_shoot.wav"),
        revolver_shoot: asset_server.load("revolver_shoot.wav"),
        rifle_shoot: asset_server.load("rifle_shoot.wav"),
        sniper_shoot: asset_server.load("sniper_shoot.wav"),
        reload: asset_server.load("gun_reload.wav"),
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
        Self {
            hp: max,
            max_hp: max,
        }
    }
}

/// Active weapon selection
#[derive(
    Resource, Default, PartialEq, Eq, Clone, Copy, Debug, serde::Serialize, serde::Deserialize,
)]
pub enum WeaponState {
    #[default]
    NoWeapon,
    Pickaxe,
    Axe,
    Sword,
    Bow,
    Laser,
    Pistol,
    Revolver,
    Rifle,
    Sniper,
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
    if keys.just_pressed(KeyCode::Digit2)
        && (inventory.has_gold_axe || inventory.has_iron_axe || inventory.has_axe)
    {
        *weapon = WeaponState::NoWeapon;
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
    if keys.just_pressed(KeyCode::Digit6) {
        *weapon = WeaponState::Pistol;
    }
    if keys.just_pressed(KeyCode::Digit7) {
        *weapon = WeaponState::Revolver;
    }
    if keys.just_pressed(KeyCode::Digit8) {
        *weapon = WeaponState::Rifle;
    }
    if keys.just_pressed(KeyCode::Digit9) {
        *weapon = WeaponState::Sniper;
    }

    for gamepad in gamepads.iter() {
        if gamepad.just_pressed(GamepadButton::RightTrigger) {
            // Cycle weapons forward: NoWeapon -> Pickaxe -> Axe -> Sword -> Laser -> Bow -> Pistol -> Revolver -> Rifle -> Sniper -> NoWeapon
            let mut next = *weapon;
            loop {
                next = match next {
                    WeaponState::NoWeapon => WeaponState::Pickaxe,
                    WeaponState::Pickaxe => WeaponState::Axe,
                    WeaponState::Axe => WeaponState::Sword,
                    WeaponState::Sword => WeaponState::Laser,
                    WeaponState::Laser => WeaponState::Bow,
                    WeaponState::Bow => WeaponState::Pistol,
                    WeaponState::Pistol => WeaponState::Revolver,
                    WeaponState::Revolver => WeaponState::Rifle,
                    WeaponState::Rifle => WeaponState::Sniper,
                    WeaponState::Sniper => WeaponState::NoWeapon,
                };

                // Check if we have the selected weapon/tool
                match next {
                    WeaponState::NoWeapon
                    | WeaponState::Laser
                    | WeaponState::Pistol
                    | WeaponState::Revolver
                    | WeaponState::Rifle
                    | WeaponState::Sniper => break,
                    WeaponState::Pickaxe => {
                        if inventory.has_gold_pickaxe
                            || inventory.has_iron_pickaxe
                            || inventory.has_pickaxe
                        {
                            break;
                        }
                    }
                    WeaponState::Axe => {
                        if inventory.has_gold_axe || inventory.has_iron_axe || inventory.has_axe {
                            break;
                        }
                    }
                    WeaponState::Sword => {
                        if inventory.has_gold_sword
                            || inventory.has_iron_sword
                            || inventory.has_sword
                        {
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
            // Cycle weapons backward: NoWeapon -> Sniper -> Rifle -> Revolver -> Pistol -> Bow -> Laser -> Sword -> Axe -> Pickaxe -> NoWeapon
            let mut prev = *weapon;
            loop {
                prev = match prev {
                    WeaponState::NoWeapon => WeaponState::Sniper,
                    WeaponState::Sniper => WeaponState::Rifle,
                    WeaponState::Rifle => WeaponState::Revolver,
                    WeaponState::Revolver => WeaponState::Pistol,
                    WeaponState::Pistol => WeaponState::Bow,
                    WeaponState::Bow => WeaponState::Laser,
                    WeaponState::Laser => WeaponState::Sword,
                    WeaponState::Sword => WeaponState::Axe,
                    WeaponState::Axe => WeaponState::Pickaxe,
                    WeaponState::Pickaxe => WeaponState::NoWeapon,
                };

                // Check if we have the selected weapon/tool
                match prev {
                    WeaponState::NoWeapon
                    | WeaponState::Laser
                    | WeaponState::Pistol
                    | WeaponState::Revolver
                    | WeaponState::Rifle
                    | WeaponState::Sniper => break,
                    WeaponState::Pickaxe => {
                        if inventory.has_gold_pickaxe
                            || inventory.has_iron_pickaxe
                            || inventory.has_pickaxe
                        {
                            break;
                        }
                    }
                    WeaponState::Axe => {
                        if inventory.has_gold_axe || inventory.has_iron_axe || inventory.has_axe {
                            break;
                        }
                    }
                    WeaponState::Sword => {
                        if inventory.has_gold_sword
                            || inventory.has_iron_sword
                            || inventory.has_sword
                        {
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
    voxel_world: VoxelWorld<NoiseGenerator>,
) {
    if ui_state.show_inventory || ui_state.show_pause_menu {
        return;
    }

    let mut is_firing = mouse_input.just_pressed(MouseButton::Left);
    for gamepad in gamepads.iter() {
        if gamepad.just_pressed(GamepadButton::RightTrigger2) {
            is_firing = true;
        }
    }

    if *weapon != WeaponState::Bow || !is_firing {
        return;
    }

    let wood_count = inventory
        .resources
        .get(&BlockType::Wood)
        .copied()
        .unwrap_or(0);
    if wood_count == 0 {
        return;
    }

    *inventory.resources.entry(BlockType::Wood).or_insert(0) -= 1;

    if let Ok(player_transform) = player_query.single()
        && let Ok(camera_transform) = camera_query.single()
    {
        let spawn_pos = player_transform.translation + Vec3::new(0.0, 1.5, 0.0);
        let camera_pos = camera_transform.translation();
        let camera_forward = camera_transform.forward();

        let ray = Ray3d::new(camera_pos, camera_forward);
        let target_point = if let Some(hit) =
            voxel_world.raycast(ray, &|(_, v): (Vec3, WorldVoxel)| v.is_solid())
        {
            hit.position
        } else {
            camera_pos + Vec3::from(camera_forward) * 100.0
        };

        let shoot_dir = (target_point - spawn_pos).normalize_or_zero();
        let velocity = shoot_dir * 40.0;

        commands.spawn((
            Mesh3d(meshes.add(Cuboid::new(0.05, 0.05, 0.4))),
            MeshMaterial3d(materials.add(StandardMaterial {
                base_color: Color::srgb(0.4, 0.25, 0.1),
                ..default()
            })),
            Transform::from_translation(spawn_pos).looking_to(shoot_dir, Vec3::Y),
            Projectile {
                velocity,
                damage: 5.0,
                lifetime: Timer::from_seconds(5.0, TimerMode::Once),
            },
        ));
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
            if arrow_transform
                .translation
                .distance(target_transform.translation())
                < 2.0
            {
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
    mut query: Query<(
        Entity,
        &mut Health,
        &DamageEvent,
        Option<&Player>,
        Option<&mut Transform>,
        Option<&TreeEntity>,
    )>,
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
                    println!(
                        "Tree chopped down! Gained 5 Wood. (Total: {})",
                        inventory.resources[&BlockType::Wood]
                    );
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
    arm_query: Query<(Entity, &crate::player::camera::PlayerArm)>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    asset_server: Res<AssetServer>,
) {
    if weapon.is_changed() {
        for entity in model_query.iter() {
            commands.entity(entity).despawn_related::<Children>();
            commands.entity(entity).despawn();
        }

        let camera = camera_query.single().ok();
        let right_arm = arm_query
            .iter()
            .find(|(_, arm)| arm.side > 0.0)
            .map(|(e, _)| e);

        match *weapon {
            WeaponState::Pickaxe => {
                let color = if inventory.has_gold_pickaxe {
                    Color::srgb(1.0, 0.84, 0.0)
                } else {
                    Color::srgb(0.5, 0.35, 0.05)
                };
                let pick_mesh = meshes.add(Cone {
                    radius: 0.1,
                    height: 0.6,
                });
                let pick_mat = materials.add(StandardMaterial {
                    base_color: color,
                    ..default()
                });

                if let Some(cam) = camera {
                    let fp = commands
                        .spawn((
                            EquippedWeaponModel,
                            FirstPersonWeaponModel,
                            Mesh3d(pick_mesh.clone()),
                            MeshMaterial3d(pick_mat.clone()),
                            Transform::from_xyz(0.35, -0.3, -0.5)
                                .with_rotation(Quat::from_rotation_x(1.5)),
                            Visibility::default(),
                            InheritedVisibility::default(),
                        ))
                        .id();
                    commands.entity(cam).add_child(fp);
                }

                if let Some(arm) = right_arm {
                    let tp = commands
                        .spawn((
                            EquippedWeaponModel,
                            ThirdPersonWeaponModel,
                            Mesh3d(pick_mesh),
                            MeshMaterial3d(pick_mat),
                            Transform::from_xyz(0.0, -0.6, 0.0)
                                .with_rotation(Quat::from_rotation_x(1.5)),
                            Visibility::default(),
                            InheritedVisibility::default(),
                        ))
                        .id();
                    commands.entity(arm).add_child(tp);
                }
            }
            WeaponState::Sword => {
                let sword_mesh = meshes.add(Cuboid::new(0.08, 0.8, 0.04));
                let sword_mat = materials.add(StandardMaterial {
                    base_color: Color::WHITE,
                    ..default()
                });

                if let Some(cam) = camera {
                    let fp = commands
                        .spawn((
                            EquippedWeaponModel,
                            FirstPersonWeaponModel,
                            Mesh3d(sword_mesh.clone()),
                            MeshMaterial3d(sword_mat.clone()),
                            Transform::from_xyz(0.35, -0.2, -0.55)
                                .with_rotation(Quat::from_rotation_z(0.2)),
                            Visibility::default(),
                            InheritedVisibility::default(),
                        ))
                        .id();
                    commands.entity(cam).add_child(fp);
                }

                if let Some(arm) = right_arm {
                    let tp = commands
                        .spawn((
                            EquippedWeaponModel,
                            ThirdPersonWeaponModel,
                            Mesh3d(sword_mesh),
                            MeshMaterial3d(sword_mat),
                            Transform::from_xyz(0.0, -0.6, 0.0)
                                .with_rotation(Quat::from_rotation_x(1.5)),
                            Visibility::default(),
                            InheritedVisibility::default(),
                        ))
                        .id();
                    commands.entity(arm).add_child(tp);
                }
            }
            WeaponState::Laser => {
                let laser_mesh = meshes.add(Cylinder::new(0.08, 0.5));
                let laser_mat = materials.add(StandardMaterial {
                    base_color: Color::srgb(0.0, 0.5, 0.5),
                    emissive: LinearRgba::from(Color::srgb(0.0, 2.0, 2.0)),
                    ..default()
                });

                if let Some(cam) = camera {
                    let fp = commands
                        .spawn((
                            EquippedWeaponModel,
                            FirstPersonWeaponModel,
                            Mesh3d(laser_mesh.clone()),
                            MeshMaterial3d(laser_mat.clone()),
                            Transform::from_xyz(0.35, -0.25, -0.45)
                                .with_rotation(Quat::from_rotation_x(1.5)),
                            Visibility::default(),
                            InheritedVisibility::default(),
                        ))
                        .id();
                    commands.entity(cam).add_child(fp);
                }

                if let Some(arm) = right_arm {
                    let tp = commands
                        .spawn((
                            EquippedWeaponModel,
                            ThirdPersonWeaponModel,
                            Mesh3d(laser_mesh),
                            MeshMaterial3d(laser_mat),
                            Transform::from_xyz(0.0, -0.6, 0.0)
                                .with_rotation(Quat::from_rotation_x(1.5)),
                            Visibility::default(),
                            InheritedVisibility::default(),
                        ))
                        .id();
                    commands.entity(arm).add_child(tp);
                }
            }
            WeaponState::Pistol => {
                let gltf_scene = asset_server.load("Gun_Pistol.gltf#Scene0");

                if let Some(cam) = camera {
                    let fp = commands
                        .spawn((
                            EquippedWeaponModel,
                            FirstPersonWeaponModel,
                            Transform::from_xyz(0.2, -0.16, -0.45)
                                .with_rotation(Quat::from_rotation_y(-std::f32::consts::FRAC_PI_2)),
                            Visibility::default(),
                            InheritedVisibility::default(),
                        ))
                        .with_children(|p| {
                            p.spawn((SceneRoot(gltf_scene.clone()), WeaponSway::default()));
                        })
                        .id();
                    commands.entity(cam).add_child(fp);
                }

                if let Some(arm) = right_arm {
                    let tp = commands
                        .spawn((
                            EquippedWeaponModel,
                            ThirdPersonWeaponModel,
                            Transform::from_xyz(0.0, -0.6, 0.0)
                                .with_rotation(Quat::from_rotation_y(-std::f32::consts::FRAC_PI_2)),
                            Visibility::default(),
                            InheritedVisibility::default(),
                        ))
                        .with_children(|p| {
                            p.spawn(SceneRoot(gltf_scene));
                        })
                        .id();
                    commands.entity(arm).add_child(tp);
                }
            }
            WeaponState::Revolver => {
                let gltf_scene = asset_server.load("Gun_Revolver.gltf#Scene0");

                if let Some(cam) = camera {
                    let fp = commands
                        .spawn((
                            EquippedWeaponModel,
                            FirstPersonWeaponModel,
                            Transform::from_xyz(0.2, -0.16, -0.45)
                                .with_rotation(Quat::from_rotation_y(-std::f32::consts::FRAC_PI_2)),
                            Visibility::default(),
                            InheritedVisibility::default(),
                        ))
                        .with_children(|p| {
                            p.spawn((SceneRoot(gltf_scene.clone()), WeaponSway::default()));
                        })
                        .id();
                    commands.entity(cam).add_child(fp);
                }

                if let Some(arm) = right_arm {
                    let tp = commands
                        .spawn((
                            EquippedWeaponModel,
                            ThirdPersonWeaponModel,
                            Transform::from_xyz(0.0, -0.6, 0.0)
                                .with_rotation(Quat::from_rotation_y(-std::f32::consts::FRAC_PI_2)),
                            Visibility::default(),
                            InheritedVisibility::default(),
                        ))
                        .with_children(|p| {
                            p.spawn(SceneRoot(gltf_scene));
                        })
                        .id();
                    commands.entity(arm).add_child(tp);
                }
            }
            WeaponState::Rifle => {
                let gltf_scene = asset_server.load("Gun_Rifle.gltf#Scene0");

                if let Some(cam) = camera {
                    let fp = commands
                        .spawn((
                            EquippedWeaponModel,
                            FirstPersonWeaponModel,
                            Transform::from_xyz(0.2, -0.18, -0.5)
                                .with_rotation(Quat::from_rotation_y(-std::f32::consts::FRAC_PI_2)),
                            Visibility::default(),
                            InheritedVisibility::default(),
                        ))
                        .with_children(|p| {
                            p.spawn((SceneRoot(gltf_scene.clone()), WeaponSway::default()));
                        })
                        .id();
                    commands.entity(cam).add_child(fp);
                }

                if let Some(arm) = right_arm {
                    let tp = commands
                        .spawn((
                            EquippedWeaponModel,
                            ThirdPersonWeaponModel,
                            Transform::from_xyz(0.0, -0.6, 0.0)
                                .with_rotation(Quat::from_rotation_y(-std::f32::consts::FRAC_PI_2)),
                            Visibility::default(),
                            InheritedVisibility::default(),
                        ))
                        .with_children(|p| {
                            p.spawn(SceneRoot(gltf_scene));
                        })
                        .id();
                    commands.entity(arm).add_child(tp);
                }
            }
            WeaponState::Sniper => {
                let gltf_scene = asset_server.load("Gun_Sniper.gltf#Scene0");

                if let Some(cam) = camera {
                    let fp = commands
                        .spawn((
                            EquippedWeaponModel,
                            FirstPersonWeaponModel,
                            Transform::from_xyz(0.2, -0.18, -0.55)
                                .with_rotation(Quat::from_rotation_y(-std::f32::consts::FRAC_PI_2)),
                            Visibility::default(),
                            InheritedVisibility::default(),
                        ))
                        .with_children(|p| {
                            p.spawn((SceneRoot(gltf_scene.clone()), WeaponSway::default()));
                        })
                        .id();
                    commands.entity(cam).add_child(fp);
                }

                if let Some(arm) = right_arm {
                    let tp = commands
                        .spawn((
                            EquippedWeaponModel,
                            ThirdPersonWeaponModel,
                            Transform::from_xyz(0.0, -0.6, 0.0)
                                .with_rotation(Quat::from_rotation_y(-std::f32::consts::FRAC_PI_2)),
                            Visibility::default(),
                            InheritedVisibility::default(),
                        ))
                        .with_children(|p| {
                            p.spawn(SceneRoot(gltf_scene));
                        })
                        .id();
                    commands.entity(arm).add_child(tp);
                }
            }
            _ => {}
        }
    }
}

fn update_weapon_visibilities(
    player_query: Query<&CameraMode, With<Player>>,
    mut fp_query: Query<
        &mut Visibility,
        (
            With<FirstPersonWeaponModel>,
            Without<ThirdPersonWeaponModel>,
        ),
    >,
    mut tp_query: Query<
        &mut Visibility,
        (
            With<ThirdPersonWeaponModel>,
            Without<FirstPersonWeaponModel>,
        ),
    >,
) {
    let Ok(mode) = player_query.single() else {
        return;
    };

    let (fp_vis, tp_vis) = match mode {
        CameraMode::FirstPerson => (Visibility::Inherited, Visibility::Hidden),
        CameraMode::ThirdPerson | CameraMode::FrontPerson => {
            (Visibility::Hidden, Visibility::Inherited)
        }
    };

    for mut vis in fp_query.iter_mut() {
        *vis = fp_vis;
    }
    for mut vis in tp_query.iter_mut() {
        *vis = tp_vis;
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
        if gamepad.just_pressed(GamepadButton::RightTrigger2) {
            is_attacking = true;
        }
    }

    if !is_attacking || ui_state.show_inventory || ui_state.show_pause_menu {
        return;
    }

    if matches!(
        *weapon,
        WeaponState::Pistol | WeaponState::Revolver | WeaponState::Rifle | WeaponState::Sniper
    ) {
        return;
    }

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
    mut hittable_query: Query<
        (Entity, &GlobalTransform, &mut Health),
        (With<Hittable>, Without<DamageEvent>, Without<Player>),
    >,
    mut hit_events: MessageWriter<LaserHitEvent>,
    time: Res<Time>,
    ui_state: Res<UiState>,
    mut beam_query: Query<(Entity, &mut Transform), With<LaserBeam>>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut laser_audio: ResMut<LaserAudio>,
    laser_heat: Res<LaserHeat>,
    gamepads: Query<&Gamepad>,
    mut water_impulses: MessageWriter<crate::world::water::WaterImpulseEvent>,
) {
    let mut is_firing = mouse_input.pressed(MouseButton::Left);
    for gamepad in gamepads.iter() {
        if gamepad.pressed(GamepadButton::RightTrigger2) {
            is_firing = true;
        }
    }

    if !is_firing
        || *weapon != WeaponState::Laser
        || ui_state.show_inventory
        || ui_state.show_pause_menu
        || laser_heat.overheated
    {
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
        laser_audio.playing_entity = Some(
            commands
                .spawn((
                    AudioPlayer::new(laser_audio.sound.clone()),
                    PlaybackSettings::LOOP, // Default volume to avoid compilation error
                ))
                .id(),
        );
    }

    if let Ok((camera_transform, _)) = camera_query.single() {
        let shoot_pos = camera_transform.translation();
        let forward = camera_transform.forward();

        // Offset the beam start point to the player's right hand position
        let right = camera_transform.right();
        let hand_offset = Vec3::from(right) * 0.5
            + Vec3::from(camera_transform.down()) * 0.3
            + Vec3::from(forward) * 0.5;
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
                let horizontal_dist = Vec2::new(
                    target_transform.translation().x,
                    target_transform.translation().z,
                )
                .distance(Vec2::new(closest_point.x, closest_point.z));
                let vertical_diff = closest_point.y - target_transform.translation().y;

                if horizontal_dist < 1.5 && (0.0..15.0).contains(&vertical_diff) {
                    health.hp -= 40.0 * time.delta_secs(); // Increased damage for faster tree chopping
                    hit_pos = closest_point; // Set hit point to the point on the ray
                    hit_events.write(LaserHitEvent {
                        position: hit_pos,
                        _normal: -forward_vec,
                    });
                    if health.hp <= 0.0 {
                        // Death is handled in health_death now, but we add DamageEvent to trigger it
                        commands
                            .entity(target_entity)
                            .insert(DamageEvent(health.hp.abs() + 1.0));
                    }
                }
            }
        }

        let ray = Ray3d::new(shoot_pos, forward);
        if let Some(hit) = voxel_world.raycast(ray, &|(_, v): (Vec3, WorldVoxel)| v.is_solid()) {
            hit_pos = hit.position;
            hit_events.write(LaserHitEvent {
                position: hit_pos,
                _normal: hit.normal.unwrap_or(Vec3::Y),
            });
            voxel_world.set_voxel(hit.voxel_pos(), WorldVoxel::Air);

            if hit_pos.y < 35.0 && hit_pos.y > 20.0 {
                water_impulses.write(crate::world::water::WaterImpulseEvent {
                    position: hit_pos,
                    force: -15.0,
                    radius: 1.5,
                });
            }
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
        if gamepad.pressed(GamepadButton::RightTrigger2) {
            is_firing = true;
        }
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

fn fire_guns(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mouse_input: Res<ButtonInput<MouseButton>>,
    weapon: Res<WeaponState>,
    mut cooldowns: ResMut<GunCooldowns>,
    mut recoil: ResMut<RecoilState>,
    mut ammo: ResMut<AmmoState>,
    gun_sounds: Res<GunSounds>,
    keys: Res<ButtonInput<KeyCode>>,
    time: Res<Time>,
    player_query: Query<&Transform, With<Player>>,
    camera_query: Query<&GlobalTransform, With<Camera3d>>,
    ui_state: Res<UiState>,
    gamepads: Query<&Gamepad>,
    voxel_world: VoxelWorld<NoiseGenerator>,
) {
    if ui_state.show_inventory || ui_state.show_pause_menu {
        return;
    }

    let dt = time.delta_secs();
    cooldowns.pistol_timer = (cooldowns.pistol_timer - dt).max(0.0);
    cooldowns.revolver_timer = (cooldowns.revolver_timer - dt).max(0.0);
    cooldowns.rifle_timer = (cooldowns.rifle_timer - dt).max(0.0);
    cooldowns.sniper_timer = (cooldowns.sniper_timer - dt).max(0.0);

    // Handle reloading timer
    if ammo.reload_timer > 0.0 {
        ammo.reload_timer -= dt;
        if ammo.reload_timer <= 0.0 {
            if let Some(wp) = ammo.reloading_weapon {
                match wp {
                    WeaponState::Pistol => ammo.pistol_ammo = 12,
                    WeaponState::Revolver => ammo.revolver_ammo = 6,
                    WeaponState::Rifle => ammo.rifle_ammo = 30,
                    WeaponState::Sniper => ammo.sniper_ammo = 5,
                    _ => {}
                }
            }
            ammo.reloading_weapon = None;
        }
    }

    let current_weapon = *weapon;
    let is_gun = matches!(
        current_weapon,
        WeaponState::Pistol | WeaponState::Revolver | WeaponState::Rifle | WeaponState::Sniper
    );

    if !is_gun {
        return;
    }

    // Trigger reload manually with R key or Gamepad West
    let mut request_reload = keys.just_pressed(KeyCode::KeyR);
    for gamepad in gamepads.iter() {
        if gamepad.just_pressed(GamepadButton::West) {
            request_reload = true;
        }
    }

    if request_reload && ammo.reloading_weapon.is_none() {
        let max_ammo = match current_weapon {
            WeaponState::Pistol => 12,
            WeaponState::Revolver => 6,
            WeaponState::Rifle => 30,
            WeaponState::Sniper => 5,
            _ => 0,
        };
        let current_ammo = match current_weapon {
            WeaponState::Pistol => ammo.pistol_ammo,
            WeaponState::Revolver => ammo.revolver_ammo,
            WeaponState::Rifle => ammo.rifle_ammo,
            WeaponState::Sniper => ammo.sniper_ammo,
            _ => 0,
        };
        if current_ammo < max_ammo {
            ammo.reload_timer = 1.5;
            ammo.reloading_weapon = Some(current_weapon);
            commands.spawn((
                AudioPlayer::new(gun_sounds.reload.clone()),
                PlaybackSettings::DESPAWN,
            ));
            return;
        }
    }

    // If currently reloading, we cannot fire
    if ammo.reloading_weapon.is_some() {
        return;
    }

    let is_firing = if current_weapon == WeaponState::Rifle {
        let mut pressed = mouse_input.pressed(MouseButton::Left);
        for gamepad in gamepads.iter() {
            if gamepad.pressed(GamepadButton::RightTrigger2) {
                pressed = true;
            }
        }
        pressed
    } else {
        let mut pressed = mouse_input.just_pressed(MouseButton::Left);
        for gamepad in gamepads.iter() {
            if gamepad.just_pressed(GamepadButton::RightTrigger2) {
                pressed = true;
            }
        }
        pressed
    };

    if !is_firing {
        return;
    }

    let can_fire = match current_weapon {
        WeaponState::Pistol => cooldowns.pistol_timer <= 0.0,
        WeaponState::Revolver => cooldowns.revolver_timer <= 0.0,
        WeaponState::Rifle => cooldowns.rifle_timer <= 0.0,
        WeaponState::Sniper => cooldowns.sniper_timer <= 0.0,
        _ => false,
    };

    if !can_fire {
        return;
    }

    // Check ammo capacity
    let current_ammo = match current_weapon {
        WeaponState::Pistol => &mut ammo.pistol_ammo,
        WeaponState::Revolver => &mut ammo.revolver_ammo,
        WeaponState::Rifle => &mut ammo.rifle_ammo,
        WeaponState::Sniper => &mut ammo.sniper_ammo,
        _ => &mut 0,
    };

    if *current_ammo == 0 {
        // Trigger auto-reload
        ammo.reload_timer = 1.5;
        ammo.reloading_weapon = Some(current_weapon);
        commands.spawn((
            AudioPlayer::new(gun_sounds.reload.clone()),
            PlaybackSettings::DESPAWN,
        ));
        return;
    }

    // Consume 1 ammo
    *current_ammo -= 1;

    // Apply exact timing, recoil, and play shoot sound
    let (timer_cooldown, recoil_amount, recoil_max, shoot_sound) = match current_weapon {
        WeaponState::Pistol => (0.5, 0.12, 0.25, gun_sounds.pistol_shoot.clone()),
        WeaponState::Revolver => (0.8, 0.22, 0.40, gun_sounds.revolver_shoot.clone()),
        WeaponState::Rifle => (0.15, 0.05, 0.18, gun_sounds.rifle_shoot.clone()),
        WeaponState::Sniper => (2.0, 0.45, 0.65, gun_sounds.sniper_shoot.clone()),
        _ => unreachable!(),
    };

    match current_weapon {
        WeaponState::Pistol => cooldowns.pistol_timer = timer_cooldown,
        WeaponState::Revolver => cooldowns.revolver_timer = timer_cooldown,
        WeaponState::Rifle => cooldowns.rifle_timer = timer_cooldown,
        WeaponState::Sniper => cooldowns.sniper_timer = timer_cooldown,
        _ => {}
    }

    recoil.amount = (recoil.amount + recoil_amount).min(recoil_max);

    // Play gunshot sound effect
    commands.spawn((AudioPlayer::new(shoot_sound), PlaybackSettings::DESPAWN));

    // Spawn projectile bullet tracer
    if let Ok(player_transform) = player_query.single()
        && let Ok(camera_transform) = camera_query.single()
    {
        let camera_pos = camera_transform.translation();
        let camera_forward = camera_transform.forward();
        let right = camera_transform.right();

        let hand_offset = Vec3::from(right) * 0.3
            + Vec3::from(camera_transform.down()) * 0.2
            + Vec3::from(camera_forward) * 0.8;
        let spawn_pos = player_transform.translation + Vec3::new(0.0, 1.5, 0.0) + hand_offset;

        // Raycast aiming
        let ray = Ray3d::new(camera_pos, camera_forward);
        let target_point = if let Some(hit) =
            voxel_world.raycast(ray, &|(_, v): (Vec3, WorldVoxel)| v.is_solid())
        {
            hit.position
        } else {
            camera_pos + Vec3::from(camera_forward) * 100.0
        };

        let shoot_dir = (target_point - spawn_pos).normalize_or_zero();

        let (velocity_scalar, damage, bullet_mesh, bullet_material) = match current_weapon {
            WeaponState::Pistol => (
                100.0,
                15.0,
                meshes.add(Cuboid::new(0.02, 0.02, 0.15)),
                materials.add(StandardMaterial {
                    base_color: Color::srgb(1.0, 0.9, 0.3),
                    emissive: LinearRgba::from(Color::srgb(1.5, 1.3, 0.4)),
                    ..default()
                }),
            ),
            WeaponState::Revolver => (
                110.0,
                25.0,
                meshes.add(Cuboid::new(0.025, 0.025, 0.18)),
                materials.add(StandardMaterial {
                    base_color: Color::srgb(1.0, 0.85, 0.2),
                    emissive: LinearRgba::from(Color::srgb(1.8, 1.5, 0.3)),
                    ..default()
                }),
            ),
            WeaponState::Rifle => (
                120.0,
                20.0,
                meshes.add(Cuboid::new(0.02, 0.02, 0.22)),
                materials.add(StandardMaterial {
                    base_color: Color::srgb(1.0, 0.7, 0.1),
                    emissive: LinearRgba::from(Color::srgb(2.0, 1.4, 0.2)),
                    ..default()
                }),
            ),
            WeaponState::Sniper => (
                180.0,
                100.0,
                meshes.add(Cuboid::new(0.03, 0.03, 0.35)),
                materials.add(StandardMaterial {
                    base_color: Color::srgb(1.0, 0.5, 0.0),
                    emissive: LinearRgba::from(Color::srgb(3.0, 1.5, 0.0)),
                    ..default()
                }),
            ),
            _ => unreachable!(),
        };

        let velocity = shoot_dir * velocity_scalar;

        commands.spawn((
            Mesh3d(bullet_mesh),
            MeshMaterial3d(bullet_material),
            Transform::from_translation(spawn_pos).looking_to(shoot_dir, Vec3::Y),
            Projectile {
                velocity,
                damage,
                lifetime: Timer::from_seconds(3.0, TimerMode::Once),
            },
        ));
    }
}

fn update_recoil(time: Res<Time>, mut recoil: ResMut<RecoilState>) {
    let dt = time.delta_secs();
    // Exponential decay of target recoil amount (recovery rate is 5.0)
    recoil.amount = (recoil.amount * (-dt * 5.0).exp()).max(0.0);
    if recoil.amount < 0.001 {
        recoil.amount = 0.0;
    }
    // Asymmetrical rise (kick) and fall (recovery) interpolation speeds
    let speed = if recoil.amount > recoil.current {
        30.0 // Fast kick up
    } else {
        8.0 // Slower smooth recovery
    };
    let t = (dt * speed).clamp(0.0, 1.0);
    recoil.current += (recoil.amount - recoil.current) * t;
}

fn update_weapon_sway(
    time: Res<Time>,
    mut mouse_events: MessageReader<bevy::input::mouse::MouseMotion>,
    mut query: Query<(&mut Transform, &mut WeaponSway)>,
    player_query: Query<&crate::player::camera::PhysicsState, With<Player>>,
    recoil: Res<RecoilState>,
) {
    let mut mouse_delta = Vec2::ZERO;
    for event in mouse_events.read() {
        mouse_delta += event.delta;
    }

    let Ok(physics) = player_query.single() else {
        return;
    };

    let dt = time.delta_secs();

    // Mouse movement sway
    let sway_x = (mouse_delta.x * -0.0008).clamp(-0.05, 0.05);
    let sway_y = (mouse_delta.y * 0.0008).clamp(-0.05, 0.05);

    // Walking movement bobbing
    let move_bob = if physics.grounded && physics.horizontal_velocity.length() > 0.1 {
        let bob_speed = if physics.speed > 10.0 { 14.0 } else { 10.0 };
        let bob_amp = if physics.speed > 10.0 { 0.015 } else { 0.008 };
        Vec3::new(
            (time.elapsed_secs() * bob_speed * 0.5).cos() * bob_amp,
            (time.elapsed_secs() * bob_speed).sin() * bob_amp,
            0.0,
        )
    } else {
        Vec3::ZERO
    };

    // Weapon model recoil translation kick offsets
    // Move the gun backward (Z is positive relative to parent FP camera look vector in local coordinates)
    // and kick it upward slightly (Y is positive)
    let recoil_kick_z = recoil.current * 0.35; // kick back
    let recoil_kick_y = recoil.current * 0.10; // kick up

    for (mut transform, mut sway) in query.iter_mut() {
        sway.target_offset = Vec3::new(sway_x, sway_y + recoil_kick_y, recoil_kick_z) + move_bob;
        let target = sway.target_offset;
        let current = sway.current_offset;
        let t_offset = (dt * 8.0).clamp(0.0, 1.0);
        sway.current_offset = current.lerp(target, t_offset);

        // Apply rotation sway (tilt slightly when turning) + recoil tilt up
        let rot_yaw = (mouse_delta.x * -0.003).clamp(-0.15, 0.15);
        let rot_pitch = (mouse_delta.y * -0.003).clamp(-0.15, 0.15);
        // Tilt the gun barrel upward by subtracting recoil (X-axis pitch rotation)
        let recoil_tilt = recoil.current * 0.25;
        sway.target_rotation = Quat::from_euler(
            EulerRot::YXZ,
            rot_yaw,
            rot_pitch - recoil_tilt,
            rot_yaw * 0.5,
        );

        let target_rot = sway.target_rotation;
        let t = (dt * 10.0).clamp(0.0, 1.0);
        sway.current_rotation = sway.current_rotation.slerp(target_rot, t).normalize();

        transform.translation = sway.current_offset;
        transform.rotation = sway.current_rotation;
    }
}
