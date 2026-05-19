use bevy::prelude::*;
use bevy_rapier3d::prelude::*;
use crate::entities::npc::{spawn_npc, NPCRole};
use crate::world::noise_generator::NoiseGenerator;
use crate::world::manager::{find_stable_ground_height};
use bevy_voxel_world::prelude::*;
use std::collections::HashSet;

pub struct SettlementPlugin;

impl Plugin for SettlementPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<SettlementRegistry>();
    }
}

#[derive(Resource, Default)]
pub struct SettlementRegistry {
    pub positions: HashSet<IVec3>,
}

#[derive(Component)]
pub struct ProcessedSettlement;

#[derive(Component)]
pub struct Solid;

#[derive(Component)]
pub struct BuildingPart(pub Vec3); // Store half-size for AABB

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum BuildingType {
    House,
    Shop,
    Forge,
    Tavern,
}

pub fn spawn_settlements(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    voxel_world: VoxelWorld<NoiseGenerator>,
    mut registry: ResMut<SettlementRegistry>,
    query: Query<(Entity, &Chunk<NoiseGenerator>), Without<ProcessedSettlement>>,
) {
    for (entity, chunk_comp) in query.iter() {
        let chunk_pos = chunk_comp.position;
        
        if let Ok(mut chunk_cmd) = commands.get_entity(entity) {
            chunk_cmd.insert(ProcessedSettlement);

            if chunk_pos.y < -1 || chunk_pos.y > 4 { continue; } // Reasonable vertical range for towns
            
            // Optimized pseudo-random check for settlement spawning
            let chunk_hash = (chunk_pos.x.wrapping_mul(73856093) ^ chunk_pos.z.wrapping_mul(19349663)).abs();
            let is_settlement_chunk = (chunk_hash % 10) == 0; 
            if is_settlement_chunk {
                // println!("SETTLEMENT CHUNK CANDIDATE at {:?}", chunk_pos);
            }

            // Use coordinate-based persistence
            if is_settlement_chunk && (chunk_pos.x != 0 || chunk_pos.z != 0) {
                if registry.positions.contains(&chunk_pos) {
                    continue; // Already spawned a town here
                }
                registry.positions.insert(chunk_pos);
                let town_base_x = (chunk_pos.x * 16 + 8) as f32;
                let town_base_z = (chunk_pos.z * 16 + 8) as f32;
                
                let get_surface = |x: f32, z: f32| -> Option<f32> {
                    find_stable_ground_height(Vec3::new(x, (chunk_pos.y * 16 + 16) as f32, z), &voxel_world)
                };

                if let Some(h) = get_surface(town_base_x, town_base_z) {
                    if (h / 16.0).floor() as i32 != chunk_pos.y { 
                        continue; 
                    }
                    // println!("CHECKING SETTLEMENT FOR CHUNK {:?}", chunk_pos);
                    if h < 16.0 { // Sea level is 15.0
                        // println!("  SETTLEMENT at {:?} skipped: underwater ({:.1})", chunk_pos, h);
                        continue;
                    }
                    // 1. Tavern
                    let world_town_pos = Vec3::new(town_base_x, h, town_base_z);
                    spawn_building(&mut commands, &mut meshes, &mut materials, world_town_pos, BuildingType::Tavern);
                    spawn_npc(&mut commands, &mut meshes, &mut materials, world_town_pos + Vec3::new(-1.5, 0.5, -1.0), NPCRole::Barkeeper, Some(world_town_pos));

                    // 2. Shop
                    let s_wx = town_base_x + 6.0;
                    let s_wz = town_base_z;
                    if let Some(sh) = get_surface(s_wx, s_wz) {
                        let shop_world_pos = Vec3::new(s_wx, sh, s_wz);
                        spawn_building(&mut commands, &mut meshes, &mut materials, shop_world_pos, BuildingType::Shop);
                        spawn_npc(&mut commands, &mut meshes, &mut materials, shop_world_pos + Vec3::new(0.0, 1.0, 0.0), NPCRole::Merchant, Some(shop_world_pos));
                    }

                    // 3. Forge
                    let f_wx = town_base_x - 6.0;
                    let f_wz = town_base_z;
                    if let Some(fh) = get_surface(f_wx, f_wz) {
                        let forge_world_pos = Vec3::new(f_wx, fh, f_wz);
                        spawn_building(&mut commands, &mut meshes, &mut materials, forge_world_pos, BuildingType::Forge);
                        spawn_npc(&mut commands, &mut meshes, &mut materials, forge_world_pos + Vec3::new(0.0, 1.0, 1.0), NPCRole::Blacksmith, Some(forge_world_pos));
                    }

                    // 4. Farm (Farmer)
                    let fa_wx = town_base_x;
                    let fa_wz = town_base_z - 8.0;
                    if let Some(fah) = get_surface(fa_wx, fa_wz) {
                        let farm_world_pos = Vec3::new(fa_wx, fah, fa_wz);
                        spawn_building(&mut commands, &mut meshes, &mut materials, farm_world_pos, BuildingType::House);
                        spawn_npc(&mut commands, &mut meshes, &mut materials, farm_world_pos + Vec3::new(-1.2, 0.5, -0.8), NPCRole::Farmer, Some(farm_world_pos));
                    }

                    // 5. Houses and Citizens
                    for i in 0..2 {
                        let h_wx = town_base_x + if i == 0 { 2.0 } else { -2.0 };
                        let h_wz = town_base_z + 6.0;
                        if let Some(hh) = get_surface(h_wx, h_wz) {
                            let h_world_pos = Vec3::new(h_wx, hh, h_wz);
                            spawn_building(&mut commands, &mut meshes, &mut materials, h_world_pos, BuildingType::House);
                            spawn_npc(&mut commands, &mut meshes, &mut materials, h_world_pos + Vec3::new(-1.2, 0.5, -0.8), NPCRole::Citizen, None);
                        }
                    }

                    // Guards
                    spawn_npc(&mut commands, &mut meshes, &mut materials, world_town_pos + Vec3::new(7.0, 1.0, 7.0), NPCRole::Guard, None);
                    spawn_npc(&mut commands, &mut meshes, &mut materials, world_town_pos + Vec3::new(-7.0, 1.0, -7.0), NPCRole::Guard, None);
                }
            }
        }
    }
}

fn spawn_building(
    commands: &mut Commands,
    meshes: &mut Assets<Mesh>,
    materials: &mut Assets<StandardMaterial>,
    world_pos: Vec3,
    building_type: BuildingType,
) {
    let (size, color, roof_color) = match building_type {
        BuildingType::House => (Vec3::new(4.0, 3.0, 4.0), Color::srgb(0.45, 0.35, 0.25), Color::srgb(0.5, 0.1, 0.1)),
        BuildingType::Shop => (Vec3::new(5.0, 3.5, 5.0), Color::srgb(0.6, 0.5, 0.4), Color::srgb(0.1, 0.3, 0.1)),
        BuildingType::Forge => (Vec3::new(5.0, 3.2, 6.0), Color::srgb(0.3, 0.3, 0.35), Color::srgb(0.1, 0.1, 0.1)),
        BuildingType::Tavern => (Vec3::new(8.0, 4.5, 6.0), Color::srgb(0.5, 0.3, 0.2), Color::srgb(0.3, 0.1, 0.05)),
    };

    let main_mat = materials.add(StandardMaterial { base_color: color, ..default() });
    let roof_mat = materials.add(StandardMaterial { base_color: roof_color, ..default() });
    let floor_mat = materials.add(StandardMaterial { base_color: Color::srgb(0.3, 0.2, 0.1), ..default() });

    // Anchor entity for the building (Now in World Space)
    commands.spawn((
        Transform::from_translation(world_pos),
        Visibility::default(),
        InheritedVisibility::default(),
    )).with_children(|building| {
                // 1. Floor
                building.spawn((
                    Mesh3d(meshes.add(Cuboid::new(size.x, 0.1, size.z))),
                    MeshMaterial3d(floor_mat),
                    Transform::from_xyz(0.0, 0.05, 0.0),
                    Collider::cuboid(size.x / 2.0, 0.05, size.z / 2.0),
                ));

                // 2. Walls (0.2 thickness)
                let wall_h = size.y;
                let wall_t = 0.2;

                // Back Wall
                let bw_size = Vec3::new(size.x, wall_h, wall_t);
                building.spawn((
                    Solid,
                    BuildingPart(bw_size / 2.0),
                    Mesh3d(meshes.add(Cuboid::from_size(bw_size))),
                    MeshMaterial3d(main_mat.clone()),
                    Transform::from_xyz(0.0, wall_h / 2.0, -size.z/2.0 + wall_t/2.0),
                    Collider::cuboid(bw_size.x / 2.0, bw_size.y / 2.0, bw_size.z / 2.0),
                ));

                // Left & Right Walls
                for side in [-1.0, 1.0] {
                    let lr_size = Vec3::new(wall_t, wall_h, size.z - wall_t * 2.0);
                    building.spawn((
                        Solid,
                        BuildingPart(lr_size / 2.0),
                        Mesh3d(meshes.add(Cuboid::from_size(lr_size))),
                        MeshMaterial3d(main_mat.clone()),
                        Transform::from_xyz(side * (size.x/2.0 - wall_t/2.0), wall_h / 2.0, 0.0),
                        Collider::cuboid(lr_size.x / 2.0, lr_size.y / 2.0, lr_size.z / 2.0),
                    ));
                }

                // Front Wall (with Door Hole)
                let door_w = 1.0;
                let door_h = 2.0;
                let side_panel_w = (size.x - door_w) / 2.0;
                
                // Left panel
                let lp_size = Vec3::new(side_panel_w, wall_h, wall_t);
                building.spawn((
                    Solid,
                    BuildingPart(lp_size / 2.0),
                    Mesh3d(meshes.add(Cuboid::from_size(lp_size))),
                    MeshMaterial3d(main_mat.clone()),
                    Transform::from_xyz(-(size.x/2.0 - side_panel_w/2.0), wall_h / 2.0, size.z/2.0 - wall_t/2.0),
                    Collider::cuboid(lp_size.x / 2.0, lp_size.y / 2.0, lp_size.z / 2.0),
                ));
                // Right panel
                let rp_size = Vec3::new(side_panel_w, wall_h, wall_t);
                building.spawn((
                    Solid,
                    BuildingPart(rp_size / 2.0),
                    Mesh3d(meshes.add(Cuboid::from_size(rp_size))),
                    MeshMaterial3d(main_mat.clone()),
                    Transform::from_xyz(size.x/2.0 - side_panel_w/2.0, wall_h / 2.0, size.z/2.0 - wall_t/2.0),
                    Collider::cuboid(rp_size.x / 2.0, rp_size.y / 2.0, rp_size.z / 2.0),
                ));
                // Top lintel
                let tl_size = Vec3::new(door_w, wall_h - door_h, wall_t);
                building.spawn((
                    Solid,
                    BuildingPart(tl_size / 2.0),
                    Mesh3d(meshes.add(Cuboid::from_size(tl_size))),
                    MeshMaterial3d(main_mat.clone()),
                    Transform::from_xyz(0.0, (wall_h + door_h) / 2.0, size.z/2.0 - wall_t/2.0),
                    Collider::cuboid(tl_size.x / 2.0, tl_size.y / 2.0, tl_size.z / 2.0),
                ));

                // 3. Roof
                building.spawn((
                    Mesh3d(meshes.add(Cuboid::new(size.x + 0.4, 0.4, size.z + 0.4))),
                    MeshMaterial3d(roof_mat),
                    Transform::from_xyz(0.0, wall_h + 0.2, 0.0),
                ));

                // 4. Interior Furniture
                match building_type {
                    BuildingType::House | BuildingType::Tavern => {
                        // Bed
                        building.spawn((
                            Mesh3d(meshes.add(Cuboid::new(1.0, 0.4, 1.8))),
                            MeshMaterial3d(materials.add(StandardMaterial { base_color: Color::srgb(0.8, 0.1, 0.1), ..default() })),
                            Transform::from_xyz(-size.x/2.0 + 0.8, 0.2, -size.z/2.0 + 1.2),
                        ));
                    }
                    _ => {}
                }

                // Previous Decor
                match building_type {
                    BuildingType::Forge => {
                        building.spawn((
                            Mesh3d(meshes.add(Cuboid::new(0.6, 0.4, 0.4))),
                            MeshMaterial3d(materials.add(StandardMaterial { base_color: Color::BLACK, ..default() })),
                            Transform::from_xyz(0.0, 0.2, 1.0),
                        ));
                    }
                    BuildingType::Shop => {
                        building.spawn((
                            Mesh3d(meshes.add(Cuboid::new(size.x - 1.0, 0.8, 0.5))),
                            MeshMaterial3d(materials.add(StandardMaterial { base_color: Color::srgb(0.4, 0.2, 0.1), ..default() })),
                            Transform::from_xyz(0.0, 0.4, 0.0),
                        ));
                    }
                    _ => {}
                }
            });
}
