use bevy::prelude::*;
use wasm_bindgen::prelude::*;

pub mod voxel;
pub mod player;
pub mod world;
pub mod machinery;
pub mod entities;
pub mod physics;
pub mod ui;
pub mod particle_effects;
pub mod persistence;
pub mod procedural_walls;

#[derive(States, Debug, Clone, Copy, Eq, PartialEq, Hash, Default)]
pub enum GameState {
    #[default]
    Loading,
    InGame,
}

#[wasm_bindgen]
pub fn start_game() {
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
        use bevy::render::{settings::{Backends, RenderCreation, WgpuSettings}, RenderPlugin};
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
        .add_plugins(bevy_egui::EguiPlugin::default())
        .add_plugins(world::WorldPlugin)
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
