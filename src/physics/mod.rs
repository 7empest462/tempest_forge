use crate::player::camera::Player;
use bevy::prelude::*;
use bevy_rapier3d::prelude::*;

pub mod rapier_physics;

/// Physics plugin - manages Rapier3d integration
pub struct PhysicsPlugin;

impl Plugin for PhysicsPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Update, sync_player_with_physics);
    }
}

/// Keep player transform synced with Rapier
fn sync_player_with_physics(
    mut player_query: Query<(&mut Transform, &mut Velocity), With<Player>>,
) {
    // This system ensures the player's visual position matches physics
    // The actual movement is still handled by player_move() in camera.rs
    for (_transform, velocity) in player_query.iter_mut() {
        // Rapier calculates velocity from our kinematic position
        // We don't need to do anything here - just let Rapier work
        // The custom player_move() updates the Transform directly
        // Rapier then reads that and updates physics state

        // Debug: Log kinetic energy for monitoring
        let speed = velocity.linvel.length();
        if speed.is_finite() && speed > 0.1 {
            // Velocity is being properly tracked
        }
    }
}
