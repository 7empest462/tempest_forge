use bevy::prelude::*;
use bevy_rapier3d::prelude::*;
use crate::player::camera::Player;

pub mod rapier_physics;

/// Physics plugin - manages Rapier3d integration
pub struct PhysicsPlugin;

impl Plugin for PhysicsPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, setup_player_physics_body)
           .add_systems(Update, sync_player_with_physics);
    }
}

/// Set up player as a Rapier rigid body (kinematic)
fn setup_player_physics_body(
    mut commands: Commands,
    player_query: Query<Entity, With<Player>>,
) {
    if let Ok(player_entity) = player_query.single() {
        // Add Rapier physics components to player
        commands.entity(player_entity)
            // Kinematic body (we control movement, physics handles collision response)
            .insert(RigidBody::KinematicPositionBased)
            // Use KinematicCharacterController for automatic step climbing and smoother movement
            .insert(KinematicCharacterController {
                offset: CharacterLength::Relative(0.1),
                ..default()
            })
            // Capsule collider for realistic player shape (0.3 radius, 1.6 height)
            .insert(Collider::capsule_y(0.8, 0.3))
            // Physical properties
            .insert(Restitution::coefficient(0.0))
            .insert(Friction::coefficient(0.5))
            // Filtering
            .insert(ActiveCollisionTypes::default() | ActiveCollisionTypes::KINEMATIC_STATIC)
            .insert(ActiveHooks::FILTER_CONTACT_PAIRS);
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
