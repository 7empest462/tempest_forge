use bevy::prelude::*;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[allow(dead_code)]
pub enum PowerType {
    Kinetic,
    Electric,
}

#[derive(Component)]
pub struct PowerNode {
    #[allow(dead_code)]
    pub power_type: PowerType,
    pub current_power: f32,
    pub capacity: f32,
}

use hashbrown::HashMap;
use rustc_hash::FxBuildHasher;

pub type FxHashMap<K, V> = HashMap<K, V, FxBuildHasher>;

#[derive(Resource, Default)]
pub struct MachineryRegistry {
    pub map: FxHashMap<IVec3, Entity>,
}

#[derive(Component)]
#[allow(dead_code)]
pub enum MachineLogic {
    Generator(f32), // Produces power per tick
    Motor(f32),     // Consumes power per tick
    Axle,           // Transfer
}
