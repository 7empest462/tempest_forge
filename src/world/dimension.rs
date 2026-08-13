use crate::player::camera::Player;
use bevy::prelude::*;

#[derive(Resource, Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum CurrentDimension {
    #[default]
    Normal,
    Alien,
}

pub struct DimensionPlugin;

impl Plugin for DimensionPlugin {
    fn build(&self, app: &mut App) {
        app.insert_resource(CurrentDimension::Normal)
            .add_systems(Update, portal_teleportation_system);
    }
}

fn portal_teleportation_system(
    player_query: Option<Single<&mut Transform, With<Player>>>,
    mut current_dim: ResMut<CurrentDimension>,
    mut ambient_light: ResMut<GlobalAmbientLight>,
    time: Res<Time>,
    keys: Res<ButtonInput<KeyCode>>,
    mut cooldown: Local<f32>,
    noise_generator: Res<crate::world::noise_generator::NoiseGenerator>,
) {
    if *cooldown > 0.0 {
        *cooldown -= time.delta_secs();
        return;
    }

    let Some(mut player_transform) = player_query else {
        return;
    };

    let pos = player_transform.translation;
    let alien_portal_y = noise_generator
        .get_adjusted_surface_height(10000.0, 10050.0)
        .round();

    let interact_pressed = keys.just_pressed(KeyCode::KeyE);

    match *current_dim {
        CurrentDimension::Normal => {
            // Normal portal is at Vec3(0.0, 42.0, 50.0)
            let portal_pos = Vec3::new(0.0, 42.0, 50.0);
            let dist = pos.distance(portal_pos);
            if dist < 1.5 || (dist < 3.5 && interact_pressed) {
                // Teleport to Alien dimension
                info!("Teleporting to Alien Dimension!");
                player_transform.translation = Vec3::new(10000.0, alien_portal_y + 3.0, 10035.0);
                *current_dim = CurrentDimension::Alien;
                *cooldown = 4.0; // 4 seconds cooldown to allow chunks to load

                // Set alien ambient light
                ambient_light.color = Color::srgb(0.4, 0.2, 0.6);
                ambient_light.brightness = 400.0;
            }
        }
        CurrentDimension::Alien => {
            // Alien portal is at Vec3(10000.0, alien_portal_y, 10050.0)
            let portal_pos = Vec3::new(10000.0, alien_portal_y, 10050.0);
            let dist = pos.distance(portal_pos);
            if dist < 1.5 || (dist < 3.5 && interact_pressed) {
                // Teleport back to Normal dimension
                info!("Teleporting back to Normal Dimension!");
                player_transform.translation = Vec3::new(0.0, 45.0, 40.0);
                *current_dim = CurrentDimension::Normal;
                *cooldown = 4.0;

                // Reset normal ambient light
                ambient_light.color = Color::WHITE;
                ambient_light.brightness = 80.0;
            }
        }
    }
}
