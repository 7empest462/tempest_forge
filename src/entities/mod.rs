use bevy::prelude::*;

pub mod birds;
pub mod animals;
pub mod npc;

pub struct WildlifePlugin;

impl Plugin for WildlifePlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins((birds::BirdsPlugin, animals::AnimalsPlugin, npc::NPCPlugin));
    }
}

#[derive(Component)]
pub struct Creature {
    pub species: Species,
    pub state: AIState,
    pub last_attack_time: f32, // Time offset for rate-limiting
}

#[derive(Component)]
pub struct CreatureData {
    pub speed: f32,
    pub size: f32,
    pub detection_radius: f32,
}

#[derive(PartialEq, Eq, Clone, Copy, Debug)]
pub enum Species {
    Bird,
    Hawk,
    Crow,
    Deer,
    Wolf,
    Cow,
    Pig,
    Chicken,
    Spider,
    Skeleton,
}

#[derive(PartialEq, Eq, Clone, Copy, Debug)]
pub enum AIState {
    Wandering,
    Flocking,
    Chasing,
    Fleeing,
    Sleeping,
    Sitting,
}
