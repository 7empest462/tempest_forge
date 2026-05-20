use bevy::{
    input::mouse::MouseMotion,
    prelude::*,
    pbr::ScatteringMedium,
};
use crate::ui::UiState;
use crate::world::noise_generator::NoiseGenerator;
use crate::world::manager::find_ground_height;
use bevy_voxel_world::prelude::*;
use crate::world::water::{WaterSimData, WaterMesh, get_water_height};
use bevy_hanabi::ParticleEffect;

pub struct CameraPlugin;

impl Plugin for CameraPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, (setup_player, setup_water_audio, setup_flight_audio));
        app.add_systems(Update, (
            player_move,
            player_look,
            mech_controls,
            camera_toggle,
            mech_visual_toggle,
            player_animation,
            player_grounding,
            update_flight_effects,
            animate_thruster_flames,
        ));
    }
}
#[derive(Resource)]
pub struct WaterAudio {
    pub splash_sound: Handle<AudioSource>,
    pub swim_sound: Handle<AudioSource>,
    pub swim_playing_entity: Option<Entity>,
}

fn setup_water_audio(mut commands: Commands, asset_server: Res<AssetServer>) {
    commands.insert_resource(WaterAudio {
        splash_sound: asset_server.load("water_splash.ogg"),
        swim_sound: asset_server.load("water_swim.ogg"),
        swim_playing_entity: None,
    });
}

#[derive(Resource)]
pub struct FlightAudio {
    pub sound: Handle<AudioSource>,
    pub playing_entity: Option<Entity>,
}

fn setup_flight_audio(mut commands: Commands, asset_server: Res<AssetServer>) {
    commands.insert_resource(FlightAudio {
        sound: asset_server.load("thruster_loop.wav"),
        playing_entity: None,
    });
}

#[derive(Component)]
pub struct Player;

#[derive(Component, Default, serde::Serialize, serde::Deserialize, Clone)]
pub struct PhysicsState {
    pub velocity: Vec3,
    pub horizontal_velocity: Vec2, 
    pub grounded: bool,
    pub flying: bool,
    pub swimming: bool,
    pub speed: f32,
    pub spawn_timer: f32,
    pub initialized: bool,
    pub waiting_for_ground: bool,
}

#[derive(Clone, Copy, PartialEq, Default, serde::Serialize, serde::Deserialize)]
pub enum MechTool {
    #[default]
    Drill,
    Axe,
    Laser,
}

#[derive(Component, Default, serde::Serialize, serde::Deserialize, Clone)]
pub struct MechSuit {
    pub active: bool,
    pub jump_level: u32,
    pub flight_enabled: bool,
    pub mining_level: u32,
    pub active_tool: MechTool,
}

#[derive(Component)]
pub struct MechDrill;

#[derive(Component)]
pub struct MechAxe;

#[derive(Component)]
pub struct MechLaser;

#[derive(Component, PartialEq, Clone, Copy, Default, Debug)]
pub enum CameraMode {
    #[default]
    ThirdPerson,
    FirstPerson,
    FrontPerson,
}

#[derive(Component)]
pub struct PlayerLeg {
    pub side: f32, 
}

#[derive(Component)]
pub struct PlayerArm {
    pub side: f32, 
    pub animation_timer: f32,
}

#[derive(Component)]
pub struct CameraPivot;

#[derive(Component)]
pub struct PlayerHead;

#[derive(Component)]
pub struct MechVisual;

#[derive(Component)]
pub struct PlayerBodyArea;

#[derive(Component)]
pub struct PlayerBody;

fn setup_player(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    _scattering_media: ResMut<Assets<ScatteringMedium>>,
    noise_gen: Res<NoiseGenerator>,
) {
    let head_mesh = meshes.add(crate::player::model::build_head_mesh());
    let body_mesh = meshes.add(crate::player::model::build_body_mesh());
    let arm_mesh_l = meshes.add(crate::player::model::build_arm_mesh(false));
    let arm_mesh_r = meshes.add(crate::player::model::build_arm_mesh(true));
    let leg_mesh = meshes.add(crate::player::model::build_leg_mesh());

    let character_mat = materials.add(StandardMaterial {
        base_color: Color::WHITE,
        perceptual_roughness: 0.8,
        reflectance: 0.2,
        ..default()
    });

    let mech_pal = crate::player::model::mech_palette();
    let mech_materials: Vec<Handle<StandardMaterial>> = mech_pal.iter().map(|(color, metallic, emissive)| {
        materials.add(StandardMaterial {
            base_color: *color,
            metallic: *metallic,
            emissive: *emissive,
            ..default()
        })
    }).collect();

    let helmet_parts = crate::player::model::mech_helmet();
    let chest_parts = crate::player::model::mech_chest();
    let reactor_parts = crate::player::model::mech_reactor();
    let shoulder_l = crate::player::model::mech_shoulder_left();
    let shoulder_r = crate::player::model::mech_shoulder_right();
    let gauntlet_parts = crate::player::model::mech_gauntlet();
    let leg_armor = crate::player::model::mech_leg_armor();
    let boot_parts = crate::player::model::mech_boot();

    let drill_mat = materials.add(StandardMaterial {
        base_color: Color::srgb(0.6, 0.6, 0.7),
        metallic: 1.0,
        ..default()
    });
    let axe_mat = materials.add(StandardMaterial {
        base_color: Color::srgb(0.8, 0.2, 0.1),
        ..default()
    });

    let mut spawn_pos = Vec3::new(0.0, 100.0, 0.0);
    let mut found = false;

    // Center-First Plateau Search: prioritize the (0,0) mainland core
    for r in 0..16 { 
        let offset = r as f32 * 10.0;
        let count = 12;
        for i in 0..count {
            let angle = (i as f32 / count as f32) * std::f32::consts::TAU;
            let pos_2d = Vec2::new(angle.cos() * offset, angle.sin() * offset);
            
            let terrain = noise_gen.get_terrain(pos_2d.x, pos_2d.y);
            // Look for high ground near origin center
            if terrain.height > 10.0 { 
                spawn_pos = Vec3::new(pos_2d.x, terrain.height + 15.0, pos_2d.y);
                found = true;
                println!("CENTRAL MAINLAND FOUND at Ring {}: Height: {}", r, terrain.height);
                break;
            }
        }
        if found { break; }
    }

    if !found {
        println!("WARNING: Central mainland not found! Defaulting to origin.");
    }

    commands.spawn((
        Player,
        PhysicsState {
            flying: true,
            spawn_timer: 8.0, // Increased to allow high-priority spawn-wave to mesh
            initialized: false,
            ..default()
        },
        MechSuit::default(),
        CameraMode::default(),
        crate::player::combat::Health::new(100.0),
        crate::player::combat::Hittable,
        Transform::from_translation(spawn_pos),
        Visibility::default(),
        InheritedVisibility::default(),
        (
            bevy_rapier3d::prelude::RigidBody::KinematicPositionBased,
            bevy_rapier3d::prelude::Collider::capsule_y(0.9, 0.35),
            bevy_rapier3d::prelude::KinematicCharacterController {
                up: Vec3::Y,
                offset: bevy_rapier3d::prelude::CharacterLength::Relative(0.1),
                slide: true,
                max_slope_climb_angle: 55.0_f32.to_radians(),
                min_slope_slide_angle: 70.0_f32.to_radians(),
                snap_to_ground: Some(bevy_rapier3d::prelude::CharacterLength::Relative(0.2)),
                autostep: Some(bevy_rapier3d::prelude::CharacterAutostep {
                    max_height: bevy_rapier3d::prelude::CharacterLength::Absolute(1.05),
                    min_width: bevy_rapier3d::prelude::CharacterLength::Absolute(0.3),
                    include_dynamic_bodies: false,
                }),
                ..default()
            },
            bevy_rapier3d::prelude::Restitution::coefficient(0.0),
            bevy_rapier3d::prelude::Friction::coefficient(0.6),
            bevy_rapier3d::prelude::ActiveCollisionTypes::default() | bevy_rapier3d::prelude::ActiveCollisionTypes::KINEMATIC_STATIC,
            bevy_rapier3d::prelude::ActiveHooks::FILTER_CONTACT_PAIRS,
        )
    )).with_children(|parent| {
        println!("PLAYER SPAWNED AT: {:?}", spawn_pos);

        parent.spawn((
            CameraPivot,
            Transform::from_xyz(0.0, 1.65, 0.0),
            Visibility::default(),
            InheritedVisibility::default(),
        )).with_children(|pivot| {
            pivot.spawn((
                Camera3d::default(),
                VoxelWorldCamera::<NoiseGenerator>::default(),
                Projection::Perspective(PerspectiveProjection {
                    fov: 90.0f32.to_radians(),
                    far: 2000.0,
                    near: 0.1, // Reverted to safer default
                    ..default()
                }),
                Transform::from_xyz(0.0, 0.5, 3.5).with_rotation(Quat::from_rotation_x(-0.25)), // Looking down
            ));
        }); 

        parent.spawn((
            Transform::from_xyz(0.0, 1.65, 0.0),
            Visibility::default(),
            InheritedVisibility::default(),
        )).with_children(|upper| {

            upper.spawn((
                PlayerHead,
                Mesh3d(head_mesh.clone()),
                MeshMaterial3d(character_mat.clone()),
                // Head mesh now includes built-in neck; lower it so neck meets torso
                Transform::from_xyz(0.0, 0.05, 0.0),
                Visibility::Hidden,
                InheritedVisibility::default(),
            ));

            for (pos, size, ci) in &helmet_parts {
                upper.spawn((
                    MechVisual,
                    Mesh3d(meshes.add(Cuboid::new(size.x, size.y, size.z))),
                    MeshMaterial3d(mech_materials[*ci].clone()),
                    Transform::from_translation(*pos),
                    Visibility::Hidden,
                ));
            }

            upper.spawn((
                PlayerArm { side: -1.0, animation_timer: 0.0 },
                Mesh3d(arm_mesh_l.clone()),
                MeshMaterial3d(character_mat.clone()),
                Transform::from_xyz(-0.28, -0.55, -0.1),
                Visibility::default(),
                InheritedVisibility::default(),
            )).with_children(|arm| {
                for (pos, size, ci) in &shoulder_l {
                    arm.spawn((
                        MechVisual,
                        Mesh3d(meshes.add(Cuboid::new(size.x, size.y, size.z))),
                        MeshMaterial3d(mech_materials[*ci].clone()),
                        Transform::from_translation(*pos),
                        Visibility::Hidden,
                    ));
                }
                for (pos, size, ci) in &gauntlet_parts {
                    arm.spawn((
                        MechVisual,
                        Mesh3d(meshes.add(Cuboid::new(size.x, size.y, size.z))),
                        MeshMaterial3d(mech_materials[*ci].clone()),
                        Transform::from_translation(*pos),
                        Visibility::Hidden,
                    ));
                }
            });

            upper.spawn((
                PlayerArm { side: 1.0, animation_timer: 0.0 },
                Mesh3d(arm_mesh_r.clone()),
                MeshMaterial3d(character_mat.clone()),
                Transform::from_xyz(0.28, -0.55, -0.1),
                Visibility::default(),
                InheritedVisibility::default(),
            )).with_children(|arm| {
                for (pos, size, ci) in &shoulder_r {
                    arm.spawn((
                        MechVisual,
                        Mesh3d(meshes.add(Cuboid::new(size.x, size.y, size.z))),
                        MeshMaterial3d(mech_materials[*ci].clone()),
                        Transform::from_translation(*pos),
                        Visibility::Hidden,
                    ));
                }
                for (pos, size, ci) in &gauntlet_parts {
                    arm.spawn((
                        MechVisual,
                        Mesh3d(meshes.add(Cuboid::new(size.x, size.y, size.z))),
                        MeshMaterial3d(mech_materials[*ci].clone()),
                        Transform::from_translation(*pos),
                        Visibility::Hidden,
                    ));
                }
                arm.spawn((
                    MechVisual,
                    MechDrill,
                    Mesh3d(meshes.add(Cone { radius: 0.15, height: 0.6 })),
                    MeshMaterial3d(drill_mat.clone()),
                    Transform::from_xyz(0.0, -0.7, 0.0).with_rotation(Quat::from_rotation_x(std::f32::consts::PI)),
                    Visibility::Hidden,
                ));
                arm.spawn((
                    MechAxe,
                    MechVisual,
                    Mesh3d(meshes.add(Cuboid::new(0.5, 0.3, 0.1))),
                    MeshMaterial3d(axe_mat.clone()),
                    Transform::from_xyz(0.2, -1.0, 0.0),
                    Visibility::Hidden,
                ));
                arm.spawn((
                    MechVisual,
                    MechLaser,
                    Mesh3d(meshes.add(Cuboid::new(0.15, 0.4, 0.15))),
                    MeshMaterial3d(materials.add(StandardMaterial {
                        base_color: Color::srgb(0.0, 1.0, 1.0),
                        emissive: LinearRgba::from(Color::srgb(0.0, 2.0, 2.0)),
                        ..default()
                    })),
                    Transform::from_xyz(0.0, -1.0, 0.0),
                    Visibility::Hidden,
                ));
            });
        });

        parent.spawn((
            PlayerBody,
            PlayerBodyArea,
            Mesh3d(body_mesh.clone()),
            MeshMaterial3d(character_mat.clone()),
            Transform::from_xyz(0.0, 1.25, 0.0),
            Visibility::default(),
            InheritedVisibility::default(),
        )).with_children(|body| {
            for (pos, size, ci) in &chest_parts {
                body.spawn((
                    MechVisual,
                    Mesh3d(meshes.add(Cuboid::new(size.x, size.y, size.z))),
                    MeshMaterial3d(mech_materials[*ci].clone()),
                    Transform::from_translation(*pos),
                    Visibility::Hidden,
                ));
            }
            for (pos, size, ci) in &reactor_parts {
                body.spawn((
                    MechVisual,
                    Mesh3d(meshes.add(Cuboid::new(size.x, size.y, size.z))),
                    MeshMaterial3d(mech_materials[*ci].clone()),
                    Transform::from_translation(*pos),
                    Visibility::Hidden,
                ));
            }

            // Spawn visual back rocket thruster nozzles (metallic, with glowing exhausts!)
            let nozzle_mesh = meshes.add(Cylinder { radius: 0.05, half_height: 0.075 });
            let nozzle_mat = mech_materials[7].clone(); // Dark metal
            for nozzle_side in [-1.0, 1.0] {
                // Metal nozzle housing
                body.spawn((
                    MechVisual,
                    Mesh3d(nozzle_mesh.clone()),
                    MeshMaterial3d(nozzle_mat.clone()),
                    Transform::from_xyz(nozzle_side * 0.12, -0.15, 0.22).with_rotation(Quat::from_rotation_x(1.57)), // Pointing down/back
                    Visibility::Hidden,
                ));
                // Thruster particle/flame anchor (pointing straight down!)
                body.spawn((
                    BackThrusterAnchor,
                    Transform::from_xyz(nozzle_side * 0.12, -0.22, 0.22),
                ));
            }
        });

        for side in [-1.0, 1.0] {
            parent.spawn((
                PlayerLeg { side },
                Mesh3d(leg_mesh.clone()),
                MeshMaterial3d(character_mat.clone()),
                Transform::from_xyz(side * 0.14, 0.475, 0.0),
                Visibility::default(),
                InheritedVisibility::default(),
            )).with_children(|leg| {
                for (pos, size, ci) in &leg_armor {
                    leg.spawn((
                        MechVisual,
                        Mesh3d(meshes.add(Cuboid::new(size.x, size.y, size.z))),
                        MeshMaterial3d(mech_materials[*ci].clone()),
                        Transform::from_translation(*pos),
                        Visibility::Hidden,
                    ));
                }
                for (pos, size, ci) in &boot_parts {
                    leg.spawn((
                        MechVisual,
                        Mesh3d(meshes.add(Cuboid::new(size.x, size.y, size.z))),
                        MeshMaterial3d(mech_materials[*ci].clone()),
                        Transform::from_translation(*pos),
                        Visibility::Hidden,
                    ));
                }
            });
        }
    });
}

fn player_move(
    keys: Res<ButtonInput<KeyCode>>,
    time: Res<Time>,
    mut query: Query<(
        &mut Transform, 
        &mut PhysicsState, 
        &mut bevy_rapier3d::prelude::KinematicCharacterController,
        Option<&bevy_rapier3d::prelude::KinematicCharacterControllerOutput>
    ), With<Player>>,
    camera_query: Query<&Transform, (With<CameraPivot>, Without<Player>)>,
    ui_state: Res<UiState>,
    voxel_world: VoxelWorld<NoiseGenerator>,
    noise_gen: Res<NoiseGenerator>,
    water_query: Query<(&WaterSimData, &Transform), (With<WaterMesh>, Without<Player>)>,
    mut commands: Commands,
    mut water_audio: ResMut<WaterAudio>,
    gamepads: Query<&Gamepad>,
) {
    if ui_state.show_inventory || ui_state.show_pause_menu {
        return;
    }
    let Ok((mut transform, mut physics, mut controller, output)) = query.single_mut() else { return };
    let Ok(_camera_transform) = camera_query.single() else { return };
    
    let was_swimming = physics.swimming;
    let mut move_dir = Vec3::ZERO;
    // Derive horizontal forward/right from the Player's yaw (body rotation)
    let (yaw, _, _) = transform.rotation.to_euler(EulerRot::YXZ);
    let forward = (Quat::from_rotation_y(yaw) * Vec3::NEG_Z).normalize_or_zero();
    let right = (Quat::from_rotation_y(yaw) * Vec3::X).normalize_or_zero();
    
    if keys.pressed(KeyCode::KeyW) { move_dir += forward; }
    if keys.pressed(KeyCode::KeyS) { move_dir -= forward; }
    if keys.pressed(KeyCode::KeyA) { move_dir -= right; }
    if keys.pressed(KeyCode::KeyD) { move_dir += right; }
    
    let mut gamepad_sprint = false;
    let mut gamepad_jump = false;
    let mut gamepad_dive = false;
    
    for gamepad in gamepads.iter() {
        let lx = gamepad.get(GamepadAxis::LeftStickX).unwrap_or(0.0);
        let ly = gamepad.get(GamepadAxis::LeftStickY).unwrap_or(0.0);
        if lx.abs() > 0.1 || ly.abs() > 0.1 {
            move_dir += forward * ly; // Y is forward
            move_dir += right * lx;
        }
        if gamepad.pressed(GamepadButton::LeftThumb) { gamepad_sprint = true; }
        if gamepad.pressed(GamepadButton::South) { gamepad_jump = true; }
        if gamepad.pressed(GamepadButton::East) { gamepad_dive = true; }
    }
    
    let mut speed = if physics.flying { 20.0 } else { 8.0 };
    let mut in_water = false;
    let mut submersion = 0.0;
    
    // Check for water physics (drag and buoyancy) using the dynamic simulated water
    if let Ok((water_sim, water_transform)) = water_query.single() {
        let grid_center = Vec2::new(water_transform.translation.x, water_transform.translation.z);
        let water_height = get_water_height(transform.translation.x, transform.translation.z, grid_center, water_sim);
        
        if transform.translation.y < water_height + 0.2 {
            speed = 5.0; // Swimming speed
            in_water = true;
            submersion = water_height - transform.translation.y;
        }
    }
    physics.swimming = in_water;

    if keys.pressed(KeyCode::ShiftLeft) || gamepad_sprint {
        speed *= 2.0;
    }

    if in_water {
        let target_vel = if move_dir != Vec3::ZERO {
            move_dir.normalize() * speed
        } else {
            Vec3::ZERO
        };
        // Apply smooth horizontal drag (viscous acceleration/deceleration)
        let accel_rate = if move_dir != Vec3::ZERO { 6.0 } else { 4.0 };
        physics.velocity.x += (target_vel.x - physics.velocity.x) * accel_rate * time.delta_secs();
        physics.velocity.z += (target_vel.z - physics.velocity.z) * accel_rate * time.delta_secs();
    } else {
        if move_dir != Vec3::ZERO {
            let velocity = move_dir.normalize() * speed;
            physics.velocity.x = velocity.x;
            physics.velocity.z = velocity.z;
        } else {
            physics.velocity.x = 0.0;
            physics.velocity.z = 0.0;
        }
    }
    
    // Read Rapier's native grounded state
    if let Some(out) = output {
        if !physics.flying && !in_water {
            physics.grounded = out.grounded;
        }
    }
    
    // Vertical movement & Flight Controls
    if physics.flying {
        if keys.pressed(KeyCode::Space) || gamepad_jump {
            physics.velocity.y = 10.0;
        } else if keys.pressed(KeyCode::ControlLeft) || gamepad_dive {
            physics.velocity.y = -10.0;
        } else {
            physics.velocity.y = 0.0; // Hover in place
        }
    } else {
        if keys.pressed(KeyCode::Space) || gamepad_jump {
            if !in_water && physics.grounded {
                physics.velocity.y = 8.0; // Slightly stronger jump to overcome friction
                physics.grounded = false;
            }
        }
    }
    
    if physics.spawn_timer > 0.0 {
        physics.spawn_timer -= time.delta_secs();
        physics.waiting_for_ground = true;
    }

    if physics.waiting_for_ground {
        if let Some(height) = find_ground_height(transform.translation, &voxel_world) {
            // Ground found: Lock physics
            physics.waiting_for_ground = false;
            physics.flying = false;
            println!("SURFACE DETECTED at {}. Physical stability achieved.", height);
        }
    }
    
    if !physics.flying && !physics.waiting_for_ground {
        let dt = time.delta_secs().min(0.05); // Tighter DT for stability
        if in_water {
            let is_swimming_up = keys.pressed(KeyCode::Space) || gamepad_jump;
            let is_diving_down = keys.pressed(KeyCode::ControlLeft) || gamepad_dive;
            
            if is_swimming_up {
                let target_y_vel = if keys.pressed(KeyCode::ShiftLeft) || gamepad_sprint { 6.0 } else { 4.0 };
                physics.velocity.y += (target_y_vel - physics.velocity.y) * 6.0 * dt;
            } else if is_diving_down {
                let target_y_vel = if keys.pressed(KeyCode::ShiftLeft) || gamepad_sprint { -6.0 } else { -4.0 };
                physics.velocity.y += (target_y_vel - physics.velocity.y) * 6.0 * dt;
            } else {
                // Natural Buoyancy & Bobbing (Spring-Mass-Damper model at chest-height of 1.2m)
                let buoyancy_accel = ((submersion - 1.2) * 12.0).clamp(-8.0, 15.0);
                physics.velocity.y += buoyancy_accel * dt;
                
                // Viscous fluid drag (damping)
                physics.velocity.y += (0.0 - physics.velocity.y) * 3.0 * dt;
            }
        } else {
            if !physics.grounded {
                physics.velocity.y -= 25.0 * dt; // Gravity
            } else if physics.velocity.y < 0.0 {
                // Prevent gravity from accumulating infinitely while standing on the ground
                physics.velocity.y = 0.0;
            }
        }
    }
    
    let dt = time.delta_secs().min(0.1);
    let delta = physics.velocity * dt;
    
    // Apply the translation delta to the controller so Rapier can sweep for collisions!
    controller.translation = Some(delta);
    
    physics.horizontal_velocity = Vec2::new(physics.velocity.x, physics.velocity.z);

    // Splash & Swimming Audio Logic
    let just_entered_water = in_water && !was_swimming;

    if just_entered_water {
        commands.spawn(AudioPlayer::new(water_audio.splash_sound.clone()));
    }

    let is_moving_in_water = in_water && (move_dir != Vec3::ZERO || physics.velocity.y.abs() > 0.5);

    if is_moving_in_water {
        if water_audio.swim_playing_entity.is_none() {
            let entity = commands.spawn((
                AudioPlayer::new(water_audio.swim_sound.clone()),
                PlaybackSettings::LOOP,
            )).id();
            water_audio.swim_playing_entity = Some(entity);
        }
    } else {
        if let Some(entity) = water_audio.swim_playing_entity.take() {
            commands.entity(entity).despawn();
        }
    }

    // Safety Respawn
    if transform.translation.y < -128.0 {
        // High-reliability spiral search for safe land on respawn
        let mut respawn_pos = Vec3::new(200.0, 100.0, 200.0);
        let mut found = false;
        for r in 0..30 {
            let offset = r as f32 * 32.0;
            let positions = [Vec2::new(offset, offset), Vec2::new(-offset, -offset), Vec2::new(offset, 0.0), Vec2::new(0.0, 0.0)];
            for p in positions {
                let terrain = noise_gen.get_terrain(p.x, p.y);
                if terrain.height > 5.0 {
                    respawn_pos = Vec3::new(p.x, terrain.height + 20.0, p.y);
                    found = true;
                    break;
                }
            }
            if found { break; }
        }
        transform.translation = respawn_pos;
        physics.velocity = Vec3::ZERO;
        physics.flying = true;
        physics.spawn_timer = 3.0;
        physics.waiting_for_ground = true;
    }
}

fn player_grounding(
    voxel_world: VoxelWorld<NoiseGenerator>,
    mut query: Query<(&mut Transform, &mut PhysicsState), With<Player>>,
) {
    let Ok((mut transform, mut physics)) = query.single_mut() else { return };
    if physics.flying { return; }

    // If we are significantly below ground (teleport/loading), snap up
    if let Some(ground) = find_ground_height(transform.translation, &voxel_world) {
        if transform.translation.y < ground - 1.0 {
            transform.translation.y = ground;
            physics.velocity.y = 0.0;
            physics.grounded = true;
        }
    }
}

fn player_look(
    mut mouse_events: MessageReader<MouseMotion>,
    mut query: Query<&mut Transform, With<Player>>,
    mut pivot_query: Query<&mut Transform, (With<CameraPivot>, Without<Player>)>,
    ui_state: Res<UiState>,
    gamepads: Query<&Gamepad>,
    time: Res<Time>,
) {
    if ui_state.show_inventory || ui_state.show_pause_menu {
        return;
    }
    let Ok(mut body_transform) = query.single_mut() else { return };
    let Ok(mut pivot_transform) = pivot_query.single_mut() else { return };
    let mut mouse_delta = Vec2::ZERO;

    for event in mouse_events.read() {
        mouse_delta += event.delta;
    }

    let mut gamepad_delta = Vec2::ZERO;
    for gamepad in gamepads.iter() {
        let rx = gamepad.get(GamepadAxis::RightStickX).unwrap_or(0.0);
        let ry = gamepad.get(GamepadAxis::RightStickY).unwrap_or(0.0);
        if rx.abs() > 0.05 || ry.abs() > 0.05 {
            // Apply scale logic for right stick aiming
            gamepad_delta.x += rx * 2500.0 * time.delta_secs(); 
            // Y is inverted between gamepad output (up is +) and mouse movement (up is -)
            gamepad_delta.y -= ry * 2500.0 * time.delta_secs();
        }
    }
    mouse_delta += gamepad_delta;

    if mouse_delta != Vec2::ZERO {
        let sensitivity = 0.002;
        let (body_yaw, _, _) = body_transform.rotation.to_euler(EulerRot::YXZ);
        let yaw = body_yaw - (mouse_delta.x * sensitivity).clamp(-0.2, 0.2); // Clamp per-frame turn
        body_transform.rotation = Quat::from_euler(EulerRot::YXZ, yaw, 0.0, 0.0).normalize();
        
        let (_body_yaw, pivot_pitch, _) = pivot_transform.rotation.to_euler(EulerRot::YXZ);
        let mut pitch: f32 = pivot_pitch - (mouse_delta.y * sensitivity).clamp(-0.2, 0.2);
        pitch = pitch.clamp(-std::f32::consts::FRAC_PI_2 + 0.01, std::f32::consts::FRAC_PI_2 - 0.01);
        pivot_transform.rotation = Quat::from_euler(EulerRot::YXZ, 0.0, pitch, 0.0).normalize();
    }
}

fn mech_controls(
    keys: Res<ButtonInput<KeyCode>>,
    mut query: Query<&mut MechSuit, With<Player>>,
    gamepads: Query<&Gamepad>,
    placement: Res<crate::player::interaction::PlacementState>,
) {
    let Ok(mut mech) = query.single_mut() else { return };
    let mut toggle = keys.just_pressed(KeyCode::KeyM);
    let mut to_drill = keys.just_pressed(KeyCode::Digit1);
    let mut to_axe = keys.just_pressed(KeyCode::Digit2);
    let mut to_laser = keys.just_pressed(KeyCode::Digit3);
    
    let is_procedural_wall = placement.current_block == crate::voxel::BlockType::ProceduralWall;
    
    for gamepad in gamepads.iter() {
        if gamepad.just_pressed(GamepadButton::North) { toggle = true; }
        if !is_procedural_wall {
            if gamepad.just_pressed(GamepadButton::DPadLeft) { to_drill = true; }
            if gamepad.just_pressed(GamepadButton::DPadUp) { to_axe = true; }
            if gamepad.just_pressed(GamepadButton::DPadRight) { to_laser = true; }
        }
    }

    if toggle { mech.active = !mech.active; }
    if to_drill { mech.active_tool = MechTool::Drill; }
    if to_axe { mech.active_tool = MechTool::Axe; }
    if to_laser { mech.active_tool = MechTool::Laser; }
}

fn camera_toggle(
    keys: Res<ButtonInput<KeyCode>>,
    mut query: Query<(&mut CameraMode, &mut PhysicsState), With<Player>>,
    mut camera_query: Query<(&mut Transform, &mut Projection), With<Camera3d>>,
    mut hit_events: MessageReader<crate::player::combat::LaserHitEvent>,
    time: Res<Time>,
    mut shake_intensity: Local<f32>,
    gamepads: Query<&Gamepad>,
) {
    let Ok((mut mode, mut physics)) = query.single_mut() else { return };
    let Ok((mut cam_transform, _proj)) = camera_query.single_mut() else { return };
    
    let mut toggle_view = keys.just_pressed(KeyCode::KeyV);
    let toggle_front = keys.just_pressed(KeyCode::KeyC);
    let mut toggle_flight = keys.just_pressed(KeyCode::KeyF);
    
    for gamepad in gamepads.iter() {
        if gamepad.just_pressed(GamepadButton::Select) { toggle_view = true; }
        if gamepad.just_pressed(GamepadButton::West) { toggle_flight = true; }
    }

    // Cycle toggle for V: Back View -> First Person
    if toggle_view {
        if *mode == CameraMode::ThirdPerson {
            *mode = CameraMode::FirstPerson;
        } else {
            *mode = CameraMode::ThirdPerson;
        }
    }

    // Explicit Front View with C
    if toggle_front {
        *mode = CameraMode::FrontPerson;
    }

    // Toggle flight with F
    if toggle_flight {
        physics.flying = !physics.flying;
        if !physics.flying {
            physics.velocity.y = 0.0;
        }
    }
    
    let mut hit = false;
    for _ in hit_events.read() {
        hit = true;
    }
    
    if hit {
        *shake_intensity = (*shake_intensity + 0.05).min(0.08);
    } else {
        *shake_intensity -= time.delta_secs() * 0.3;
        if *shake_intensity < 0.0 { *shake_intensity = 0.0; }
    }
    
    let shake_x = (time.elapsed_secs() * 45.0).sin() * *shake_intensity;
    let shake_y = (time.elapsed_secs() * 55.0).cos() * *shake_intensity;
    let shake_quat = Quat::from_euler(EulerRot::YXZ, shake_y, shake_x, 0.0);

    // Apply exact camera transform per frame, preventing drift and supporting shake
    if *mode == CameraMode::FirstPerson {
        cam_transform.translation = Vec3::new(0.0, 0.0, -0.15);
        cam_transform.rotation = Quat::IDENTITY * shake_quat;
    } else if *mode == CameraMode::ThirdPerson {
        cam_transform.translation = Vec3::new(0.0, 1.2, 3.0);
        cam_transform.rotation = Quat::from_rotation_x(-0.1) * shake_quat;
    } else if *mode == CameraMode::FrontPerson {
        cam_transform.translation = Vec3::new(0.0, 1.2, -1.8);
        cam_transform.rotation = Quat::from_euler(EulerRot::YXZ, std::f32::consts::PI, -0.1, 0.0) * shake_quat;
    }
}

fn mech_visual_toggle(
    player_query: Query<(&MechSuit, &CameraMode), With<Player>>,
    mut set: ParamSet<(
        Query<(&mut Visibility, Option<&PlayerHead>, Option<&PlayerBody>), Or<(With<PlayerHead>, With<PlayerBody>)>>,
        Query<(&mut Visibility, Option<&MechDrill>, Option<&MechAxe>, Option<&MechLaser>), With<MechVisual>>,
    )>,
) {
    let Ok((mech, mode)) = player_query.single() else { return };
    
    // 1. Handle Organic Parts (Hide only in First Person)
    for (mut visibility, head, body) in set.p0().iter_mut() {
        if mode == &CameraMode::FirstPerson && (head.is_some() || body.is_some()) {
            *visibility = Visibility::Hidden;
        } else {
            *visibility = Visibility::Inherited;
        }
    }

    // 2. Handle Mech Parts (Hide when inactive)
    for (mut visibility, drill, axe, laser) in set.p1().iter_mut() {
        if !mech.active {
            *visibility = Visibility::Hidden;
        } else {
            *visibility = Visibility::Inherited;
            
            // Tool-specific logic
            if let Some(_) = drill {
                if mech.active_tool != MechTool::Drill { *visibility = Visibility::Hidden; }
            }
            if let Some(_) = axe {
                if mech.active_tool != MechTool::Axe { *visibility = Visibility::Hidden; }
            }
            if let Some(_) = laser {
                if mech.active_tool != MechTool::Laser { *visibility = Visibility::Hidden; }
            }
        }
    }
}

fn player_animation(
    time: Res<Time>,
    player_query: Query<(&PhysicsState, &CameraMode, &MechSuit), With<Player>>,
    mut leg_query: Query<(&mut Transform, &PlayerLeg)>,
    mut arm_query: Query<(&mut Transform, &mut PlayerArm), Without<PlayerLeg>>,
    mut drill_query: Query<&mut Transform, (With<MechDrill>, Without<PlayerLeg>, Without<PlayerArm>)>,
    mouse_input: Res<ButtonInput<MouseButton>>,
    ui_state: Res<UiState>,
) {
    if ui_state.show_inventory || ui_state.show_pause_menu {
        return;
    }
    let Ok((physics, _mode, mech)) = player_query.single() else { return };
    let t = time.elapsed_secs();
    
    let horizontal_speed = physics.horizontal_velocity.length();
    let is_moving = horizontal_speed > 0.1 && physics.grounded;
    
    for (mut transform, leg) in leg_query.iter_mut() {
        if physics.swimming {
            // Swimming / Treading Water Leg Kick
            let kick_speed = if horizontal_speed > 0.1 { 14.0 } else { 4.0 };
            let kick_amplitude = if horizontal_speed > 0.1 { 0.8 } else { 0.2 };
            let angle = (t * kick_speed + (if leg.side > 0.0 { 0.0 } else { std::f32::consts::PI })).sin() * kick_amplitude;
            transform.rotation = Quat::from_euler(EulerRot::YXZ, 0.0, angle + 0.3, 0.0);
        } else if physics.flying {
            // High-tech stabilized flight pose
            let pitch = 0.45; // Tilted slightly backward to point thrusters
            let vibration = (t * 30.0 + (if leg.side > 0.0 { std::f32::consts::PI } else { 0.0 })).sin() * 0.03; // Thruster resonance sizzle
            transform.rotation = Quat::from_euler(EulerRot::YXZ, 0.0, pitch + vibration, leg.side * 0.08);
        } else if is_moving {
            let walk_speed = 10.0 * (horizontal_speed / 4.0).max(0.5);
            let angle = (t * walk_speed + (if leg.side > 0.0 { 0.0 } else { std::f32::consts::PI })).sin() * 0.5;
            transform.rotation = Quat::from_rotation_x(angle);
        } else {
            transform.rotation = Quat::IDENTITY;
        }
    }

    let is_mining = mouse_input.pressed(MouseButton::Left);

    for (mut transform, mut arm) in arm_query.iter_mut() {
        if mouse_input.just_pressed(MouseButton::Left) && arm.side > 0.0 {
            arm.animation_timer = 0.3;
        }

        if arm.animation_timer > 0.0 {
            arm.animation_timer -= time.delta_secs();
            let progress = (1.0 - (arm.animation_timer / 0.3)).clamp(0.0, 1.0);
            let angle = (progress * std::f32::consts::PI).sin() * 1.2;
            transform.rotation = Quat::from_rotation_x(angle);
            
            let scale_pulse = 1.0 + (progress * std::f32::consts::PI).sin() * 0.2;
            transform.scale = Vec3::new(1.0, scale_pulse, 1.0);
        } else if physics.swimming {
            // Swimming / Breaststroke stroke motion
            if horizontal_speed > 0.1 {
                let stroke_speed = 8.0;
                let forward_sweep = (t * stroke_speed).cos() * 0.6 - 0.2;
                let outward_sweep = (t * stroke_speed).sin() * 0.8 * arm.side;
                transform.rotation = Quat::from_euler(EulerRot::YXZ, outward_sweep, forward_sweep, 0.0);
            } else {
                // Treading water
                let sway_speed = 4.0;
                let angle = (t * sway_speed).sin() * 0.2 * arm.side;
                transform.rotation = Quat::from_euler(EulerRot::YXZ, angle, 0.0, 0.2 * arm.side);
            }
            transform.scale = Vec3::ONE;
        } else if physics.flying {
            // Flight stabilizer pose (Iron Man flight balancing!)
            let pitch = 0.35; // Pushed slightly back
            let yaw = arm.side * 0.22; // Spread outwards for flight stabilization
            let vibration = (t * 30.0 + (if arm.side > 0.0 { std::f32::consts::PI } else { 0.0 })).sin() * 0.02; // Thruster sizzle
            transform.rotation = Quat::from_euler(EulerRot::YXZ, yaw, pitch + vibration, arm.side * -0.15);
            transform.scale = Vec3::ONE;
        } else if is_moving {
            let sway_speed = 10.0;
            let angle = (t * sway_speed + (if arm.side > 0.0 { std::f32::consts::PI } else { 0.0 })).sin() * 0.3;
            transform.rotation = Quat::from_rotation_x(angle);
            transform.scale = Vec3::ONE;
        } else {
            transform.rotation = Quat::IDENTITY;
            transform.scale = Vec3::ONE;
        }
    }

    if mech.active && mech.active_tool == MechTool::Drill && is_mining {
        for mut transform in drill_query.iter_mut() {
            transform.rotate_y(20.0 * time.delta_secs());
        }
    }
}

#[derive(Component)]
pub struct FlightThruster;

#[derive(Component)]
pub struct BackThrusterAnchor;

#[derive(Component)]
pub struct BackThrusterFlame;

#[derive(Component)]
pub struct FlightLight;

fn update_flight_effects(
    mut commands: Commands,
    player_query: Query<(Entity, &PhysicsState, &MechSuit), With<Player>>,
    thruster_query: Query<Entity, With<FlightThruster>>,
    arm_query: Query<Entity, With<PlayerArm>>,
    leg_query: Query<Entity, With<PlayerLeg>>,
    back_query: Query<Entity, With<BackThrusterAnchor>>,
    flame_query: Query<Entity, With<BackThrusterFlame>>,
    flight_light_query: Query<Entity, With<FlightLight>>,
    thruster_effect: Option<Res<crate::particle_effects::ThrusterEffect>>,
    mut flight_audio: ResMut<FlightAudio>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    let Ok((player_entity, physics, mech)) = player_query.single() else { return };
    let has_thrusters = !thruster_query.is_empty();

    if physics.flying && mech.active {
        // 1. Spawn Thrusters and Flame Cones if they don't exist
        if !has_thrusters {
            if let Some(effect) = &thruster_effect {
                // Spawn under each hand (PlayerArm)
                for arm_entity in arm_query.iter() {
                    let thruster = commands.spawn((
                        FlightThruster,
                        ParticleEffect {
                            handle: effect.0.clone(),
                            ..default()
                        },
                        Transform::from_xyz(0.0, -0.8, 0.0),
                    )).id();
                    commands.entity(arm_entity).add_child(thruster);
                }
                // Spawn under each foot (PlayerLeg)
                for leg_entity in leg_query.iter() {
                    let thruster = commands.spawn((
                        FlightThruster,
                        ParticleEffect {
                            handle: effect.0.clone(),
                            ..default()
                        },
                        Transform::from_xyz(0.0, -0.5, 0.0),
                    )).id();
                    commands.entity(leg_entity).add_child(thruster);
                }
                // Spawn under back thruster nozzles (BackThrusterAnchor)
                let flame_mesh = meshes.add(Cone { radius: 0.04, height: 0.35 });
                let flame_mat = materials.add(StandardMaterial {
                    base_color: Color::srgb(0.0, 1.0, 1.0),
                    emissive: LinearRgba::from(Color::srgb(0.0, 5.0, 5.0)), // Glowing HDR cyan!
                    ..default()
                });

                for back_entity in back_query.iter() {
                    // Spawn Hanabi particles
                    let thruster = commands.spawn((
                        FlightThruster,
                        ParticleEffect {
                            handle: effect.0.clone(),
                            ..default()
                        },
                        Transform::from_xyz(0.0, 0.0, 0.0),
                    )).id();
                    commands.entity(back_entity).add_child(thruster);

                    // Spawn physical glowing cone flame
                    let flame = commands.spawn((
                        BackThrusterFlame,
                        Mesh3d(flame_mesh.clone()),
                        MeshMaterial3d(flame_mat.clone()),
                        Transform::from_xyz(0.0, -0.15, 0.0).with_rotation(Quat::from_rotation_x(std::f32::consts::PI)), // Pointing down
                    )).id();
                    commands.entity(back_entity).add_child(flame);
                }
            }
        }

        // 2. Spawn dynamic flickering FlightLight as player child
        if flight_light_query.is_empty() {
            let light = commands.spawn((
                FlightLight,
                PointLight {
                    color: Color::srgb(0.0, 1.0, 1.0),
                    intensity: 6000.0, // Lumens for excellent visibility
                    range: 20.0,
                    shadows_enabled: false,
                    ..default()
                },
                Transform::from_xyz(0.0, -1.0, 0.0),
            )).id();
            commands.entity(player_entity).add_child(light);
        }

        // 3. Play flight sound loop
        if flight_audio.playing_entity.is_none() {
            let entity = commands.spawn((
                AudioPlayer::new(flight_audio.sound.clone()),
                PlaybackSettings::LOOP,
            )).id();
            flight_audio.playing_entity = Some(entity);
        }
    } else {
        // 1. Despawn Thrusters if they exist
        for thruster in thruster_query.iter() {
            commands.entity(thruster).despawn();
        }

        // Despawn any spawned BackThrusterFlame entities
        for flame in flame_query.iter() {
            commands.entity(flame).despawn();
        }

        // 2. Despawn FlightLight
        for light in flight_light_query.iter() {
            commands.entity(light).despawn();
        }

        // 3. Stop flight sound
        if let Some(entity) = flight_audio.playing_entity.take() {
            commands.entity(entity).despawn();
        }
    }
}

fn animate_thruster_flames(
    time: Res<Time>,
    mut flame_query: Query<&mut Transform, With<BackThrusterFlame>>,
    mut light_query: Query<&mut PointLight, With<FlightLight>>,
) {
    let t = time.elapsed_secs();
    
    // Animate physical thruster flames Y-scale and XZ-scale with resonance noise frequency
    for mut transform in flame_query.iter_mut() {
        let scale_y = 1.0 + (t * 55.0).sin() * 0.25;
        let scale_xz = 0.9 + (t * 45.0).cos() * 0.15;
        transform.scale = Vec3::new(scale_xz, scale_y, scale_xz);
    }

    // Flicker dynamic thruster light to look like a real plasma jet
    for mut light in light_query.iter_mut() {
        let noise = (t * 40.0).sin() * 0.15 + (t * 85.0).cos() * 0.08;
        light.intensity = 6000.0 * (1.0 + noise);
    }
}
