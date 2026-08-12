use super::components::*;
use bevy::prelude::*;
use rustc_hash::FxHashMap;
use smallvec::SmallVec;

pub fn update_power_grid(
    mut query: Query<(Entity, &mut PowerNode, &MachineLogic, &Transform)>,
    registry: Res<MachineryRegistry>,
) {
    // Pass 1: Generators produce
    for (_, mut node, logic, _) in query.iter_mut() {
        if let MachineLogic::Generator(amount) = logic {
            node.current_power = (node.current_power + amount).min(node.capacity);
        }
    }

    // Pass 2: Motors consume
    for (_, mut node, logic, _) in query.iter_mut() {
        if let MachineLogic::Motor(amount) = logic {
            node.current_power = (node.current_power - amount).max(0.0);
        }
    }

    // Pass 3: Naive Equalization
    // We'll iterate through all nodes and move power to neighbors
    let directions = [
        IVec3::X,
        IVec3::NEG_X,
        IVec3::Y,
        IVec3::NEG_Y,
        IVec3::Z,
        IVec3::NEG_Z,
    ];

    let mut transfers: SmallVec<[(Entity, Entity, f32); 16]> = SmallVec::new();
    let mut power_deltas: FxHashMap<Entity, f32> = FxHashMap::default();

    for (entity, node, _, transform) in query.iter() {
        if node.current_power <= 1.0 {
            continue;
        }

        let pos = transform.translation.floor().as_ivec3();
        for dir in directions {
            let neighbor_pos = pos + dir;
            if let Some(&neighbor_entity) = registry.map.get(&neighbor_pos)
                && neighbor_entity != entity
            {
                transfers.push((entity, neighbor_entity, 0.5)); // Move 0.5 units per tick
            }
        }
    }

    for (from, to, amount) in transfers {
        *power_deltas.entry(from).or_insert(0.0) -= amount;
        *power_deltas.entry(to).or_insert(0.0) += amount;
    }

    for (entity, mut node, _, _) in query.iter_mut() {
        if let Some(&delta) = power_deltas.get(&entity) {
            node.current_power = (node.current_power + delta).clamp(0.0, node.capacity);
        }
    }
}

pub fn visualize_rotation(
    mut query: Query<(&mut Transform, &PowerNode, &MachineLogic)>,
    time: Res<Time>,
) {
    let dt = time.delta_secs();
    for (mut transform, node, logic) in query.iter_mut() {
        match logic {
            MachineLogic::Generator(_) => {
                transform.rotate_y(dt * 5.0);
            }
            MachineLogic::Motor(_) => {
                if node.current_power > 0.0 {
                    transform.rotate_y(dt * 2.0);
                }
            }
            MachineLogic::Axle => {
                if node.current_power > 0.0 {
                    transform.rotate_y(dt * 3.0);
                }
            }
        }
    }
}
