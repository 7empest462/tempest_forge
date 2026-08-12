use crate::machinery::components::{MachineLogic, MachineryRegistry, PowerNode, PowerType};
use crate::player::camera::{MechSuit, Player};
use crate::player::combat::WeaponState;
use crate::ui::UiState;
use crate::voxel::BlockType;
use crate::world::noise_generator::NoiseGenerator;
use bevy::ecs::system::SystemParam;
use bevy::prelude::*;
use bevy_voxel_world::prelude::*;
use rand::RngExt;
use serde::{Deserialize, Serialize};

// Custom serialization module for FxHashMap with IVec3 keys
mod blocks_serde {
    use crate::voxel::BlockType;
    use bevy::prelude::IVec3;
    use rustc_hash::FxHashMap;
    use serde::{Deserialize, Deserializer, Serialize, Serializer};

    pub fn serialize<S>(map: &FxHashMap<IVec3, BlockType>, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let vec: Vec<(IVec3, BlockType)> = map.iter().map(|(k, v)| (*k, *v)).collect();
        vec.serialize(serializer)
    }

    pub fn deserialize<'de, D>(deserializer: D) -> Result<FxHashMap<IVec3, BlockType>, D::Error>
    where
        D: Deserializer<'de>,
    {
        let vec: Vec<(IVec3, BlockType)> = Vec::deserialize(deserializer)?;
        Ok(vec.into_iter().collect())
    }
}

// Custom serialization module for FxHashMap with enum keys
mod resources_serde {
    use crate::voxel::BlockType;
    use rustc_hash::FxHashMap;
    use serde::{Deserialize, Deserializer, Serialize, Serializer};

    pub fn serialize<S>(map: &FxHashMap<BlockType, u32>, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let vec: Vec<(BlockType, u32)> = map.iter().map(|(k, v)| (*k, *v)).collect();
        vec.serialize(serializer)
    }

    pub fn deserialize<'de, D>(deserializer: D) -> Result<FxHashMap<BlockType, u32>, D::Error>
    where
        D: Deserializer<'de>,
    {
        let vec: Vec<(BlockType, u32)> = Vec::deserialize(deserializer)?;
        Ok(vec.into_iter().collect())
    }
}

pub struct InteractionPlugin;

impl Plugin for InteractionPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<SelectedBlock>()
            .init_resource::<PlacementState>()
            .init_resource::<MiningProgress>()
            .init_resource::<SelectedEntity>()
            .init_resource::<Inventory>()
            .init_resource::<WorldPersistence>()
            .add_systems(OnEnter(crate::GameState::InGame), setup_ui)
            .add_systems(
                Update,
                (
                    update_interaction,
                    draw_selection_highlight,
                    update_mining_ui,
                    update_particles,
                    toggle_doors,
                )
                    .run_if(in_state(crate::GameState::InGame)),
            );
    }
}

#[derive(Component)]
pub struct Particle {
    pub velocity: Vec3,
    pub lifetime: Timer,
}

#[derive(Component)]
pub struct Door {
    pub open: bool,
    pub hinge_side: f32, // -1.0 or 1.0
}

#[derive(Component)]
pub struct Slope;

#[derive(Resource, Default)]
pub struct MiningProgress {
    pub progress: f32, // 0.0 to 1.0
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum ArmorTier {
    #[default]
    None,
    Iron,
    Gold,
}

use rustc_hash::FxHashMap;

#[derive(Resource, Serialize, Deserialize, Clone)]
pub struct Inventory {
    #[serde(with = "resources_serde")]
    pub resources: FxHashMap<BlockType, u32>,
    #[serde(default = "default_true")]
    pub has_bow: bool,
    #[serde(default = "default_true")]
    pub has_axe: bool,
    #[serde(default = "default_true")]
    pub has_pickaxe: bool,
    #[serde(default)]
    pub armor_tier: ArmorTier,
    #[serde(default = "default_true")]
    pub has_sword: bool,
    #[serde(default = "default_true")]
    pub has_iron_pickaxe: bool,
    #[serde(default)]
    pub has_gold_pickaxe: bool,
    #[serde(default = "default_true")]
    pub has_iron_axe: bool,
    #[serde(default)]
    pub has_gold_axe: bool,
    #[serde(default = "default_true")]
    pub has_iron_sword: bool,
    #[serde(default)]
    pub has_gold_sword: bool,
}

fn default_true() -> bool {
    true
}

impl Default for Inventory {
    fn default() -> Self {
        Self {
            resources: FxHashMap::default(),
            has_bow: true,
            has_axe: true,
            has_pickaxe: true,
            armor_tier: ArmorTier::None,
            has_sword: true,
            has_iron_pickaxe: true,
            has_gold_pickaxe: false,
            has_iron_axe: true,
            has_gold_axe: false,
            has_iron_sword: true,
            has_gold_sword: false,
        }
    }
}

#[derive(Resource, Default, Serialize, Deserialize, Clone)]
pub struct WorldPersistence {
    #[serde(with = "blocks_serde")]
    pub modified_blocks: FxHashMap<IVec3, BlockType>,
}

fn setup_ui(mut commands: Commands) {
    commands
        .spawn(Node {
            width: Val::Percent(100.0),
            height: Val::Percent(100.0),
            align_items: AlignItems::Center,
            justify_content: JustifyContent::Center,
            ..default()
        })
        .with_children(|parent| {
            parent.spawn((
                Node {
                    width: Val::Px(4.0),
                    height: Val::Px(4.0),
                    ..default()
                },
                BackgroundColor(Color::WHITE),
            ));

            // Progress Bar Container
            parent
                .spawn((
                    Node {
                        position_type: PositionType::Absolute,
                        bottom: Val::Px(50.0),
                        width: Val::Px(200.0),
                        height: Val::Px(10.0),
                        border: UiRect::all(Val::Px(2.0)),
                        ..default()
                    },
                    BorderColor::all(Color::BLACK),
                    BackgroundColor(Color::from(bevy::color::palettes::css::GRAY)),
                ))
                .with_children(|bar_bg| {
                    bar_bg.spawn((
                        Node {
                            width: Val::Percent(0.0),
                            height: Val::Percent(100.0),
                            ..default()
                        },
                        BackgroundColor(Color::from(bevy::color::palettes::css::GREEN)),
                        MiningProgressBar,
                    ));
                });
        });
}

#[derive(Component)]
struct MiningProgressBar;

fn update_mining_ui(
    mining_progress: Res<MiningProgress>,
    mut query: Query<&mut Node, With<MiningProgressBar>>,
) {
    if let Ok(mut node) = query.single_mut() {
        node.width = Val::Percent(mining_progress.progress * 100.0);
    }
}

#[derive(Resource, Default)]
pub struct SelectedBlock {
    pub position: Option<IVec3>,
    pub normal: Option<IVec3>,
}

#[derive(Resource, Default)]
pub struct SelectedEntity {
    pub entity: Option<Entity>,
}

#[derive(Resource)]
pub struct PlacementState {
    pub current_block: BlockType,
}

impl Default for PlacementState {
    fn default() -> Self {
        Self {
            current_block: BlockType::Stone,
        }
    }
}

use crate::world::water::MainCamera;

#[derive(SystemParam)]
pub struct InteractionParams<'w, 's> {
    pub player_query: Query<'w, 's, (&'static Transform, &'static MechSuit), With<Player>>,
    pub camera_query: Query<'w, 's, (&'static Camera, &'static GlobalTransform), With<MainCamera>>,
    pub machinery_registry: ResMut<'w, MachineryRegistry>,
    pub mouse_input: Res<'w, ButtonInput<MouseButton>>,
    pub placement: ResMut<'w, PlacementState>,
    pub selection: ResMut<'w, SelectedBlock>,
    pub entity_selection: ResMut<'w, SelectedEntity>,
    pub mining_progress: ResMut<'w, MiningProgress>,
    pub inventory: ResMut<'w, Inventory>,
    pub world_persistence: ResMut<'w, WorldPersistence>,
    pub asset_server: Res<'w, AssetServer>,
    pub commands: Commands<'w, 's>,
    pub meshes: ResMut<'w, Assets<Mesh>>,
    pub materials: ResMut<'w, Assets<StandardMaterial>>,
    pub time: Res<'w, Time>,
    pub ui_state: Res<'w, UiState>,
    pub weapon: Res<'w, WeaponState>,
    pub health_query: Query<
        'w,
        's,
        (
            Entity,
            &'static Transform,
            &'static mut crate::player::combat::Health,
        ),
        (
            With<crate::player::combat::Hittable>,
            Without<crate::player::combat::DamageEvent>,
        ),
    >,
    pub gamepads: Query<'w, 's, &'static Gamepad>,
    pub water_impulses: MessageWriter<'w, crate::world::water::WaterImpulseEvent>,
}

fn update_interaction(
    mut params: InteractionParams,
    mut voxel_world: VoxelWorld<NoiseGenerator>,
    mut entity_spark_cache: Local<Option<(Handle<Mesh>, Handle<StandardMaterial>)>>,
    mut block_spark_cache: Local<Option<(Handle<Mesh>, Handle<StandardMaterial>)>>,
    mut splinter_cache: Local<FxHashMap<BlockType, (Handle<Mesh>, Handle<StandardMaterial>)>>,
) {
    // Skip all interaction when menus are open (don't access egui context - it breaks button clicks)
    if params.ui_state.show_inventory || params.ui_state.show_pause_menu {
        return;
    }

    let (_player_transform, mech) = params.player_query.single().expect("Player must exist");
    let (camera, camera_transform) = params.camera_query.single().expect("Camera must exist");

    // Switch block type
    // Raycast logic

    // Raycast logic
    let viewport_size = camera
        .logical_viewport_size()
        .unwrap_or(Vec2::new(1920.0, 1080.0));
    let ray = camera
        .viewport_to_world(camera_transform, viewport_size / 2.0)
        .unwrap();

    params.selection.position = None;
    params.selection.normal = None;

    // Entity Raycast: Check for trees or other hittables using a manual cylinder check
    let mut entity_hit = None;
    params.entity_selection.entity = None;

    for (entity, target_transform, _health) in params.health_query.iter() {
        // EXCLUDE PLAYER: Don't hit yourself or block your own view
        if params.player_query.get(entity).is_ok() {
            continue;
        }

        let to_target = target_transform.translation - ray.origin;
        let forward_vec = Vec3::from(ray.direction);
        let t = to_target.dot(forward_vec);

        if t > 0.0 && t < 25.0 {
            let closest_point = ray.origin + forward_vec * t;
            let horizontal_dist = Vec2::new(
                target_transform.translation.x,
                target_transform.translation.z,
            )
            .distance(Vec2::new(closest_point.x, closest_point.z));
            let vertical_diff = closest_point.y - target_transform.translation.y;

            if horizontal_dist < 1.5 && (0.0..15.0).contains(&vertical_diff) {
                entity_hit = Some(entity);
                params.entity_selection.entity = Some(entity);
                break;
            }
        }
    }

    if let Some(hit_entity) = entity_hit {
        let mut is_mining = params.mouse_input.pressed(MouseButton::Left);
        for gamepad in params.gamepads.iter() {
            if gamepad.pressed(GamepadButton::RightTrigger2) {
                is_mining = true;
            }
        }
        if is_mining && let Ok((_ent, _trans, mut health)) = params.health_query.get_mut(hit_entity)
        {
            let efficiency = if *params.weapon == WeaponState::Axe {
                4.0
            } else {
                0.5
            };
            health.hp -= params.time.delta_secs() * 40.0 * efficiency;

            // Visual feedback (sparks on entity)
            let mut rng = rand::rng();
            if rng.random_bool(0.2) {
                let p_pos = ray.origin + ray.direction * 5.0; // Approximation
                let (mesh, mat) = entity_spark_cache
                    .get_or_insert_with(|| {
                        (
                            params.meshes.add(Cuboid::from_size(Vec3::splat(0.1))),
                            params.materials.add(StandardMaterial {
                                base_color: Color::WHITE,
                                ..default()
                            }),
                        )
                    })
                    .clone();
                params.commands.spawn((
                    Mesh3d(mesh),
                    MeshMaterial3d(mat),
                    Transform::from_translation(p_pos),
                    Particle {
                        velocity: Vec3::new(0.0, 2.0, 0.0),
                        lifetime: Timer::from_seconds(0.3, TimerMode::Once),
                    },
                ));
            }

            if health.hp <= 0.0 {
                params
                    .commands
                    .entity(hit_entity)
                    .insert(crate::player::combat::DamageEvent(100.0));
            }
        }
    } else {
        // Machinery / Architectural Raycast: Check if we're looking at a custom block
        let mut closest_dist = 25.0;
        let mut hit_pos = None;
        let mut hit_normal = None;

        for &pos in params.machinery_registry.map.keys() {
            let center = pos.as_vec3() + 0.5;
            // Simple Ray-AABB intersection for a 1x1x1 cube
            let to_center = center - ray.origin;
            let forward_vec = Vec3::from(ray.direction);
            let t = to_center.dot(forward_vec);

            if t > 0.0 && t < closest_dist {
                let closest_point = ray.origin + forward_vec * t;
                let to_point = closest_point - center;
                if closest_point.distance(center) < 0.6 {
                    // Rough hit check for 1x1x1
                    closest_dist = t;
                    hit_pos = Some(pos);
                    // Calculate correct AABB face normal
                    let normal = if to_point.x.abs() > to_point.y.abs()
                        && to_point.x.abs() > to_point.z.abs()
                    {
                        IVec3::new(to_point.x.signum() as i32, 0, 0)
                    } else if to_point.y.abs() > to_point.x.abs()
                        && to_point.y.abs() > to_point.z.abs()
                    {
                        IVec3::new(0, to_point.y.signum() as i32, 0)
                    } else {
                        IVec3::new(0, 0, to_point.z.signum() as i32)
                    };
                    hit_normal = Some(normal);
                }
            }
        }

        if let Some(pos) = hit_pos {
            params.selection.position = Some(pos);
            params.selection.normal = hit_normal;
        } else if let Some(result) = voxel_world.raycast(ray, &|(_, vox)| {
            if let WorldVoxel::Solid(id) = vox {
                crate::voxel::BlockType::from(id) != crate::voxel::BlockType::Water
            } else {
                false
            }
        }) {
            params.selection.position = Some(result.position.as_ivec3());
            params.selection.normal = result.normal.map(|n| n.as_ivec3());
        }
    }

    if let Some(pos) = params.selection.position {
        let mut is_mining = params.mouse_input.pressed(MouseButton::Left);
        for gamepad in params.gamepads.iter() {
            if gamepad.pressed(GamepadButton::RightTrigger2) {
                is_mining = true;
            }
        }
        if is_mining {
            if mech.active {
                // Determine block type (supports both standard solid voxels and custom architectural elements)
                let block_type_opt = if let WorldVoxel::Solid(mat) = voxel_world.get_voxel(pos) {
                    Some(material_to_block(mat))
                } else if params.machinery_registry.map.contains_key(&pos) {
                    params.world_persistence.modified_blocks.get(&pos).copied()
                } else {
                    None
                };

                if let Some(block) = block_type_opt {
                    let (base_efficiency, required_weapon) = match block {
                        BlockType::Wood | BlockType::Leaves => (4.0, WeaponState::Axe),
                        BlockType::Stone
                        | BlockType::IronOre
                        | BlockType::GoldOre
                        | BlockType::Gear
                        | BlockType::Axle => (4.0, WeaponState::Pickaxe),
                        _ => (1.0, WeaponState::NoWeapon),
                    };

                    let efficiency = if *params.weapon == required_weapon {
                        let mut mult = base_efficiency;
                        if required_weapon == WeaponState::Pickaxe {
                            if params.inventory.has_gold_pickaxe {
                                mult *= 4.0;
                            } else if params.inventory.has_iron_pickaxe {
                                mult *= 2.0;
                            }
                        } else if required_weapon == WeaponState::Axe {
                            if params.inventory.has_gold_axe {
                                mult *= 4.0;
                            } else if params.inventory.has_iron_axe {
                                mult *= 2.0;
                            }
                        }
                        mult
                    } else {
                        0.5 // Penalty for wrong tool
                    };

                    params.mining_progress.progress += params.time.delta_secs()
                        * efficiency
                        * (1.0 + mech.mining_level as f32 * 0.2);

                    // Spawn "mining sparks" during progress
                    let mut rng = rand::rng();
                    if rng.random_bool(0.1) {
                        for _ in 0..2 {
                            let p_pos =
                                pos.as_vec3() + Vec3::new(rng.random(), rng.random(), rng.random());
                            let p_vel = Vec3::new(
                                rng.random_range(-1.0..1.0),
                                rng.random_range(1.0..3.0),
                                rng.random_range(-1.0..1.0),
                            );
                            let (mesh, mat) = block_spark_cache
                                .get_or_insert_with(|| {
                                    (
                                        params.meshes.add(Cuboid::from_size(Vec3::splat(0.05))),
                                        params.materials.add(StandardMaterial {
                                            base_color: Color::from(
                                                bevy::color::palettes::css::ORANGE_RED,
                                            ),
                                            emissive: LinearRgba::RED,
                                            ..default()
                                        }),
                                    )
                                })
                                .clone();
                            params.commands.spawn((
                                Mesh3d(mesh),
                                MeshMaterial3d(mat),
                                Transform::from_translation(p_pos),
                                Particle {
                                    velocity: p_vel,
                                    lifetime: Timer::from_seconds(0.5, TimerMode::Once),
                                },
                            ));
                        }
                    }

                    if params.mining_progress.progress >= 1.0 {
                        modify_block(
                            &mut voxel_world,
                            &mut params.machinery_registry,
                            &mut params.inventory,
                            &mut params.world_persistence,
                            pos,
                            BlockType::Air,
                            &mut params.commands,
                            &mut params.meshes,
                            &mut params.materials,
                            Vec3::ZERO,
                            &params.asset_server,
                            &mut params.water_impulses,
                            &mut splinter_cache,
                        );
                        params.mining_progress.progress = 0.0;
                    }
                }
            } else {
                println!("Mining requires Mech Suit activation (M)");
            }
        } else {
            params.mining_progress.progress = 0.0;
        }

        let mut is_placing = params.mouse_input.just_pressed(MouseButton::Right);
        for gamepad in params.gamepads.iter() {
            if gamepad.just_pressed(GamepadButton::LeftTrigger2) {
                is_placing = true;
            }
        }
        if is_placing
            && params.placement.current_block != BlockType::ProceduralWall
            && let Some(normal) = params.selection.normal
        {
            // Permit building on top or on the sides of both standard blocks and custom machinery/slopes
            let can_build = if let WorldVoxel::Solid(_) = voxel_world.get_voxel(pos) {
                true
            } else {
                params.machinery_registry.map.contains_key(&pos)
            };

            if can_build {
                let (t, _) = params.player_query.single().expect("Player must exist");
                let player_pos = t.translation;
                modify_block(
                    &mut voxel_world,
                    &mut params.machinery_registry,
                    &mut params.inventory,
                    &mut params.world_persistence,
                    pos + normal,
                    params.placement.current_block,
                    &mut params.commands,
                    &mut params.meshes,
                    &mut params.materials,
                    player_pos,
                    &params.asset_server,
                    &mut params.water_impulses,
                    &mut splinter_cache,
                );
            }
        }
    } else {
        params.mining_progress.progress = 0.0;
    }
}

fn modify_block(
    voxel_world: &mut VoxelWorld<NoiseGenerator>,
    machinery_registry: &mut MachineryRegistry,
    inventory: &mut Inventory,
    world_persistence: &mut WorldPersistence,
    pos: IVec3,
    block_type: BlockType,
    commands: &mut Commands,
    meshes: &mut Assets<Mesh>,
    materials: &mut Assets<StandardMaterial>,
    player_pos: Vec3,
    asset_server: &AssetServer,
    water_impulses: &mut MessageWriter<crate::world::water::WaterImpulseEvent>,
    splinter_cache: &mut FxHashMap<BlockType, (Handle<Mesh>, Handle<StandardMaterial>)>,
) {
    let old_block = if let WorldVoxel::Solid(mat) = voxel_world.get_voxel(pos) {
        material_to_block(mat)
    } else {
        BlockType::Air
    };

    // Update the voxel world
    if block_type == BlockType::Air {
        voxel_world.set_voxel(pos, WorldVoxel::Air);

        // ADD TO INVENTORY
        if old_block != BlockType::Air {
            *inventory.resources.entry(old_block).or_insert(0) += 1;
            println!(
                "Collected: {:?} (Total: {})",
                old_block, inventory.resources[&old_block]
            );
        }
    } else {
        // Only place a solid voxel for TERRAIN blocks.
        // Machinery (Generator, Motor, Gear, etc.) should be Air visually in the voxel world
        // so their entity mesh is visible.
        let is_machinery = matches!(
            block_type,
            BlockType::Generator
                | BlockType::Motor
                | BlockType::Gear
                | BlockType::Axle
                | BlockType::Boat
                | BlockType::Crafter
                | BlockType::Chest
                | BlockType::Furnace
                | BlockType::Pipe
                | BlockType::Door
                | BlockType::CastleDoor
                | BlockType::SlidingDoor
                | BlockType::Slope
        );

        if is_machinery {
            voxel_world.set_voxel(pos, WorldVoxel::Air);
        } else {
            // Update the voxel world: Slope types and machinery are set to Air in the voxel world
            // so they don't render as blocks, but we keep them in our MachineryRegistry for selection.
            let voxel_to_set = match block_type {
                BlockType::Slope
                | BlockType::SlopeCorner
                | BlockType::SlopeValley
                | BlockType::Door
                | BlockType::CastleDoor => WorldVoxel::Air,
                _ => WorldVoxel::Solid(block_to_material(block_type)),
            };
            voxel_world.set_voxel(pos, voxel_to_set);
        }
    }

    // Update persistence
    world_persistence.modified_blocks.insert(pos, block_type);

    // Handle machinery
    if block_type == BlockType::Air {
        if let Some(entity) = machinery_registry.map.remove(&pos) {
            commands.entity(entity).despawn();
        }
    } else {
        match block_type {
            BlockType::Generator => {
                let entity = commands
                    .spawn((
                        Mesh3d(meshes.add(Cuboid::from_size(Vec3::ONE))),
                        MeshMaterial3d(materials.add(StandardMaterial {
                            base_color: Color::srgb(1.0, 0.8, 0.0), // Gold
                            ..default()
                        })),
                        PowerNode {
                            power_type: PowerType::Kinetic,
                            current_power: 0.0,
                            capacity: 100.0,
                        },
                        MachineLogic::Generator(1.0),
                        Transform::from_translation(pos.as_vec3() + 0.5)
                            .with_scale(Vec3::splat(0.8)),
                        bevy_rapier3d::prelude::Collider::cuboid(0.4, 0.4, 0.4),
                    ))
                    .id();
                machinery_registry.map.insert(pos, entity);
            }
            BlockType::Motor => {
                let entity = commands
                    .spawn((
                        Mesh3d(meshes.add(Cuboid::from_size(Vec3::ONE))),
                        MeshMaterial3d(materials.add(StandardMaterial {
                            base_color: Color::srgb(0.0, 0.8, 1.0), // Cyan
                            ..default()
                        })),
                        PowerNode {
                            power_type: PowerType::Kinetic,
                            current_power: 0.0,
                            capacity: 50.0,
                        },
                        MachineLogic::Motor(0.5),
                        Transform::from_translation(pos.as_vec3() + 0.5)
                            .with_scale(Vec3::splat(0.8)),
                        bevy_rapier3d::prelude::Collider::cuboid(0.4, 0.4, 0.4),
                    ))
                    .id();
                machinery_registry.map.insert(pos, entity);
            }
            BlockType::Gear => {
                let entity = commands
                    .spawn((
                        Mesh3d(meshes.add(Cylinder::new(0.5, 0.2))),
                        MeshMaterial3d(materials.add(StandardMaterial {
                            base_color: Color::srgb(0.7, 0.7, 0.7), // Steel
                            ..default()
                        })),
                        PowerNode {
                            power_type: PowerType::Kinetic,
                            current_power: 0.0,
                            capacity: 10.0,
                        },
                        Transform::from_translation(pos.as_vec3() + 0.5)
                            .with_rotation(Quat::from_rotation_x(1.5)),
                        bevy_rapier3d::prelude::Collider::cylinder(0.1, 0.5),
                    ))
                    .id();
                machinery_registry.map.insert(pos, entity);
            }
            BlockType::Axle => {
                let entity = commands
                    .spawn((
                        Mesh3d(meshes.add(Cylinder::new(0.1, 1.0))),
                        MeshMaterial3d(materials.add(StandardMaterial {
                            base_color: Color::srgb(0.4, 0.4, 0.4),
                            ..default()
                        })),
                        PowerNode {
                            power_type: PowerType::Kinetic,
                            current_power: 0.0,
                            capacity: 10.0,
                        },
                        MachineLogic::Axle,
                        Transform::from_translation(pos.as_vec3() + 0.5),
                        bevy_rapier3d::prelude::Collider::cylinder(0.5, 0.1),
                    ))
                    .id();
                machinery_registry.map.insert(pos, entity);
            }
            BlockType::Boat => {
                let entity = commands
                    .spawn((
                        crate::world::water::Boat,
                        crate::world::water::Buoyant { force: 25.0 },
                        Transform::from_translation(pos.as_vec3() + Vec3::new(0.5, 0.2, 0.5)),
                        Visibility::default(),
                        InheritedVisibility::default(),
                    ))
                    .with_children(|parent| {
                        // Boat Hull
                        parent.spawn((
                            Mesh3d(meshes.add(Cuboid::new(1.2, 0.3, 2.0))),
                            MeshMaterial3d(materials.add(StandardMaterial {
                                base_color: Color::srgb(0.5, 0.3, 0.1), // Brown
                                ..default()
                            })),
                            Transform::from_xyz(0.0, 0.0, 0.0),
                        ));
                        // Sides
                        for side in [-0.5, 0.5] {
                            parent.spawn((
                                Mesh3d(meshes.add(Cuboid::new(0.1, 0.6, 2.0))),
                                MeshMaterial3d(materials.add(StandardMaterial {
                                    base_color: Color::srgb(0.4, 0.2, 0.1),
                                    ..default()
                                })),
                                Transform::from_xyz(side, 0.3, 0.0),
                            ));
                        }
                    })
                    .id();
                machinery_registry.map.insert(pos, entity);
            }
            BlockType::Door => {
                let to_player = (player_pos - pos.as_vec3()).normalize();
                let rotation = if to_player.x.abs() > to_player.z.abs() {
                    if to_player.x > 0.0 {
                        Quat::from_rotation_y(1.57)
                    } else {
                        Quat::from_rotation_y(-1.57)
                    }
                } else {
                    if to_player.z > 0.0 {
                        Quat::IDENTITY
                    } else {
                        Quat::from_rotation_y(std::f32::consts::PI)
                    }
                };

                let entity = commands
                    .spawn((
                        Transform::from_translation(pos.as_vec3() + Vec3::new(0.5, 0.0, 0.5))
                            .with_rotation(rotation),
                        Visibility::default(),
                        InheritedVisibility::default(),
                    ))
                    .with_children(|parent| {
                        // Spawn Hinge Child at local Z = -0.45
                        parent
                            .spawn((
                                Door {
                                    open: false,
                                    hinge_side: -1.0,
                                },
                                Transform::from_xyz(0.0, 0.0, -0.45),
                                Visibility::default(),
                                InheritedVisibility::default(),
                            ))
                            .with_children(|hinge| {
                                // Spawn Door Mesh child at local Z = 0.45 (centered inside parent closed block!)
                                hinge.spawn((
                                    Mesh3d(meshes.add(Cuboid::new(0.1, 2.0, 0.9))),
                                    MeshMaterial3d(materials.add(StandardMaterial {
                                        base_color: Color::srgb(0.5, 0.3, 0.1),
                                        ..default()
                                    })),
                                    Transform::from_xyz(0.0, 1.0, 0.45),
                                    bevy_rapier3d::prelude::Collider::cuboid(0.05, 1.0, 0.45),
                                ));
                            });
                    })
                    .id();
                machinery_registry.map.insert(pos, entity);
            }
            BlockType::CastleDoor => {
                let to_player = (player_pos - pos.as_vec3()).normalize();
                let rotation = if to_player.x.abs() > to_player.z.abs() {
                    if to_player.x > 0.0 {
                        Quat::from_rotation_y(1.57)
                    } else {
                        Quat::from_rotation_y(-1.57)
                    }
                } else {
                    if to_player.z > 0.0 {
                        Quat::IDENTITY
                    } else {
                        Quat::from_rotation_y(std::f32::consts::PI)
                    }
                };

                let entity = commands
                    .spawn((
                        Transform::from_translation(pos.as_vec3() + Vec3::new(0.5, 0.0, 0.5))
                            .with_rotation(rotation),
                        Visibility::default(),
                        InheritedVisibility::default(),
                    ))
                    .with_children(|parent| {
                        // Spawn Hinge Child at local Z = -0.9
                        parent
                            .spawn((
                                Door {
                                    open: false,
                                    hinge_side: -1.0,
                                },
                                Transform::from_xyz(0.0, 0.0, -0.9),
                                Visibility::default(),
                                InheritedVisibility::default(),
                            ))
                            .with_children(|hinge| {
                                // Spawn Door Mesh child at local Z = 0.9 (centered inside parent closed block!)
                                hinge.spawn((
                                    Mesh3d(meshes.add(Cuboid::new(0.2, 3.0, 1.8))),
                                    MeshMaterial3d(materials.add(StandardMaterial {
                                        base_color: Color::srgb(0.3, 0.3, 0.3),
                                        ..default()
                                    })),
                                    Transform::from_xyz(0.0, 1.5, 0.9),
                                    bevy_rapier3d::prelude::Collider::cuboid(0.1, 1.5, 0.9),
                                ));
                            });
                    })
                    .id();
                machinery_registry.map.insert(pos, entity);
            }
            BlockType::Slope | BlockType::SlopeCorner | BlockType::SlopeValley => {
                // Determine orientation based on player position
                let to_player = (player_pos - pos.as_vec3()).normalize();
                let rotation = if to_player.x.abs() > to_player.z.abs() {
                    if to_player.x > 0.0 {
                        Quat::from_rotation_y(1.57)
                    } else {
                        Quat::from_rotation_y(-1.57)
                    }
                } else {
                    if to_player.z > 0.0 {
                        Quat::IDENTITY
                    } else {
                        Quat::from_rotation_y(std::f32::consts::PI)
                    }
                };

                let mesh_handle = match block_type {
                    BlockType::Slope => meshes.add(create_wedge_mesh()),
                    BlockType::SlopeCorner => meshes.add(create_slope_corner_mesh()),
                    BlockType::SlopeValley => meshes.add(create_slope_valley_mesh()),
                    _ => unreachable!(),
                };

                let entity = commands
                    .spawn((
                        Mesh3d(mesh_handle),
                        MeshMaterial3d(materials.add(StandardMaterial {
                            base_color_texture: Some(
                                asset_server.load("textures/roof_shingles.png"),
                            ),
                            cull_mode: None,
                            perceptual_roughness: 0.8,
                            ..default()
                        })),
                        Transform::from_translation(pos.as_vec3() + 0.5).with_rotation(rotation),
                        Slope,
                        bevy_rapier3d::prelude::Collider::cuboid(0.5, 0.5, 0.5),
                    ))
                    .id();
                machinery_registry.map.insert(pos, entity);
            }
            _ => {}
        }
    }

    // Spawn particles if breaking a block
    if block_type == BlockType::Air && old_block != BlockType::Air {
        let mut rng = rand::rng();
        let color = match old_block {
            BlockType::IronOre => Color::from(bevy::color::palettes::css::ORANGE),
            BlockType::GoldOre => Color::from(bevy::color::palettes::css::GOLD),
            BlockType::Stone | BlockType::Limestone => {
                Color::from(bevy::color::palettes::css::GRAY)
            }
            BlockType::Granite => Color::from(bevy::color::palettes::css::SILVER),
            BlockType::Basalt => Color::from(bevy::color::palettes::css::DARK_GRAY),
            BlockType::Slate => Color::from(bevy::color::palettes::css::MIDNIGHT_BLUE),
            BlockType::Wood => Color::from(bevy::color::palettes::css::BROWN),
            BlockType::Leaves | BlockType::Fern | BlockType::Flower => {
                Color::from(bevy::color::palettes::css::GREEN)
            }
            BlockType::Gear | BlockType::Axle => Color::from(bevy::color::palettes::css::SILVER),
            _ => Color::WHITE,
        };

        let (splinter_mesh, splinter_mat) = splinter_cache
            .entry(old_block)
            .or_insert_with(|| {
                (
                    meshes.add(Cuboid::from_size(Vec3::ONE)),
                    materials.add(StandardMaterial {
                        base_color: color,
                        ..default()
                    }),
                )
            })
            .clone();

        for _ in 0..8 {
            let p_pos = pos.as_vec3()
                + Vec3::new(
                    rng.random_range(0.1..0.9),
                    rng.random_range(0.1..0.9),
                    rng.random_range(0.1..0.9),
                );
            let p_vel = Vec3::new(
                rng.random_range(-2.0..2.0),
                rng.random_range(2.0..5.0),
                rng.random_range(-2.0..2.0),
            );

            commands.spawn((
                Mesh3d(splinter_mesh.clone()),
                MeshMaterial3d(splinter_mat.clone()),
                Transform::from_translation(p_pos).with_scale(Vec3::splat(0.2)),
                Particle {
                    velocity: p_vel,
                    lifetime: Timer::from_seconds(
                        1.0 + rng.random_range(0.0..1.0),
                        TimerMode::Once,
                    ),
                },
            ));
        }
    }

    // Fire water impulse event for splashing when blocks are modified (placed or broken)
    // especially for blocks near the water surface level (Y around 30.0)
    if (pos.y as f32) < 35.0 && (pos.y as f32) > 20.0 {
        let force = if block_type == BlockType::Air {
            -15.0
        } else {
            15.0
        };
        water_impulses.write(crate::world::water::WaterImpulseEvent {
            position: pos.as_vec3() + 0.5,
            force,
            radius: 1.5,
        });
    }
}

fn material_to_block(mat: u8) -> BlockType {
    BlockType::from(mat)
}

fn block_to_material(block: BlockType) -> u8 {
    block as u8
}

fn update_particles(
    mut commands: Commands,
    time: Res<Time>,
    mut query: Query<(Entity, &mut Transform, &mut Particle)>,
) {
    let dt = time.delta_secs();
    for (entity, mut transform, mut particle) in query.iter_mut() {
        particle.lifetime.tick(time.delta());
        if particle.lifetime.just_finished() {
            commands.entity(entity).despawn();
        } else {
            particle.velocity.y -= 9.8 * dt; // Gravity
            transform.translation += particle.velocity * dt;
            transform.rotate_local_x(dt * 5.0);
            transform.rotate_local_y(dt * 3.0);
        }
    }
}

fn toggle_doors(
    mouse: Res<ButtonInput<MouseButton>>,
    camera_query: Query<(&Camera, &GlobalTransform), With<MainCamera>>,
    mut door_query: Query<(Entity, &GlobalTransform, &mut Transform, &mut Door)>,
    gamepads: Query<&Gamepad>,
) {
    let mut toggling = mouse.just_pressed(MouseButton::Left);
    for gamepad in gamepads.iter() {
        if gamepad.just_pressed(GamepadButton::RightTrigger2) {
            toggling = true;
        }
    }
    if !toggling {
        return;
    }

    let (camera, camera_transform) = camera_query.single().expect("Camera must exist");
    let viewport_size = camera
        .logical_viewport_size()
        .unwrap_or(Vec2::new(1920.0, 1080.0));
    let ray = camera
        .viewport_to_world(camera_transform, viewport_size / 2.0)
        .unwrap();

    let mut best_door: Option<(Entity, Mut<Transform>, Mut<Door>, f32)> = None;

    for (entity, global_transform, transform, door) in door_query.iter_mut() {
        let hinge_pos = global_transform.translation();

        // Calculate the center of the door panel to allow clicking anywhere on the door wood
        let local_transform = global_transform.compute_transform();
        let right = local_transform.rotation * Vec3::X;
        let panel_offset = door.hinge_side * -0.6; // Handles standard/castle door panel centers
        let panel_center = hinge_pos + right * panel_offset;

        let dist_to_hinge = ray.origin.distance(hinge_pos);
        let dist_to_panel = ray.origin.distance(panel_center);

        if dist_to_hinge < 6.0 || dist_to_panel < 6.0 {
            let to_hinge = hinge_pos - ray.origin;
            let to_panel = panel_center - ray.origin;

            let dot_hinge = to_hinge.normalize().dot(Vec3::from(ray.direction));
            let dot_panel = to_panel.normalize().dot(Vec3::from(ray.direction));

            let best_dot = dot_hinge.max(dot_panel);

            // Forgiving click alignment threshold
            if best_dot > 0.88 {
                if let Some((_, _, _, current_best_dot)) = best_door {
                    if best_dot > current_best_dot {
                        best_door = Some((entity, transform, door, best_dot));
                    }
                } else {
                    best_door = Some((entity, transform, door, best_dot));
                }
            }
        }
    }

    // Toggle only the single closest door targeted by player crosshairs
    if let Some((_entity, mut transform, mut door, _)) = best_door {
        door.open = !door.open;
        if door.open {
            transform.rotation = Quat::from_rotation_y(1.5 * door.hinge_side);
        } else {
            transform.rotation = Quat::IDENTITY;
        }
        println!(
            "Toggled Door (Open: {}, Side: {})",
            door.open, door.hinge_side
        );
    }
}

fn draw_selection_highlight(
    mut gizmos: Gizmos,
    selection: Res<SelectedBlock>,
    entity_selection: Res<SelectedEntity>,
    entity_query: Query<&Transform, With<crate::player::combat::Hittable>>,
    mining_progress: Res<MiningProgress>,
    time: Res<Time>,
) {
    // 1. Entity Highlight (Trees, etc.)
    if let Some(entity) = entity_selection.entity
        && let Ok(transform) = entity_query.get(entity)
    {
        let center = transform.translation + Vec3::Y * 5.0; // Most entities are tall (trees)
        gizmos.primitive_3d(
            &Cuboid::new(3.0, 10.0, 3.0),
            center,
            Color::srgba(0.0, 1.0, 1.0, 0.3),
        );
    }

    if let Some(pos) = selection.position {
        let mut center = pos.as_vec3() + Vec3::splat(0.5);
        let progress = mining_progress.progress;

        // 1. Block Shake Effect
        if progress > 0.0 {
            let shake = progress * 0.1 * (time.elapsed_secs() * 50.0).sin();
            center += Vec3::new(shake, shake, shake);
        }

        // 2. Selection Box
        let _color = if progress > 0.0 {
            Color::srgba(1.0, 1.0 - progress, 0.0, 0.6)
        } else {
            Color::srgba(1.0, 1.0, 1.0, 0.4)
        };

        gizmos.primitive_3d(
            &Cuboid::from_size(Vec3::splat(1.01)),
            center,
            Color::from(bevy::color::palettes::css::WHITE.with_alpha(0.5)),
        );

        // 3. Mining Cracks
        if progress > 0.1 {
            let crack_count = (progress * 12.0) as i32;
            for i in 0..crack_count {
                let offset = (i as f32 * 0.2).sin() * 0.4;
                gizmos.line(
                    center + Vec3::new(-0.5, 0.5 - offset, 0.51),
                    center + Vec3::new(0.5, -0.5 + offset, 0.51),
                    Color::BLACK,
                );
                gizmos.line(
                    center + Vec3::new(0.51, 0.5 - offset, -0.5),
                    center + Vec3::new(0.51, -0.5 + offset, 0.5),
                    Color::BLACK,
                );
            }
        }
    }
}

fn create_wedge_mesh() -> Mesh {
    use bevy::asset::RenderAssetUsages;
    use bevy::mesh::Indices;
    use bevy::render::render_resource::PrimitiveTopology;

    let vertices = [
        // Bottom (y=-0.5)
        [-0.5, -0.5, -0.5],
        [0.5, -0.5, -0.5],
        [0.5, -0.5, 0.5],
        [-0.5, -0.5, 0.5],
        // Back (z=-0.5)
        [-0.5, -0.5, -0.5],
        [0.5, -0.5, -0.5],
        [0.5, 0.5, -0.5],
        [-0.5, 0.5, -0.5],
        // Right (x=0.5)
        [0.5, -0.5, -0.5],
        [0.5, -0.5, 0.5],
        [0.5, 0.5, -0.5],
        // Left (x=-0.5)
        [-0.5, -0.5, -0.5],
        [-0.5, -0.5, 0.5],
        [-0.5, 0.5, -0.5],
        // Slope
        [-0.5, 0.5, -0.5],
        [0.5, 0.5, -0.5],
        [0.5, -0.5, 0.5],
        [-0.5, -0.5, 0.5],
    ];

    let normals = [
        [0.0, -1.0, 0.0],
        [0.0, -1.0, 0.0],
        [0.0, -1.0, 0.0],
        [0.0, -1.0, 0.0],
        [0.0, 0.0, -1.0],
        [0.0, 0.0, -1.0],
        [0.0, 0.0, -1.0],
        [0.0, 0.0, -1.0],
        [1.0, 0.0, 0.0],
        [1.0, 0.0, 0.0],
        [1.0, 0.0, 0.0],
        [-1.0, 0.0, 0.0],
        [-1.0, 0.0, 0.0],
        [-1.0, 0.0, 0.0],
        [0.0, 0.707, 0.707],
        [0.0, 0.707, 0.707],
        [0.0, 0.707, 0.707],
        [0.0, 0.707, 0.707],
    ];

    let uvs = [
        [0.0, 0.0],
        [1.0, 0.0],
        [1.0, 1.0],
        [0.0, 1.0],
        [0.0, 0.0],
        [1.0, 0.0],
        [1.0, 1.0],
        [0.0, 1.0],
        [0.0, 0.0],
        [1.0, 0.0],
        [1.0, 1.0],
        [0.0, 0.0],
        [1.0, 0.0],
        [1.0, 1.0],
        [0.0, 0.0],
        [1.0, 0.0],
        [1.0, 1.0],
        [0.0, 1.0],
    ];

    let indices = Indices::U32(vec![
        0, 2, 1, 0, 3, 2, // Bottom
        4, 5, 6, 4, 6, 7, // Back
        8, 9, 10, // Right
        11, 13, 12, // Left
        14, 15, 16, 14, 16, 17, // Slope
    ]);

    let mut mesh = Mesh::new(
        PrimitiveTopology::TriangleList,
        RenderAssetUsages::default(),
    );
    mesh.insert_attribute(Mesh::ATTRIBUTE_POSITION, vertices.to_vec());
    mesh.insert_attribute(Mesh::ATTRIBUTE_NORMAL, normals.to_vec());
    mesh.insert_attribute(Mesh::ATTRIBUTE_UV_0, uvs.to_vec());
    mesh.insert_indices(indices);
    mesh
}

fn create_slope_corner_mesh() -> Mesh {
    use bevy::asset::RenderAssetUsages;
    use bevy::mesh::Indices;
    use bevy::render::render_resource::PrimitiveTopology;

    let vertices = [
        // Bottom (Square)
        [-0.5, -0.5, -0.5],
        [0.5, -0.5, -0.5],
        [0.5, -0.5, 0.5],
        [-0.5, -0.5, 0.5],
        // Back (Vertical)
        [-0.5, -0.5, -0.5],
        [0.5, -0.5, -0.5],
        [0.5, 0.5, -0.5],
        [-0.5, 0.5, -0.5],
        // Left (Vertical)
        [-0.5, -0.5, -0.5],
        [-0.5, -0.5, 0.5],
        [-0.5, 0.5, -0.5],
        // Slope Hip (Diagonal faces)
        [-0.5, 0.5, -0.5],
        [0.5, 0.5, -0.5],
        [0.5, -0.5, 0.5],
        [-0.5, -0.5, 0.5],
    ];

    let normals = [
        [0.0, -1.0, 0.0],
        [0.0, -1.0, 0.0],
        [0.0, -1.0, 0.0],
        [0.0, -1.0, 0.0], // Bottom
        [0.0, 0.0, -1.0],
        [0.0, 0.0, -1.0],
        [0.0, 0.0, -1.0],
        [0.0, 0.0, -1.0], // Back
        [-1.0, 0.0, 0.0],
        [-1.0, 0.0, 0.0],
        [-1.0, 0.0, 0.0], // Left
        [0.5, 0.5, 0.5],
        [0.5, 0.5, 0.5],
        [0.5, 0.5, 0.5],
        [0.5, 0.5, 0.5], // Diagonal (normalized later)
    ];

    let uvs = [
        [0.0, 0.0],
        [1.0, 0.0],
        [1.0, 1.0],
        [0.0, 1.0],
        [0.0, 0.0],
        [1.0, 0.0],
        [1.0, 1.0],
        [0.0, 1.0],
        [0.0, 0.0],
        [1.0, 0.0],
        [1.0, 1.0],
        [0.0, 0.0],
        [1.0, 0.0],
        [1.0, 1.0],
        [0.0, 1.0],
    ];

    let indices = Indices::U32(vec![
        0, 2, 1, 0, 3, 2, // Bottom
        4, 5, 6, 4, 6, 7, // Back
        8, 9, 10, // Left
        11, 12, 13, 11, 13, 14, // Slope
    ]);

    let mut mesh = Mesh::new(
        PrimitiveTopology::TriangleList,
        RenderAssetUsages::default(),
    );
    mesh.insert_attribute(Mesh::ATTRIBUTE_POSITION, vertices.to_vec());
    mesh.insert_attribute(Mesh::ATTRIBUTE_NORMAL, normals.to_vec());
    mesh.insert_attribute(Mesh::ATTRIBUTE_UV_0, uvs.to_vec());
    mesh.insert_indices(indices);
    mesh
}

fn create_slope_valley_mesh() -> Mesh {
    use bevy::asset::RenderAssetUsages;
    use bevy::mesh::Indices;
    use bevy::render::render_resource::PrimitiveTopology;

    // A valley is basically a block with a corner cut out diagonally
    let vertices = [
        // Bottom (Square)
        [-0.5, -0.5, -0.5],
        [0.5, -0.5, -0.5],
        [0.5, -0.5, 0.5],
        [-0.5, -0.5, 0.5],
        // Two high sides (trapezoids)
        [-0.5, 0.5, -0.5],
        [0.5, 0.5, -0.5],
        [0.5, -0.5, -0.5], // Side 1
        [-0.5, 0.5, -0.5],
        [-0.5, 0.5, 0.5],
        [-0.5, -0.5, 0.5], // Side 2
        // Valley (Diagonal)
        [-0.5, 0.5, -0.5],
        [0.5, -0.5, -0.5],
        [0.5, -0.5, 0.5],
        [-0.5, -0.5, 0.5],
        [-0.5, 0.5, 0.5],
    ];

    let normals = [
        [0.0, -1.0, 0.0],
        [0.0, -1.0, 0.0],
        [0.0, -1.0, 0.0],
        [0.0, -1.0, 0.0],
        [0.0, 0.0, -1.0],
        [0.0, 0.0, -1.0],
        [0.0, 0.0, -1.0],
        [-1.0, 0.0, 0.0],
        [-1.0, 0.0, 0.0],
        [-1.0, 0.0, 0.0],
        [0.5, 0.5, 0.5],
        [0.5, 0.5, 0.5],
        [0.5, 0.5, 0.5],
        [0.5, 0.5, 0.5],
        [0.5, 0.5, 0.5],
    ];

    let uvs = [
        [0.0, 0.0],
        [1.0, 0.0],
        [1.0, 1.0],
        [0.0, 1.0],
        [0.0, 0.0],
        [1.0, 0.0],
        [1.0, 1.0],
        [0.0, 0.0],
        [1.0, 0.0],
        [1.0, 1.0],
        [0.0, 0.0],
        [1.0, 0.0],
        [1.0, 1.0],
        [0.5, 0.5],
        [0.0, 1.0],
    ];

    let indices = Indices::U32(vec![
        0, 2, 1, 0, 3, 2, // Bottom
        4, 5, 6, // Side 1
        7, 8, 9, // Side 2
        10, 11, 12, 10, 12, 13, 10, 13, 14, // Valley
    ]);

    let mut mesh = Mesh::new(
        PrimitiveTopology::TriangleList,
        RenderAssetUsages::default(),
    );
    mesh.insert_attribute(Mesh::ATTRIBUTE_POSITION, vertices.to_vec());
    mesh.insert_attribute(Mesh::ATTRIBUTE_NORMAL, normals.to_vec());
    mesh.insert_attribute(Mesh::ATTRIBUTE_UV_0, uvs.to_vec());
    mesh.insert_indices(indices);
    mesh
}
