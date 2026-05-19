use bevy::prelude::*;
use std::fs::File;
use std::io::{Read, Write};
use crate::player::interaction::{Inventory, WorldPersistence};
use crate::player::camera::{PhysicsState, MechSuit, Player};

#[derive(serde::Serialize, serde::Deserialize, Clone)]
pub struct PlayerSaveData {
    pub translation: Vec3,
    pub rotation: Quat,
    pub physics: PhysicsState,
    pub mech: MechSuit,
    pub inventory: Inventory,
}

#[derive(serde::Serialize, serde::Deserialize, Clone)]
pub struct SaveData {
    pub player: PlayerSaveData,
    pub world: WorldPersistence,
}

pub struct PersistencePlugin;

impl Plugin for PersistencePlugin {
    fn build(&self, app: &mut App) {
        app.add_message::<SaveEvent>()
           .add_message::<LoadEvent>()
           .add_systems(Update, (handle_save_event, handle_load_event));
    }
}

#[derive(Message)]
pub struct SaveEvent;

#[derive(Message)]
pub struct LoadEvent;

fn handle_save_event(
    mut events: MessageReader<SaveEvent>,
    player_query: Query<(&Transform, &PhysicsState, &MechSuit), With<Player>>,
    inventory: Res<Inventory>,
    world_persistence: Res<WorldPersistence>,
) {
    for _ in events.read() {
        if let Ok((transform, physics, mech)) = player_query.single() {
            let save_data = SaveData {
                player: PlayerSaveData {
                    translation: transform.translation,
                    rotation: transform.rotation,
                    physics: (*physics).clone(),
                    mech: (*mech).clone(),
                    inventory: (*inventory).clone(),
                },
                world: (*world_persistence).clone(),
            };

            let json = serde_json::to_string_pretty(&save_data).unwrap();
            let mut file = File::create("world_save.json").unwrap();
            file.write_all(json.as_bytes()).unwrap();
            println!("Game Saved to world_save.json");
        }
    }
}

fn handle_load_event(
    mut events: MessageReader<LoadEvent>,
    mut player_query: Query<(&mut Transform, &mut PhysicsState, &mut MechSuit), With<Player>>,
    mut inventory: ResMut<Inventory>,
    mut world_persistence: ResMut<WorldPersistence>,
) {
    for _ in events.read() {
        if let Ok(mut file) = File::open("world_save.json") {
            let mut contents = String::new();
            file.read_to_string(&mut contents).unwrap();
            let save_data: SaveData = serde_json::from_str(&contents).unwrap();

            if let Ok((mut transform, mut physics, mut mech)) = player_query.single_mut() {
                transform.translation = save_data.player.translation;
                transform.rotation = save_data.player.rotation;
                *physics = save_data.player.physics.clone();
                *mech = save_data.player.mech.clone();
                *inventory = save_data.player.inventory.clone();
                *world_persistence = save_data.world.clone();
                println!("Game Loaded from world_save.json");
            }
        }
    }
}
