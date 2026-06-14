pub mod components;
pub mod systems;

use bevy::prelude::*;
use components::MachineryRegistry;
use systems::*;

pub struct MachineryPlugin;

impl Plugin for MachineryPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<MachineryRegistry>()
            .add_systems(FixedUpdate, (update_power_grid, visualize_rotation));
    }
}
