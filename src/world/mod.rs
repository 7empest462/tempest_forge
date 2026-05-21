use bevy::prelude::*;
use bevy_voxel_world::prelude::*;

pub mod env;
pub mod noise_generator;
pub mod manager;
pub mod settlement;
pub mod water;
pub mod tree_generator;

pub struct WorldPlugin;

impl Plugin for WorldPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins(VoxelWorldPlugin::with_config(noise_generator::NoiseGenerator::new()))
        .add_plugins(settlement::SettlementPlugin)
        .add_plugins(env::EnvironmentPlugin)
        .add_plugins(water::WaterPlugin)
        .add_plugins(bevy_procedural_tree::TreeProceduralGenerationPlugin)
        .init_resource::<tree_generator::TreeGenerator>()
        .add_systems(Update, (
            // Tree pipeline must be ordered (each step feeds the next)
            tree_generator::chunk_vegetation_system,
            tree_generator::start_tree_generation,
            tree_generator::complete_tree_generation,
            tree_generator::despawn_trees_near_buildings,
        ).chain())
        // These systems are independent — let Bevy run them in parallel
        .add_systems(Update, settlement::spawn_settlements)
        .add_systems(Update, make_water_transparent)
        .add_systems(Update, sync_chunk_colliders)
        .add_systems(Startup, setup);
    }
}

fn make_water_transparent(
    mut materials: ResMut<Assets<StandardMaterial>>,
    chunk_query: Query<&MeshMaterial3d<StandardMaterial>, With<Chunk<noise_generator::NoiseGenerator>>>,
    mut done: Local<bool>,
) {
    if *done { return; }
    for mat_handle in chunk_query.iter() {
        if let Some(mat) = materials.get_mut(mat_handle) {
            mat.alpha_mode = AlphaMode::Opaque; // Back to solid to fix X-ray
            *done = true;
            info!("Voxel world material set to Opaque");
        }
    }
}

fn sync_chunk_colliders(
    mut commands: Commands,
    player_query: Query<&Transform, With<crate::player::camera::Player>>,
    chunk_query: Query<(Entity, &Mesh3d, &Transform, Option<&bevy_rapier3d::prelude::Collider>), With<Chunk<noise_generator::NoiseGenerator>>>,
    changed_chunk_query: Query<Entity, (Changed<Mesh3d>, With<Chunk<noise_generator::NoiseGenerator>>)>,
    meshes: Res<Assets<Mesh>>,
    mut timer: Local<f32>,
    time: Res<Time>,
) {
    // Run periodic range-based cleanup/activation every 0.15 seconds to minimize CPU footprint
    *timer += time.delta_secs();
    let run_periodic = if *timer >= 0.15 {
        *timer = 0.0;
        true
    } else {
        false
    };

    let player_pos = player_query.iter().next().map(|t| t.translation).unwrap_or(Vec3::ZERO);
    let changed_entities: std::collections::HashSet<Entity> = changed_chunk_query.iter().collect();

    // 64 meters (4 chunks radius) is the physical loading zone. 
    // 72 meters adds a hysteresis buffer to prevent mesh remaking if the player treads on a border.
    const LOAD_RADIUS: f32 = 64.0;
    const UNLOAD_RADIUS: f32 = 72.0;

    for (entity, mesh_handle, transform, maybe_collider) in chunk_query.iter() {
        let chunk_pos = transform.translation;
        let distance = player_pos.distance(chunk_pos);
        let mesh_changed = changed_entities.contains(&entity);

        if maybe_collider.is_some() {
            if distance > UNLOAD_RADIUS && run_periodic {
                // Out of range: drop the physical collider to save CPU memory and solver time
                commands.entity(entity).remove::<(
                    bevy_rapier3d::prelude::Collider,
                    bevy_rapier3d::prelude::RigidBody,
                    bevy_rapier3d::prelude::Friction,
                )>();
            } else if mesh_changed {
                // In range and mesh changed: rebuild collider immediately to match modified terrain
                if let Some(mesh) = meshes.get(mesh_handle) {
                    if let Some(collider) = bevy_rapier3d::prelude::Collider::from_bevy_mesh(
                        mesh,
                        &bevy_rapier3d::prelude::ComputedColliderShape::TriMesh(Default::default()),
                    ) {
                        commands.entity(entity).insert(collider);
                    } else {
                        commands.entity(entity).remove::<(
                            bevy_rapier3d::prelude::Collider,
                            bevy_rapier3d::prelude::RigidBody,
                            bevy_rapier3d::prelude::Friction,
                        )>();
                    }
                }
            }
        } else {
            // No physical collider active currently
            if distance <= LOAD_RADIUS && (mesh_changed || run_periodic) {
                // Enters range: generate and insert standard friction terrain colliders
                if let Some(mesh) = meshes.get(mesh_handle) {
                    if let Some(collider) = bevy_rapier3d::prelude::Collider::from_bevy_mesh(
                        mesh,
                        &bevy_rapier3d::prelude::ComputedColliderShape::TriMesh(Default::default()),
                    ) {
                        commands.entity(entity).insert((
                            collider,
                            bevy_rapier3d::prelude::RigidBody::Fixed,
                            bevy_rapier3d::prelude::Friction::coefficient(1.0),
                        ));
                    }
                }
            }
        }
    }
}

fn setup(
    mut _commands: Commands,
) {
    // Setup logic for the new world system
}
