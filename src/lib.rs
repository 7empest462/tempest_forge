#![allow(clippy::type_complexity, clippy::too_many_arguments)]

use bevy::prelude::*;
use wasm_bindgen::prelude::*;

pub mod entities;
pub mod error;

pub mod machinery;
pub mod particle_effects;
pub mod persistence;
pub mod physics;
pub mod player;
pub mod procedural_walls;
pub mod ui;
pub mod voxel;
pub mod world;

#[derive(States, Debug, Clone, Copy, Eq, PartialEq, Hash, Default)]
pub enum GameState {
    #[default]
    Loading,
    InGame,
}

#[wasm_bindgen]
pub fn start_game() {
    #[cfg(not(target_arch = "wasm32"))]
    let is_dev = std::env::args().any(|arg| arg == "--dev");
    #[cfg(target_arch = "wasm32")]
    let is_dev = false;

    let mut app = App::new();

    #[allow(unused_mut)]
    let mut plugins = DefaultPlugins.set(WindowPlugin {
        primary_window: Some(Window {
            title: "Tempest Forge".into(),
            canvas: Some("#bevy".to_string()),
            fit_canvas_to_parent: true,
            ..default()
        }),
        ..default()
    });

    #[cfg(target_arch = "wasm32")]
    {
        use bevy::render::{
            RenderPlugin,
            settings::{Backends, RenderCreation, WgpuSettings},
        };
        plugins = plugins.set(RenderPlugin {
            render_creation: RenderCreation::Automatic(WgpuSettings {
                backends: Some(Backends::BROWSER_WEBGPU),
                ..default()
            }),
            ..default()
        });
    }

    app.add_plugins(plugins);

    #[cfg(target_arch = "wasm32")]
    app.add_systems(Update, handle_browser_gestures);

    app.init_state::<GameState>()
        .add_plugins(bevy_rapier3d::prelude::RapierPhysicsPlugin::<()>::default())
        .add_plugins(bevy_hanabi::HanabiPlugin)
        .add_plugins(bevy_egui::EguiPlugin::default());

    if is_dev {
        app.add_plugins(bevy_inspector_egui::quick::WorldInspectorPlugin::new())
            .add_systems(Startup, register_custom_inspector);
    }

    app.add_plugins(world::WorldPlugin)
        .add_plugins(machinery::MachineryPlugin)
        .add_plugins(entities::WildlifePlugin)
        .add_plugins(physics::PhysicsPlugin)
        .add_plugins(player::camera::CameraPlugin)
        .add_plugins(player::interaction::InteractionPlugin)
        .add_plugins(player::combat::CombatPlugin)
        .add_plugins(procedural_walls::ProceduralWallsPlugin)
        .add_plugins(ui::UiPlugin)
        .add_plugins(particle_effects::ParticleEffectsPlugin)
        .add_plugins(persistence::PersistencePlugin)
        .insert_resource(crate::voxel::chunk::build_block_registry())
        .add_systems(Startup, setup)
        .run();
}

fn register_custom_inspector(type_registry: Res<AppTypeRegistry>) {
    let custom_impl = bevy_inspector_egui::inspector_egui_impls::InspectorEguiImpl::new(
        |val_any, ui, _, _, _| {
            if let Some(arc_bool) =
                val_any.downcast_mut::<std::sync::Arc<std::sync::atomic::AtomicBool>>()
            {
                let val = arc_bool.load(std::sync::atomic::Ordering::Relaxed);
                let mut new_val = val;
                if ui.checkbox(&mut new_val, "").changed() {
                    arc_bool.store(new_val, std::sync::atomic::Ordering::Relaxed);
                    true
                } else {
                    false
                }
            } else {
                false
            }
        },
        |val_any, ui, _, _, _| {
            if let Some(arc_bool) =
                val_any.downcast_ref::<std::sync::Arc<std::sync::atomic::AtomicBool>>()
            {
                let val = arc_bool.load(std::sync::atomic::Ordering::Relaxed);
                let mut val_copy = val;
                ui.add_enabled_ui(false, |ui| {
                    ui.checkbox(&mut val_copy, "");
                });
            }
        },
        |_, _, _, _, _, _| false,
    );

    let mut registry_write = type_registry.write();
    if let Some(registration) = registry_write.get_mut(std::any::TypeId::of::<
        std::sync::Arc<std::sync::atomic::AtomicBool>,
    >()) {
        registration.insert(custom_impl);
        info!("Successfully registered custom InspectorEguiImpl for Arc<AtomicBool>");
    } else {
        warn!("Could not find Arc<AtomicBool> in TypeRegistry to insert custom InspectorEguiImpl!");
    }
}

fn setup(
    mut commands: Commands,
    _meshes: ResMut<Assets<Mesh>>,
    _materials: ResMut<Assets<StandardMaterial>>,
) {
    // Ambient light
    commands.insert_resource(GlobalAmbientLight {
        color: Color::srgb(1.0, 1.0, 1.0),
        brightness: 100.0,
        affects_lightmapped_meshes: true,
    });

    // Dynamic Sun
    commands.spawn((
        DirectionalLight {
            shadows_enabled: true,
            illuminance: 14000.0,
            shadow_depth_bias: 0.03,
            shadow_normal_bias: 1.5,
            ..default()
        },
        Transform::IDENTITY,
        crate::world::env::Sun,
    ));

    // Dynamic Moon
    commands.spawn((
        DirectionalLight {
            shadows_enabled: true,
            illuminance: 10000.0,
            shadow_depth_bias: 0.03,
            shadow_normal_bias: 1.5,
            ..default()
        },
        Transform::IDENTITY,
        crate::world::env::Moon,
    ));
}

#[cfg(target_arch = "wasm32")]
fn handle_browser_gestures(
    mouse_input: Res<ButtonInput<MouseButton>>,
    mut window_query: Query<&mut bevy::window::CursorOptions, With<bevy::window::PrimaryWindow>>,
) {
    if mouse_input.just_pressed(MouseButton::Left) {
        #[cfg(target_arch = "wasm32")]
        {
            web_sys::console::log_1(&"GESTURE DETECTED: LOCKING MOUSE".into());
        }

        if let Ok(mut cursor_options) = window_query.single_mut() {
            cursor_options.grab_mode = bevy::window::CursorGrabMode::Locked;
            cursor_options.visible = false;
        }
    }
}
