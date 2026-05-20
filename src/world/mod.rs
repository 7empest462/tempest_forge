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
            settlement::spawn_settlements,
            tree_generator::chunk_vegetation_system,
            tree_generator::start_tree_generation,
            tree_generator::complete_tree_generation,
            tree_generator::despawn_trees_near_buildings,
            make_water_transparent,
            sync_chunk_colliders,
        ).chain())
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
            println!("VOXEL WORLD MATERIAL SET TO OPAQUE");
        }
    }
}

fn sync_chunk_colliders(
    mut commands: Commands,
    chunk_query: Query<(Entity, &Mesh3d), (Changed<Mesh3d>, With<Chunk<noise_generator::NoiseGenerator>>)>,
    meshes: Res<Assets<Mesh>>,
) {
    for (entity, mesh_handle) in chunk_query.iter() {
        if let Some(mesh) = meshes.get(mesh_handle) {
            // Using TriMesh(Default::default()) for flags or TriMesh
            if let Some(collider) = bevy_rapier3d::prelude::Collider::from_bevy_mesh(mesh, &bevy_rapier3d::prelude::ComputedColliderShape::TriMesh(Default::default())) {
                commands.entity(entity).insert((
                    collider,
                    bevy_rapier3d::prelude::RigidBody::Fixed,
                    // Give chunks a high friction so players don't slide down shallow slopes
                    bevy_rapier3d::prelude::Friction::coefficient(1.0),
                ));
            } else {
                // If the mesh is empty (e.g. all air chunk), just ensure no collider exists
                commands.entity(entity).remove::<(bevy_rapier3d::prelude::Collider, bevy_rapier3d::prelude::RigidBody)>();
            }
        }
    }
}

fn setup(
    mut _commands: Commands,
) {
    // Setup logic for the new world system
}
