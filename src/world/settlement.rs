use crate::entities::npc::{NPCRole, spawn_npc};
use crate::voxel::chunk::BlockType;
use crate::world::manager::find_stable_ground_height;
use crate::world::noise_generator::NoiseGenerator;
use bevy::prelude::ChildSpawnerCommands;
use bevy::prelude::*;
use bevy_hanabi::prelude::EffectAsset;
use bevy_rapier3d::prelude::*;
use bevy_voxel_world::prelude::*;
use hashbrown::HashSet;
use rustc_hash::FxBuildHasher;

pub struct SettlementPlugin;

impl Plugin for SettlementPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<SettlementRegistry>();
    }
}

#[derive(Resource, Default)]
pub struct SettlementRegistry {
    pub positions: HashSet<IVec3, FxBuildHasher>,
}

#[derive(Component)]
pub struct ProcessedSettlement;

#[derive(Component)]
pub struct Solid;

#[derive(Component)]
pub struct BuildingPart(pub Vec3); // Store half-size for AABB

#[derive(Component)]
pub struct Bridge {
    pub start: Vec3,
    pub end: Vec3,
}

#[derive(Component, Clone)]
pub struct SettlementWaypoints {
    pub nodes: Vec<Vec3>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BuildingType {
    House,
    Shop,
    Forge,
    Tavern,
    GuardTower,
    Plaza,
}

#[derive(Component)]
pub struct SettlementBuilding {
    pub building_type: BuildingType,
    pub center: Vec3,
}

pub fn spawn_settlements(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut voxel_world: VoxelWorld<NoiseGenerator>,
    mut registry: ResMut<SettlementRegistry>,
    query: Query<(Entity, &Chunk<NoiseGenerator>), Without<ProcessedSettlement>>,
    smoke_effect: Option<Res<crate::particle_effects::ChimneySmokeEffect>>,
) {
    for (entity, chunk_comp) in query.iter() {
        let chunk_pos = chunk_comp.position;

        if let Ok(mut chunk_cmd) = commands.get_entity(entity) {
            chunk_cmd.insert(ProcessedSettlement);

            if chunk_pos.y < -1 || chunk_pos.y > 4 {
                continue;
            } // Reasonable vertical range for towns

            // Pseudo-random check for settlement spawning
            let chunk_hash =
                (chunk_pos.x.wrapping_mul(73856093) ^ chunk_pos.z.wrapping_mul(19349663)).abs();
            let is_settlement_chunk = (chunk_hash % 10) == 0;

            // Prevent spawning at chunk origin (0, 0), and do not spawn medieval towns in the alien dimension (x >= 5000)
            if is_settlement_chunk && (chunk_pos.x != 0 || chunk_pos.z != 0) && chunk_pos.x < 312 {
                if registry.positions.contains(&chunk_pos) {
                    continue;
                }
                registry.positions.insert(chunk_pos);
                let town_base_x = (chunk_pos.x * 16 + 8) as f32;
                let town_base_z = (chunk_pos.z * 16 + 8) as f32;

                let h = find_stable_ground_height(
                    Vec3::new(town_base_x, (chunk_pos.y * 16 + 16) as f32, town_base_z),
                    &voxel_world,
                );
                if let Some(h) = h {
                    if (h / 16.0).floor() as i32 != chunk_pos.y {
                        continue;
                    }
                    if h < 16.0 {
                        // Sea level is 15.0
                        continue;
                    }

                    // Get GPU smoke handle
                    let smoke_handle = smoke_effect.as_ref().map(|r| r.0.clone());

                    let plaza_center = Vec3::new(town_base_x, h, town_base_z);

                    // Scoped borrow block to release immutable borrow of voxel_world
                    let (
                        tavern_pos,
                        shop_pos,
                        forge_pos,
                        farm_pos,
                        house1_pos,
                        house2_pos,
                        tower_pos,
                        town_waypoints,
                    ) = {
                        let get_surface = |x: f32, z: f32| -> Option<f32> {
                            find_stable_ground_height(
                                Vec3::new(x, (chunk_pos.y * 16 + 16) as f32, z),
                                &voxel_world,
                            )
                        };

                        let get_building_pos = |dx: f32, dz: f32| -> Option<Vec3> {
                            let wx = town_base_x + dx;
                            let wz = town_base_z + dz;
                            if let Some(bh) = get_surface(wx, wz)
                                && (bh - h).abs() < 8.0
                                && bh >= 16.0
                            {
                                return Some(Vec3::new(wx, bh, wz));
                            }
                            None
                        };

                        let tavern_pos = get_building_pos(0.0, 16.0);
                        let shop_pos = get_building_pos(22.0, 6.0);
                        let forge_pos = get_building_pos(-22.0, 6.0);
                        let farm_pos = get_building_pos(0.0, -26.0);
                        let house1_pos = get_building_pos(18.0, -18.0);
                        let house2_pos = get_building_pos(-18.0, -18.0);
                        let tower_pos = get_building_pos(26.0, -26.0);

                        // Hub-and-Spoke Waypoints Configuration
                        let node0 = plaza_center;
                        let node1 = tavern_pos
                            .map(|p| p + Vec3::new(0.0, 0.0, -6.75))
                            .unwrap_or(node0);
                        let node2 = shop_pos
                            .map(|p| p + Vec3::new(-4.5, 0.0, 0.0))
                            .unwrap_or(node0);
                        let node3 = forge_pos
                            .map(|p| p + Vec3::new(4.5, 0.0, 0.0))
                            .unwrap_or(node0);
                        let node4 = farm_pos
                            .map(|p| p + Vec3::new(0.0, 0.0, 4.5))
                            .unwrap_or(node0);
                        let node5 = house1_pos
                            .map(|p| p + Vec3::new(0.0, 0.0, 4.5))
                            .unwrap_or(node0);
                        let node6 = house2_pos
                            .map(|p| p + Vec3::new(0.0, 0.0, 4.5))
                            .unwrap_or(node0);
                        let node7 = tower_pos
                            .map(|p| p + Vec3::new(-3.0, 0.0, 3.0))
                            .unwrap_or(node0);
                        let node8 = Vec3::new(
                            town_base_x,
                            get_surface(town_base_x, town_base_z + 32.0).unwrap_or(h),
                            town_base_z + 32.0,
                        );
                        let node9 = Vec3::new(
                            town_base_x,
                            get_surface(town_base_x, town_base_z - 32.0).unwrap_or(h),
                            town_base_z - 32.0,
                        );

                        let town_waypoints = vec![
                            node0, node1, node2, node3, node4, node5, node6, node7, node8, node9,
                        ];
                        (
                            tavern_pos,
                            shop_pos,
                            forge_pos,
                            farm_pos,
                            house1_pos,
                            house2_pos,
                            tower_pos,
                            town_waypoints,
                        )
                    };

                    // Voxel foundation fill loop: fill uneven terrain under the central plaza pavement
                    let target_y = plaza_center.y.round() as i32 - 1;
                    for dx in -7..=7 {
                        for dz in -7..=7 {
                            let vx = town_base_x.round() as i32 + dx;
                            let vz = town_base_z.round() as i32 + dz;

                            let start_y = if let Some(sy) = find_stable_ground_height(
                                Vec3::new(vx as f32, (chunk_pos.y * 16 + 16) as f32, vz as f32),
                                &voxel_world,
                            ) {
                                (sy.round() as i32).max(target_y - 8)
                            } else {
                                target_y - 4
                            };

                            for vy in start_y..=target_y {
                                let pos = IVec3::new(vx, vy, vz);
                                voxel_world.set_voxel(
                                    pos,
                                    WorldVoxel::Solid(BlockType::Cobblestone as u8),
                                );
                            }
                        }
                    }

                    // Spawn Town Plaza and Road Network Anchor
                    commands
                        .spawn((
                            Transform::from_translation(plaza_center),
                            Visibility::default(),
                            InheritedVisibility::default(),
                            SettlementWaypoints {
                                nodes: town_waypoints.clone(),
                            },
                            SettlementBuilding {
                                building_type: BuildingType::Plaza,
                                center: plaza_center,
                            },
                        ))
                        .with_children(|town| {
                            // 1. Central Plaza Pavement
                            town.spawn((
                                Mesh3d(meshes.add(Cuboid::new(15.0, 0.05, 15.0))),
                                MeshMaterial3d(materials.add(StandardMaterial {
                                    base_color: Color::srgb(0.3, 0.3, 0.32),
                                    perceptual_roughness: 0.9,
                                    ..default()
                                })),
                                Transform::from_xyz(0.0, 0.015, 0.0),
                                Collider::cuboid(7.5, 0.025, 7.5), // <--- Solid Rapier collider!
                            ));

                            // 2. Central Plaza Well/Fountain
                            town.spawn((
                                Mesh3d(meshes.add(Cuboid::new(3.3, 1.2, 3.3))),
                                MeshMaterial3d(materials.add(StandardMaterial {
                                    base_color: Color::srgb(0.25, 0.25, 0.25),
                                    ..default()
                                })),
                                Transform::from_xyz(0.0, 0.6, 0.0),
                                Collider::cuboid(1.65, 0.6, 1.65),
                            ))
                            .with_children(|well| {
                                // Well water core
                                well.spawn((
                                    Mesh3d(meshes.add(Cuboid::new(2.7, 0.15, 2.7))),
                                    MeshMaterial3d(materials.add(StandardMaterial {
                                        base_color: Color::srgb(0.1, 0.3, 0.7),
                                        perceptual_roughness: 0.1,
                                        metallic: 0.5,
                                        ..default()
                                    })),
                                    Transform::from_xyz(0.0, 0.54, 0.0),
                                ));
                                // Support beams
                                for side in [-1.0, 1.0] {
                                    well.spawn((
                                        Mesh3d(meshes.add(Cuboid::new(0.18, 2.4, 0.18))),
                                        MeshMaterial3d(materials.add(StandardMaterial {
                                            base_color: Color::srgb(0.2, 0.12, 0.08),
                                            ..default()
                                        })),
                                        Transform::from_xyz(side * 1.35, 1.2, 0.0),
                                    ));
                                }
                                // Well roof
                                well.spawn((
                                    Mesh3d(meshes.add(Cuboid::new(3.6, 0.225, 2.1))),
                                    MeshMaterial3d(materials.add(StandardMaterial {
                                        base_color: Color::srgb(0.4, 0.15, 0.1),
                                        ..default()
                                    })),
                                    Transform::from_xyz(0.0, 2.4, 0.0)
                                        .with_rotation(Quat::from_rotation_x(0.2)),
                                ));
                            });

                            // 3. Sprawling gravel paths connecting building doors back to plaza node0
                            if tavern_pos.is_some() {
                                spawn_road_path(
                                    town,
                                    &mut meshes,
                                    &mut materials,
                                    town_waypoints[0],
                                    town_waypoints[1],
                                    plaza_center,
                                    &voxel_world,
                                );
                            }
                            if shop_pos.is_some() {
                                spawn_road_path(
                                    town,
                                    &mut meshes,
                                    &mut materials,
                                    town_waypoints[0],
                                    town_waypoints[2],
                                    plaza_center,
                                    &voxel_world,
                                );
                            }
                            if forge_pos.is_some() {
                                spawn_road_path(
                                    town,
                                    &mut meshes,
                                    &mut materials,
                                    town_waypoints[0],
                                    town_waypoints[3],
                                    plaza_center,
                                    &voxel_world,
                                );
                            }
                            if farm_pos.is_some() {
                                spawn_road_path(
                                    town,
                                    &mut meshes,
                                    &mut materials,
                                    town_waypoints[0],
                                    town_waypoints[4],
                                    plaza_center,
                                    &voxel_world,
                                );
                            }
                            if house1_pos.is_some() {
                                spawn_road_path(
                                    town,
                                    &mut meshes,
                                    &mut materials,
                                    town_waypoints[0],
                                    town_waypoints[5],
                                    plaza_center,
                                    &voxel_world,
                                );
                            }
                            if house2_pos.is_some() {
                                spawn_road_path(
                                    town,
                                    &mut meshes,
                                    &mut materials,
                                    town_waypoints[0],
                                    town_waypoints[6],
                                    plaza_center,
                                    &voxel_world,
                                );
                            }
                            if tower_pos.is_some() {
                                spawn_road_path(
                                    town,
                                    &mut meshes,
                                    &mut materials,
                                    town_waypoints[0],
                                    town_waypoints[7],
                                    plaza_center,
                                    &voxel_world,
                                );
                            }

                            // 4. Street lamp posts around the plaza
                            let lamp_positions =
                                [Vec3::new(6.75, 0.0, 6.75), Vec3::new(-6.75, 0.0, -6.75)];
                            for pos in lamp_positions {
                                town.spawn((
                                    Mesh3d(meshes.add(Cuboid::new(0.24, 4.2, 0.24))),
                                    MeshMaterial3d(materials.add(StandardMaterial {
                                        base_color: Color::srgb(0.15, 0.15, 0.15),
                                        ..default()
                                    })),
                                    Transform::from_translation(pos + Vec3::new(0.0, 2.1, 0.0)),
                                ))
                                .with_children(|lamp| {
                                    // Lantern frame
                                    lamp.spawn((
                                        Mesh3d(meshes.add(Cuboid::new(0.525, 0.675, 0.525))),
                                        MeshMaterial3d(materials.add(StandardMaterial {
                                            base_color: Color::srgb(1.0, 0.9, 0.6),
                                            emissive: LinearRgba::new(2.5, 2.0, 0.5, 1.0),
                                            ..default()
                                        })),
                                        Transform::from_xyz(0.0, 2.25, 0.0),
                                    ));
                                    // Lantern light source
                                    lamp.spawn((
                                        PointLight {
                                            color: Color::srgb(1.0, 0.85, 0.5),
                                            intensity: 15000.0,
                                            range: 16.0,
                                            shadows_enabled: true,
                                            ..default()
                                        },
                                        Transform::from_xyz(0.0, 2.25, 0.0),
                                    ));
                                });
                            }
                        });

                    // 5. Spawning Scenic Architecture
                    if let Some(pos) = tavern_pos {
                        spawn_building(
                            &mut commands,
                            &mut meshes,
                            &mut materials,
                            pos,
                            BuildingType::Tavern,
                            smoke_handle.clone(),
                        );
                        spawn_npc(
                            &mut commands,
                            &mut meshes,
                            &mut materials,
                            pos + Vec3::new(-1.5, 0.5, -1.0),
                            NPCRole::Barkeeper,
                            Some(pos),
                            town_waypoints.clone(),
                        );
                    }

                    if let Some(pos) = shop_pos {
                        spawn_building(
                            &mut commands,
                            &mut meshes,
                            &mut materials,
                            pos,
                            BuildingType::Shop,
                            smoke_handle.clone(),
                        );
                        spawn_npc(
                            &mut commands,
                            &mut meshes,
                            &mut materials,
                            pos + Vec3::new(0.0, 0.8, 0.0),
                            NPCRole::Merchant,
                            Some(pos),
                            town_waypoints.clone(),
                        );
                    }

                    if let Some(pos) = forge_pos {
                        spawn_building(
                            &mut commands,
                            &mut meshes,
                            &mut materials,
                            pos,
                            BuildingType::Forge,
                            smoke_handle.clone(),
                        );
                        spawn_npc(
                            &mut commands,
                            &mut meshes,
                            &mut materials,
                            pos + Vec3::new(0.0, 0.8, 1.0),
                            NPCRole::Blacksmith,
                            Some(pos),
                            town_waypoints.clone(),
                        );
                    }

                    if let Some(pos) = farm_pos {
                        spawn_building(
                            &mut commands,
                            &mut meshes,
                            &mut materials,
                            pos,
                            BuildingType::House,
                            smoke_handle.clone(),
                        );
                        spawn_npc(
                            &mut commands,
                            &mut meshes,
                            &mut materials,
                            pos + Vec3::new(-1.2, 0.5, -0.8),
                            NPCRole::Farmer,
                            Some(pos),
                            town_waypoints.clone(),
                        );

                        // Spawn Fenced Wheat Crop Garden next to farm
                        spawn_garden_field(
                            &mut commands,
                            &mut meshes,
                            &mut materials,
                            pos + Vec3::new(5.5, 0.0, 0.0),
                        );
                    }

                    if let Some(pos) = house1_pos {
                        spawn_building(
                            &mut commands,
                            &mut meshes,
                            &mut materials,
                            pos,
                            BuildingType::House,
                            smoke_handle.clone(),
                        );
                        spawn_npc(
                            &mut commands,
                            &mut meshes,
                            &mut materials,
                            pos + Vec3::new(-1.2, 0.5, -0.8),
                            NPCRole::Citizen,
                            None,
                            town_waypoints.clone(),
                        );
                    }

                    if let Some(pos) = house2_pos {
                        spawn_building(
                            &mut commands,
                            &mut meshes,
                            &mut materials,
                            pos,
                            BuildingType::House,
                            smoke_handle.clone(),
                        );
                        spawn_npc(
                            &mut commands,
                            &mut meshes,
                            &mut materials,
                            pos + Vec3::new(-1.2, 0.5, -0.8),
                            NPCRole::Citizen,
                            None,
                            town_waypoints.clone(),
                        );
                    }

                    if let Some(pos) = tower_pos {
                        spawn_building(
                            &mut commands,
                            &mut meshes,
                            &mut materials,
                            pos,
                            BuildingType::GuardTower,
                            None,
                        );

                        // Patrol Guards
                        spawn_npc(
                            &mut commands,
                            &mut meshes,
                            &mut materials,
                            pos + Vec3::new(-1.0, 1.0, 1.0),
                            NPCRole::Guard,
                            None,
                            town_waypoints.clone(),
                        );
                        spawn_npc(
                            &mut commands,
                            &mut meshes,
                            &mut materials,
                            plaza_center + Vec3::new(4.0, 1.0, -4.0),
                            NPCRole::Guard,
                            None,
                            town_waypoints.clone(),
                        );
                    }
                }
            }
        }
    }
}

#[allow(clippy::needless_range_loop)]
fn spawn_road_path(
    town: &mut ChildSpawnerCommands,
    meshes: &mut Assets<Mesh>,
    materials: &mut Assets<StandardMaterial>,
    a: Vec3,
    b: Vec3,
    rel_base: Vec3,
    voxel_world: &VoxelWorld<NoiseGenerator>,
) {
    let dist = a.distance(b);
    if dist < 0.1 {
        return;
    }

    // Segment road every 1.5 meters
    let seg_len = 1.5f32;
    let num_segments = (dist / seg_len).max(1.0).round() as usize;

    let gravel_mat = materials.add(StandardMaterial {
        base_color: Color::srgb(0.24, 0.22, 0.19),
        perceptual_roughness: 0.95,
        ..default()
    });

    let bridge_deck_mat = materials.add(StandardMaterial {
        base_color: Color::srgb(0.35, 0.2, 0.12), // Rich dark wood
        perceptual_roughness: 0.8,
        ..default()
    });

    let bridge_rail_mat = materials.add(StandardMaterial {
        base_color: Color::srgb(0.25, 0.15, 0.08), // Darker wood for rails
        perceptual_roughness: 0.85,
        ..default()
    });

    // 1. Generate node positions along path
    let mut nodes = Vec::with_capacity(num_segments + 1);
    for j in 0..=num_segments {
        let t = j as f32 / num_segments as f32;
        nodes.push(a.lerp(b, t));
    }

    // 2. Query terrain stable ground height at each node
    let mut terrain_y = Vec::with_capacity(num_segments + 1);
    for j in 0..=num_segments {
        terrain_y.push(find_stable_ground_height(nodes[j], voxel_world));
    }

    // 3. Mark which nodes are bridge nodes (deep terrain or None)
    let mut is_bridge_node = vec![false; num_segments + 1];
    for j in 1..num_segments {
        if let Some(hy) = terrain_y[j] {
            if hy < nodes[j].y - 1.5 {
                is_bridge_node[j] = true;
            }
        } else {
            is_bridge_node[j] = true;
        }
    }

    // 4. Compute final node heights with linear interpolation across gaps
    let mut final_y = vec![0.0; num_segments + 1];
    for j in 0..=num_segments {
        if !is_bridge_node[j] {
            final_y[j] = terrain_y[j].unwrap_or(nodes[j].y);
        }
    }

    let mut j = 0;
    while j <= num_segments {
        if is_bridge_node[j] {
            let start_bridge = j;
            while j <= num_segments && is_bridge_node[j] {
                j += 1;
            }
            let end_bridge = j - 1;

            let left_idx = start_bridge - 1;
            let right_idx = (end_bridge + 1).min(num_segments);

            let y_left = final_y[left_idx];
            let y_right = final_y[right_idx];

            let span = (right_idx - left_idx) as f32;
            for k in start_bridge..=end_bridge {
                let t = (k - left_idx) as f32 / span;
                final_y[k] = y_left + (y_right - y_left) * t;
            }
        } else {
            j += 1;
        }
    }

    // 5. Spawn segment entities
    for i in 0..num_segments {
        let p0 = Vec3::new(nodes[i].x, final_y[i], nodes[i].z);
        let p1 = Vec3::new(nodes[i + 1].x, final_y[i + 1], nodes[i + 1].z);
        let p_mid = (p0 + p1) / 2.0;

        let local_p0 = p0 - rel_base;
        let local_p1 = p1 - rel_base;
        let seg_dist = local_p0.distance(local_p1);
        if seg_dist < 0.01 {
            continue;
        }
        let seg_mid = (local_p0 + local_p1) / 2.0;
        let seg_dir = (local_p1 - local_p0).normalize();

        let pitch = (-seg_dir.y).asin();
        let yaw = seg_dir.z.atan2(seg_dir.x);
        let rot = Quat::from_rotation_y(-yaw + std::f32::consts::FRAC_PI_2)
            * Quat::from_rotation_x(pitch);

        let is_bridge_seg = is_bridge_node[i] || is_bridge_node[i + 1];

        if !is_bridge_seg {
            // Flat gravel road!
            town.spawn((
                Mesh3d(meshes.add(Cuboid::new(2.2, 0.05, seg_dist))),
                MeshMaterial3d(gravel_mat.clone()),
                Transform::from_translation(seg_mid + Vec3::new(0.0, 0.015, 0.0))
                    .with_rotation(rot),
                Collider::cuboid(1.1, 0.025, seg_dist / 2.0),
                Bridge { start: p0, end: p1 }, // Track as bridge segment for NPC snapping
            ));
        } else {
            // Bridge segment!
            town.spawn((
                Mesh3d(meshes.add(Cuboid::new(2.2, 0.15, seg_dist))),
                MeshMaterial3d(bridge_deck_mat.clone()),
                Transform::from_translation(seg_mid).with_rotation(rot),
                Collider::cuboid(1.1, 0.075, seg_dist / 2.0),
                Bridge { start: p0, end: p1 }, // Track as bridge segment for NPC snapping
            ))
            .with_children(|deck| {
                // Handrails (left and right)
                deck.spawn((
                    Mesh3d(meshes.add(Cuboid::new(0.08, 0.08, seg_dist))),
                    MeshMaterial3d(bridge_rail_mat.clone()),
                    Transform::from_xyz(-1.05, 0.45, 0.0),
                ));
                deck.spawn((
                    Mesh3d(meshes.add(Cuboid::new(0.08, 0.08, seg_dist))),
                    MeshMaterial3d(bridge_rail_mat.clone()),
                    Transform::from_xyz(1.05, 0.45, 0.0),
                ));

                // Vert posts (4 corners of the segment)
                for side in [-1.0, 1.0] {
                    for z_offset in [-1.0, 1.0] {
                        deck.spawn((
                            Mesh3d(meshes.add(Cuboid::new(0.08, 0.45, 0.08))),
                            MeshMaterial3d(bridge_rail_mat.clone()),
                            Transform::from_xyz(
                                side * 1.05,
                                0.225,
                                z_offset * (seg_dist / 2.0 - 0.05),
                            ),
                        ));
                    }
                }

                // Vertical Support Pillars (extending down to the valley/gap floor)
                if let Some(h_below) = find_stable_ground_height(p_mid, voxel_world)
                    && h_below < p_mid.y - 0.2
                {
                    let pillar_h = p_mid.y - h_below;
                    for side in [-1.0, 1.0] {
                        deck.spawn((
                            Mesh3d(meshes.add(Cuboid::new(0.16, pillar_h, 0.16))),
                            MeshMaterial3d(bridge_deck_mat.clone()),
                            Transform::from_xyz(side * 0.9, -pillar_h / 2.0, 0.0),
                        ));
                    }
                }
            });
        }
    }
}

fn spawn_garden_field(
    commands: &mut Commands,
    meshes: &mut Assets<Mesh>,
    materials: &mut Assets<StandardMaterial>,
    pos: Vec3,
) {
    let dirt_mat = materials.add(StandardMaterial {
        base_color: Color::srgb(0.18, 0.12, 0.08),
        perceptual_roughness: 0.95,
        ..default()
    });
    let wood_mat = materials.add(StandardMaterial {
        base_color: Color::srgb(0.3, 0.18, 0.1),
        ..default()
    });
    let crop_stem_mat = materials.add(StandardMaterial {
        base_color: Color::srgb(0.2, 0.5, 0.1),
        ..default()
    });
    let crop_gold_mat = materials.add(StandardMaterial {
        base_color: Color::srgb(0.8, 0.6, 0.1),
        ..default()
    });

    commands
        .spawn((
            Transform::from_translation(pos),
            Visibility::default(),
            InheritedVisibility::default(),
        ))
        .with_children(|field| {
            // Tilled Soil Bed
            field.spawn((
                Mesh3d(meshes.add(Cuboid::new(6.0, 0.15, 8.0))),
                MeshMaterial3d(dirt_mat),
                Transform::from_xyz(0.0, 0.075, 0.0),
                Collider::cuboid(3.0, 0.075, 4.0),
            ));

            // Border Fence
            for side in [-1.0, 1.0] {
                // Horizontal rails
                field.spawn((
                    Mesh3d(meshes.add(Cuboid::new(0.1, 0.8, 8.0))),
                    MeshMaterial3d(wood_mat.clone()),
                    Transform::from_xyz(side * 3.0, 0.4, 0.0),
                ));
                field.spawn((
                    Mesh3d(meshes.add(Cuboid::new(6.0, 0.8, 0.1))),
                    MeshMaterial3d(wood_mat.clone()),
                    Transform::from_xyz(0.0, 0.4, side * 4.0),
                ));
            }

            // Rows of wheat crops!
            for x_idx in -2..=2 {
                let row_x = x_idx as f32 * 1.0;
                for z_idx in -3..=3 {
                    let crop_z = z_idx as f32 * 1.1;

                    // Crop stalk entity
                    field
                        .spawn((
                            Mesh3d(meshes.add(Cuboid::new(0.08, 0.45, 0.08))),
                            MeshMaterial3d(crop_stem_mat.clone()),
                            Transform::from_xyz(row_x, 0.35, crop_z),
                        ))
                        .with_children(|stalk| {
                            // Golden wheat head
                            stalk.spawn((
                                Mesh3d(meshes.add(Cuboid::new(0.12, 0.22, 0.12))),
                                MeshMaterial3d(crop_gold_mat.clone()),
                                Transform::from_xyz(0.0, 0.25, 0.0),
                            ));
                        });
                }
            }
        });
}

fn spawn_building(
    commands: &mut Commands,
    meshes: &mut Assets<Mesh>,
    materials: &mut Assets<StandardMaterial>,
    world_pos: Vec3,
    building_type: BuildingType,
    smoke_handle: Option<Handle<EffectAsset>>,
) {
    let size = match building_type {
        BuildingType::House => Vec3::new(4.5, 3.2, 4.5),
        BuildingType::Shop => Vec3::new(5.5, 3.6, 5.5),
        BuildingType::Forge => Vec3::new(5.5, 3.4, 6.5),
        BuildingType::Tavern => Vec3::new(8.5, 4.6, 6.5),
        BuildingType::GuardTower => Vec3::new(3.2, 6.0, 3.2),
        BuildingType::Plaza => Vec3::ZERO,
    };

    let main_mat = materials.add(StandardMaterial {
        base_color: Color::srgb(0.85, 0.80, 0.72),
        perceptual_roughness: 0.9,
        ..default()
    }); // Plaster
    let foundation_mat = materials.add(StandardMaterial {
        base_color: Color::srgb(0.28, 0.28, 0.28),
        perceptual_roughness: 0.95,
        ..default()
    }); // Cobblestone
    let framing_mat = materials.add(StandardMaterial {
        base_color: Color::srgb(0.22, 0.14, 0.08),
        perceptual_roughness: 0.9,
        ..default()
    }); // Dark Timber
    let roof_mat = materials.add(StandardMaterial {
        base_color: Color::srgb(0.48, 0.12, 0.12),
        perceptual_roughness: 0.8,
        ..default()
    }); // Terracotta Red Tiles
    let floor_mat = materials.add(StandardMaterial {
        base_color: Color::srgb(0.35, 0.22, 0.12),
        perceptual_roughness: 0.85,
        ..default()
    }); // Wood floor

    // Anchor entity (World Space) with 1.5x scale
    commands
        .spawn((
            Transform::from_translation(world_pos).with_scale(Vec3::splat(1.5)),
            Visibility::default(),
            InheritedVisibility::default(),
            SettlementBuilding {
                building_type,
                center: world_pos,
            },
        ))
        .with_children(|building| {
            if building_type == BuildingType::GuardTower {
                // Guard Tower specific structure
                // 1. Foundation Base Stone columns
                building.spawn((
                    Solid,
                    BuildingPart(size / 2.0),
                    Mesh3d(meshes.add(Cuboid::from_size(size))),
                    MeshMaterial3d(foundation_mat.clone()),
                    Transform::from_xyz(0.0, size.y / 2.0, 0.0),
                    Collider::cuboid(size.x / 2.0, size.y / 2.0, size.z / 2.0),
                ));

                // 2. Platform guard rails and watch deck
                building.spawn((
                    Mesh3d(meshes.add(Cuboid::new(size.x + 0.6, 0.3, size.z + 0.6))),
                    MeshMaterial3d(floor_mat.clone()),
                    Transform::from_xyz(0.0, size.y + 0.15, 0.0),
                    Collider::cuboid((size.x + 0.6) / 2.0, 0.15, (size.z + 0.6) / 2.0),
                ));

                // 3. Platform columns supporting the watch roof
                for dx in [-1.0, 1.0] {
                    for dz in [-1.0, 1.0] {
                        building.spawn((
                            Mesh3d(meshes.add(Cuboid::new(0.12, 1.6, 0.12))),
                            MeshMaterial3d(framing_mat.clone()),
                            Transform::from_xyz(
                                dx * (size.x / 2.0 + 0.1),
                                size.y + 1.1,
                                dz * (size.z / 2.0 + 0.1),
                            ),
                        ));
                    }
                }

                // 4. Slanted Watch Tower Cap Roof
                building.spawn((
                    Mesh3d(meshes.add(Cone::default())),
                    MeshMaterial3d(roof_mat.clone()),
                    Transform::from_xyz(0.0, size.y + 2.4, 0.0).with_scale(Vec3::new(
                        size.x + 1.0,
                        1.2,
                        size.z + 1.0,
                    )),
                ));

                return;
            }

            // Standard Medieval Houses
            // 1. Cobblestone Foundation
            building.spawn((
                Mesh3d(meshes.add(Cuboid::new(size.x + 0.2, 1.5, size.z + 0.2))),
                MeshMaterial3d(foundation_mat),
                Transform::from_xyz(0.0, -0.65, 0.0),
                Collider::cuboid((size.x + 0.2) / 2.0, 0.75, (size.z + 0.2) / 2.0),
            ));

            // 2. Floor
            building.spawn((
                Mesh3d(meshes.add(Cuboid::new(size.x, 0.1, size.z))),
                MeshMaterial3d(floor_mat),
                Transform::from_xyz(0.0, 0.05, 0.0),
            ));

            // 3. Corner Timber Support Pillars
            let col_h = size.y;
            for dx in [-1.0, 1.0] {
                for dz in [-1.0, 1.0] {
                    building.spawn((
                        Solid,
                        BuildingPart(Vec3::new(0.35, col_h, 0.35) / 2.0),
                        Mesh3d(meshes.add(Cuboid::new(0.35, col_h, 0.35))),
                        MeshMaterial3d(framing_mat.clone()),
                        Transform::from_xyz(
                            dx * (size.x / 2.0 - 0.15),
                            col_h / 2.0,
                            dz * (size.z / 2.0 - 0.15),
                        ),
                    ));
                }
            }

            // 4. Plaster Walls (with Window Openings & Doors)
            let wall_t = 0.2;

            // Back Wall
            let bw_size = Vec3::new(size.x - 0.6, col_h, wall_t);
            building.spawn((
                Solid,
                BuildingPart(bw_size / 2.0),
                Mesh3d(meshes.add(Cuboid::from_size(bw_size))),
                MeshMaterial3d(main_mat.clone()),
                Transform::from_xyz(0.0, col_h / 2.0, -size.z / 2.0 + wall_t / 2.0),
                Collider::cuboid(bw_size.x / 2.0, bw_size.y / 2.0, bw_size.z / 2.0),
            ));

            // Left & Right Walls
            for side in [-1.0, 1.0] {
                let lr_size = Vec3::new(wall_t, col_h, size.z - 0.6);
                building.spawn((
                    Solid,
                    BuildingPart(lr_size / 2.0),
                    Mesh3d(meshes.add(Cuboid::from_size(lr_size))),
                    MeshMaterial3d(main_mat.clone()),
                    Transform::from_xyz(side * (size.x / 2.0 - wall_t / 2.0), col_h / 2.0, 0.0),
                    Collider::cuboid(lr_size.x / 2.0, lr_size.y / 2.0, lr_size.z / 2.0),
                ));
            }

            // Front Wall (with Door Hole)
            let door_w = 1.1;
            let door_h = 2.1;
            let side_panel_w = (size.x - door_w - 0.6) / 2.0;

            // Front panels
            for side in [-1.0, 1.0] {
                let panel_size = Vec3::new(side_panel_w, col_h, wall_t);
                building.spawn((
                    Solid,
                    BuildingPart(panel_size / 2.0),
                    Mesh3d(meshes.add(Cuboid::from_size(panel_size))),
                    MeshMaterial3d(main_mat.clone()),
                    Transform::from_xyz(
                        side * (size.x / 2.0 - side_panel_w / 2.0 - 0.3),
                        col_h / 2.0,
                        size.z / 2.0 - wall_t / 2.0,
                    ),
                    Collider::cuboid(panel_size.x / 2.0, panel_size.y / 2.0, panel_size.z / 2.0),
                ));
            }
            // Top lintel
            let tl_size = Vec3::new(door_w, col_h - door_h, wall_t);
            building.spawn((
                Solid,
                BuildingPart(tl_size / 2.0),
                Mesh3d(meshes.add(Cuboid::from_size(tl_size))),
                MeshMaterial3d(main_mat.clone()),
                Transform::from_xyz(0.0, (col_h + door_h) / 2.0, size.z / 2.0 - wall_t / 2.0),
                Collider::cuboid(tl_size.x / 2.0, tl_size.y / 2.0, tl_size.z / 2.0),
            ));

            // 5. Tudor Timber Horizontal Beams (Top frame)
            building.spawn((
                Mesh3d(meshes.add(Cuboid::new(size.x, 0.25, size.z))),
                MeshMaterial3d(framing_mat.clone()),
                Transform::from_xyz(0.0, col_h + 0.125, 0.0),
            ));

            // 6. Glowing Windows (Cyan/Yellow Emissive)
            let win_color = materials.add(StandardMaterial {
                base_color: Color::srgb(1.0, 0.9, 0.6),
                emissive: LinearRgba::new(2.5, 2.0, 0.5, 1.0),
                ..default()
            });

            // Spawn small window panes on side walls
            for side in [-1.0, 1.0] {
                building.spawn((
                    Mesh3d(meshes.add(Cuboid::new(0.12, 0.8, 1.0))),
                    MeshMaterial3d(win_color.clone()),
                    Transform::from_xyz(side * (size.x / 2.0 - 0.08), col_h * 0.55, 0.0),
                ));
            }

            // 7. Slanted Cozy Gabled Roof
            let roof_width = size.x / 2.0 + 0.6;
            let roof_length = size.z + 0.45;
            let _peak_y = col_h + (size.x * 0.28);

            // Left slant
            building.spawn((
                Mesh3d(meshes.add(Cuboid::new(roof_width, 0.15, roof_length))),
                MeshMaterial3d(roof_mat.clone()),
                Transform::from_xyz(-size.x / 4.0 - 0.08, col_h + (size.x * 0.14) + 0.22, 0.0)
                    .with_rotation(Quat::from_rotation_z(0.52)), // ~30 degrees
            ));
            // Right slant
            building.spawn((
                Mesh3d(meshes.add(Cuboid::new(roof_width, 0.15, roof_length))),
                MeshMaterial3d(roof_mat.clone()),
                Transform::from_xyz(size.x / 4.0 + 0.08, col_h + (size.x * 0.14) + 0.22, 0.0)
                    .with_rotation(Quat::from_rotation_z(-0.52)), // ~-30 degrees
            ));

            // Gable timber triangles (Front and back panels to close roof)
            for side in [-1.0, 1.0] {
                building.spawn((
                    Mesh3d(meshes.add(Cuboid::new(size.x - 0.3, size.x * 0.28, 0.1))),
                    MeshMaterial3d(framing_mat.clone()),
                    Transform::from_xyz(
                        0.0,
                        col_h + (size.x * 0.14) + 0.08,
                        side * (size.z / 2.0 - 0.15),
                    ),
                ));
            }

            // 8. Stone Chimney with Cozy rising smoke!
            let chimney_x = size.x / 2.0 - 0.6;
            let chimney_z = -size.z / 2.0 + 0.6;
            building.spawn((
                Mesh3d(meshes.add(Cuboid::new(0.7, col_h + 1.2, 0.7))),
                MeshMaterial3d(materials.add(StandardMaterial {
                    base_color: Color::srgb(0.3, 0.3, 0.3),
                    perceptual_roughness: 0.95,
                    ..default()
                })),
                Transform::from_xyz(chimney_x, (col_h + 1.2) / 2.0, chimney_z),
                Collider::cuboid(0.35, (col_h + 1.2) / 2.0, 0.35),
            ));

            // Connect the GPU chimney smoke particle emitter
            if let Some(smoke_asset) = smoke_handle {
                building.spawn((
                    bevy_hanabi::ParticleEffect {
                        handle: smoke_asset,
                        ..default()
                    },
                    Transform::from_xyz(chimney_x, col_h + 1.35, chimney_z),
                ));
            }

            // 9. Front door glowing lanterns
            building
                .spawn((
                    Mesh3d(meshes.add(Cuboid::new(0.12, 0.12, 0.3))),
                    MeshMaterial3d(framing_mat.clone()),
                    Transform::from_xyz(0.7, door_h + 0.2, size.z / 2.0 - 0.1),
                ))
                .with_children(|lantern| {
                    lantern.spawn((
                        Mesh3d(meshes.add(Cuboid::new(0.24, 0.3, 0.24))),
                        MeshMaterial3d(win_color.clone()),
                        Transform::from_xyz(0.0, -0.22, 0.12),
                    ));
                    lantern.spawn((
                        PointLight {
                            color: Color::srgb(1.0, 0.85, 0.5),
                            intensity: 6000.0,
                            range: 8.0,
                            shadows_enabled: false,
                            ..default()
                        },
                        Transform::from_xyz(0.0, -0.22, 0.12),
                    ));
                });

            // 10. Cozy Interior Furniture and Props
            match building_type {
                BuildingType::House | BuildingType::Tavern => {
                    // Bed
                    building
                        .spawn((
                            Mesh3d(meshes.add(Cuboid::new(1.2, 0.4, 2.0))),
                            MeshMaterial3d(materials.add(StandardMaterial {
                                base_color: Color::srgb(0.72, 0.15, 0.15),
                                perceptual_roughness: 0.8,
                                ..default()
                            })),
                            Transform::from_xyz(-size.x / 2.0 + 1.0, 0.2, -size.z / 2.0 + 1.3),
                        ))
                        .with_children(|bed| {
                            // Bed pillow
                            bed.spawn((
                                Mesh3d(meshes.add(Cuboid::new(1.0, 0.15, 0.5))),
                                MeshMaterial3d(materials.add(StandardMaterial {
                                    base_color: Color::WHITE,
                                    ..default()
                                })),
                                Transform::from_xyz(0.0, 0.22, -0.7),
                            ));
                        });
                }
                _ => {}
            }

            match building_type {
                BuildingType::Tavern => {
                    // Tavern Bar Counter
                    building.spawn((
                        Mesh3d(meshes.add(Cuboid::new(4.0, 0.95, 0.6))),
                        MeshMaterial3d(materials.add(StandardMaterial {
                            base_color: Color::srgb(0.28, 0.16, 0.1),
                            perceptual_roughness: 0.85,
                            ..default()
                        })),
                        Transform::from_xyz(1.5, 0.475, 1.2),
                        Collider::cuboid(2.0, 0.475, 0.3),
                    ));

                    // Benches and tables
                    let table_mat = materials.add(StandardMaterial {
                        base_color: Color::srgb(0.36, 0.22, 0.14),
                        perceptual_roughness: 0.9,
                        ..default()
                    });
                    for side in [-1.0, 1.0] {
                        // Tables
                        building.spawn((
                            Mesh3d(meshes.add(Cuboid::new(1.3, 0.75, 1.3))),
                            MeshMaterial3d(table_mat.clone()),
                            Transform::from_xyz(side * 2.2, 0.375, -1.2),
                            Collider::cuboid(0.65, 0.375, 0.65),
                        ));
                        // Bench seats
                        for offset_z in [-0.95, 0.95] {
                            building.spawn((
                                Mesh3d(meshes.add(Cuboid::new(1.1, 0.45, 0.4))),
                                MeshMaterial3d(framing_mat.clone()),
                                Transform::from_xyz(side * 2.2, 0.225, -1.2 + offset_z),
                                Collider::cuboid(0.55, 0.225, 0.2),
                            ));
                        }
                    }
                }
                BuildingType::Forge => {
                    // Smelting Stone Furnace (hot coal center)
                    building
                        .spawn((
                            Mesh3d(meshes.add(Cuboid::new(1.4, 1.6, 1.4))),
                            MeshMaterial3d(materials.add(StandardMaterial {
                                base_color: Color::srgb(0.22, 0.22, 0.22),
                                perceptual_roughness: 0.95,
                                ..default()
                            })),
                            Transform::from_xyz(-size.x / 2.0 + 1.2, 0.8, -size.z / 2.0 + 1.2),
                            Collider::cuboid(0.7, 0.8, 0.7),
                        ))
                        .with_children(|furnace| {
                            // Red hot fire core
                            furnace.spawn((
                                Mesh3d(meshes.add(Cuboid::new(0.8, 0.6, 0.15))),
                                MeshMaterial3d(materials.add(StandardMaterial {
                                    base_color: Color::srgb(1.0, 0.3, 0.0),
                                    emissive: LinearRgba::new(6.0, 1.8, 0.1, 1.0),
                                    ..default()
                                })),
                                Transform::from_xyz(0.0, -0.2, 0.65),
                            ));
                            // Fire point light
                            furnace.spawn((
                                PointLight {
                                    color: Color::srgb(1.0, 0.5, 0.1),
                                    intensity: 9000.0,
                                    range: 7.0,
                                    shadows_enabled: false,
                                    ..default()
                                },
                                Transform::from_xyz(0.0, -0.2, 0.8),
                            ));
                        });

                    // Solid Anvil on wood base
                    building
                        .spawn((
                            Mesh3d(meshes.add(Cuboid::new(0.6, 0.5, 0.6))),
                            MeshMaterial3d(materials.add(StandardMaterial {
                                base_color: Color::srgb(0.42, 0.26, 0.15),
                                perceptual_roughness: 0.9,
                                ..default()
                            })),
                            Transform::from_xyz(0.5, 0.25, 0.5),
                            Collider::cuboid(0.3, 0.25, 0.3),
                        ))
                        .with_children(|base| {
                            // Black heavy steel top
                            base.spawn((
                                Mesh3d(meshes.add(Cuboid::new(0.7, 0.3, 0.35))),
                                MeshMaterial3d(materials.add(StandardMaterial {
                                    base_color: Color::srgb(0.1, 0.1, 0.12),
                                    perceptual_roughness: 0.7,
                                    metallic: 0.8,
                                    ..default()
                                })),
                                Transform::from_xyz(0.0, 0.4, 0.0),
                            ));
                        });
                }
                BuildingType::Shop => {
                    // Shop Counter
                    building.spawn((
                        Mesh3d(meshes.add(Cuboid::new(3.2, 0.9, 0.5))),
                        MeshMaterial3d(materials.add(StandardMaterial {
                            base_color: Color::srgb(0.38, 0.24, 0.15),
                            perceptual_roughness: 0.85,
                            ..default()
                        })),
                        Transform::from_xyz(0.0, 0.45, 0.6),
                        Collider::cuboid(1.6, 0.45, 0.25),
                    ));

                    // Back wood shelves with display boxes
                    building.spawn((
                        Mesh3d(meshes.add(Cuboid::new(size.x - 1.2, 1.8, 0.3))),
                        MeshMaterial3d(framing_mat.clone()),
                        Transform::from_xyz(0.0, 0.9, -size.z / 2.0 + 0.3),
                    ));
                }
                _ => {}
            }
        });
}
