use bevy::asset::Asset;
use bevy::pbr::{
    ExtendedMaterial, MaterialExtension, MaterialExtensionKey, MaterialExtensionPipeline,
};
use bevy::prelude::*;
use bevy::render::render_resource::{
    AsBindGroup, RenderPipelineDescriptor, SpecializedMeshPipelineError,
};
use bevy::shader::ShaderRef;
use bevy_voxel_world::prelude::*;
use bevy_voxel_world::rendering::VoxelWorldMaterialHandle;

pub mod dimension;
pub mod env;
pub mod manager;
pub mod noise_generator;
pub mod settlement;
pub mod tree_generator;
pub mod water;
pub mod water_gpu;
pub mod weather;

#[derive(Asset, AsBindGroup, Debug, Clone, Default, TypePath)]
pub struct MyVoxelMaterial {
    #[texture(100, dimension = "2d_array")]
    #[sampler(101)]
    pub voxels_texture: Handle<Image>,

    #[texture(102, dimension = "2d_array")]
    #[sampler(103)]
    pub voxels_normal_texture: Handle<Image>,

    #[texture(104, dimension = "2d_array")]
    #[sampler(105)]
    pub voxels_orm_texture: Handle<Image>,
}

impl Material for MyVoxelMaterial {
    fn enable_prepass() -> bool {
        false
    }
}

impl MaterialExtension for MyVoxelMaterial {
    fn fragment_shader() -> ShaderRef {
        "shaders/voxel_texture_pbr.wgsl".into()
    }

    fn vertex_shader() -> ShaderRef {
        "shaders/voxel_texture_pbr.wgsl".into()
    }

    fn specialize(
        _pipeline: &MaterialExtensionPipeline,
        descriptor: &mut RenderPipelineDescriptor,
        layout: &bevy::mesh::MeshVertexBufferLayoutRef,
        _key: MaterialExtensionKey<Self>,
    ) -> Result<(), SpecializedMeshPipelineError> {
        let vertex_layout = layout
            .0
            .get_layout(&bevy_voxel_world::rendering::vertex_layout())?;
        descriptor.vertex.buffers = vec![vertex_layout];
        Ok(())
    }
}
#[derive(Resource)]
pub struct VoxelTextureLoading {
    pub color_handle: Handle<Image>,
    pub normal_handle: Handle<Image>,
    pub orm_handle: Handle<Image>,
    pub alien_handle: Handle<Image>,
    pub is_loaded: bool,
}

pub struct WorldPlugin;

impl Plugin for WorldPlugin {
    fn build(&self, app: &mut App) {
        let asset_server = app.world().resource::<AssetServer>();
        let color_handle = asset_server.load("default_texture.png");
        let normal_handle = asset_server.load("default_texture_normal.png");
        let orm_handle = asset_server.load("default_texture_orm.png");
        let alien_handle = asset_server.load("alien_texture.png");

        app.insert_resource(VoxelTextureLoading {
            color_handle,
            normal_handle,
            orm_handle,
            alien_handle,
            is_loaded: false,
        });

        let material = ExtendedMaterial {
            base: StandardMaterial {
                reflectance: 0.05,
                metallic: 0.05,
                perceptual_roughness: 0.95,
                alpha_mode: AlphaMode::Mask(0.5), // Enables transparent glass
                ..default()
            },
            extension: MyVoxelMaterial {
                voxels_texture: Handle::default(),
                voxels_normal_texture: Handle::default(),
                voxels_orm_texture: Handle::default(),
            },
        };
        app.add_plugins(MaterialPlugin::<
            ExtendedMaterial<StandardMaterial, MyVoxelMaterial>,
        >::default())
            .add_plugins(
                VoxelWorldPlugin::with_config(noise_generator::NoiseGenerator::new(
                    fastrand::u32(..),
                    8,
                ))
                .with_material(material),
            )
            .add_plugins(settlement::SettlementPlugin)
            .add_plugins(env::EnvironmentPlugin)
            .add_plugins(water::WaterPlugin)
            .add_plugins(weather::WeatherPlugin)
            .init_resource::<tree_generator::TreeGenerator>()
            .add_systems(
                Update,
                (
                    // Tree pipeline must be ordered (each step feeds the next)
                    tree_generator::chunk_vegetation_system,
                    tree_generator::start_tree_generation,
                    tree_generator::complete_tree_generation,
                    tree_generator::despawn_trees_near_buildings,
                )
                    .chain(),
            )
            // These systems are independent — let Bevy run them in parallel
            .add_systems(Update, settlement::spawn_settlements)
            .add_systems(Update, prepare_voxel_texture)
            .add_systems(Update, sync_chunk_colliders)
            .add_systems(Startup, setup);
    }
}

fn generate_combined_mipmapped_array_texture(
    source_image: &Image,
    alien_image: Option<&Image>,
    is_normal: bool,
    is_orm: bool,
) -> Image {
    let tile_width = source_image.width();
    let tile_height = source_image.height() / 14;

    // Calculate max mip levels for a square tile: log2(128) + 1 = 8
    let mip_levels = (tile_width.max(tile_height) as f32).log2().floor() as u32 + 1;

    let dynamic_image = source_image.clone().try_into_dynamic().unwrap();
    let mut texture_data = Vec::new();

    // Process the 14 layers from default_texture.png
    for layer in 0..14 {
        let tile = dynamic_image.crop_imm(0, layer * tile_height, tile_width, tile_height);

        for mip in 0..mip_levels {
            let mip_w = (tile_width >> mip).max(1);
            let mip_h = (tile_height >> mip).max(1);

            let resized = if mip == 0 {
                tile.clone()
            } else {
                tile.resize_exact(mip_w, mip_h, image::imageops::FilterType::Nearest)
            };

            texture_data.extend_from_slice(&resized.to_rgba8().into_raw());
        }
    }

    // Process the 5 alien layers
    if let Some(alien_img) = alien_image {
        let alien_dynamic = alien_img.clone().try_into_dynamic().unwrap();
        let alien_w = alien_img.width();
        let alien_h = alien_img.height() / 5;

        for layer in 0..5 {
            let tile = alien_dynamic.crop_imm(0, layer * alien_h, alien_w, alien_h);
            let tile_resized = tile.resize_exact(
                tile_width,
                tile_height,
                image::imageops::FilterType::Nearest,
            );

            for mip in 0..mip_levels {
                let mip_w = (tile_width >> mip).max(1);
                let mip_h = (tile_height >> mip).max(1);

                let resized = if mip == 0 {
                    tile_resized.clone()
                } else {
                    tile_resized.resize_exact(mip_w, mip_h, image::imageops::FilterType::Nearest)
                };

                texture_data.extend_from_slice(&resized.to_rgba8().into_raw());
            }
        }
    } else {
        // Generate 5 procedural 128x128 layers
        let fill_color = if is_normal {
            [127, 127, 255, 255] // Flat normal map (neutral blue)
        } else if is_orm {
            [255, 243, 0, 255] // Default ORM: Occlusion=255, Roughness=0.95 (243/255), Metallic=0
        } else {
            [0, 0, 0, 0]
        };

        let procedural_tile = image::DynamicImage::ImageRgba8(image::ImageBuffer::from_pixel(
            tile_width,
            tile_height,
            image::Rgba(fill_color),
        ));

        for _ in 0..5 {
            for mip in 0..mip_levels {
                let mip_w = (tile_width >> mip).max(1);
                let mip_h = (tile_height >> mip).max(1);

                let resized = if mip == 0 {
                    procedural_tile.clone()
                } else {
                    procedural_tile.resize_exact(mip_w, mip_h, image::imageops::FilterType::Nearest)
                };

                texture_data.extend_from_slice(&resized.to_rgba8().into_raw());
            }
        }
    }

    let mut image = Image::new_uninit(
        bevy::render::render_resource::Extent3d {
            width: tile_width,
            height: tile_height,
            depth_or_array_layers: 19, // 14 default + 5 alien layers
        },
        bevy::render::render_resource::TextureDimension::D2,
        source_image.texture_descriptor.format,
        bevy::asset::RenderAssetUsages::RENDER_WORLD | bevy::asset::RenderAssetUsages::MAIN_WORLD,
    );
    image.texture_descriptor.mip_level_count = mip_levels;
    image.data = Some(texture_data);
    image
}

fn prepare_voxel_texture(
    asset_server: Res<AssetServer>,
    mut loading: ResMut<VoxelTextureLoading>,
    mut images: ResMut<Assets<Image>>,
    mut materials: ResMut<Assets<ExtendedMaterial<StandardMaterial, MyVoxelMaterial>>>,
    material_handle: Option<
        Res<VoxelWorldMaterialHandle<ExtendedMaterial<StandardMaterial, MyVoxelMaterial>>>,
    >,
) {
    if loading.is_loaded {
        return;
    }
    let Some(material_handle) = material_handle else {
        return;
    };

    let color_loaded = matches!(
        asset_server.get_load_state(loading.color_handle.clone().id()),
        Some(bevy::asset::LoadState::Loaded)
    );
    let normal_loaded = matches!(
        asset_server.get_load_state(loading.normal_handle.clone().id()),
        Some(bevy::asset::LoadState::Loaded)
    );
    let orm_loaded = matches!(
        asset_server.get_load_state(loading.orm_handle.clone().id()),
        Some(bevy::asset::LoadState::Loaded)
    );
    let alien_loaded = matches!(
        asset_server.get_load_state(loading.alien_handle.clone().id()),
        Some(bevy::asset::LoadState::Loaded)
    );

    if color_loaded && normal_loaded && orm_loaded && alien_loaded {
        // Safely get required images; if not yet available in the asset map, bail and try next frame
        let (Some(color_src), Some(normal_src), Some(orm_src), Some(alien_src)) = (
            images.get(&loading.color_handle),
            images.get(&loading.normal_handle),
            images.get(&loading.orm_handle),
            images.get(&loading.alien_handle),
        ) else {
            return;
        };

        // Alien sheet is loaded
        let alien_src_opt = Some(alien_src);

        let color_mipmapped =
            generate_combined_mipmapped_array_texture(color_src, alien_src_opt, false, false);
        let normal_mipmapped =
            generate_combined_mipmapped_array_texture(normal_src, None, true, false);
        let orm_mipmapped = generate_combined_mipmapped_array_texture(orm_src, None, false, true);

        // 2. Add the new mipmapped images as assets and get handles
        let color_handle = images.add(color_mipmapped);
        let normal_handle = images.add(normal_mipmapped);
        let orm_handle = images.add(orm_mipmapped);

        // 3. Configure repeating, mipmapped, and anisotropic samplers
        for handle in [&color_handle, &normal_handle, &orm_handle] {
            let image = images.get_mut(handle).unwrap();
            let descriptor = image.sampler.get_or_init_descriptor();
            descriptor.address_mode_u = bevy::image::ImageAddressMode::Repeat;
            descriptor.address_mode_v = bevy::image::ImageAddressMode::Repeat;
            descriptor.address_mode_w = bevy::image::ImageAddressMode::Repeat;
            descriptor.min_filter = bevy::image::ImageFilterMode::Linear;
            descriptor.mag_filter = bevy::image::ImageFilterMode::Linear;
            descriptor.mipmap_filter = bevy::image::ImageFilterMode::Linear;
            descriptor.anisotropy_clamp = 16;
        }

        // 4. Assign the mipmapped handles to our material
        if let Some(mat) = materials.get_mut(&material_handle.handle) {
            mat.extension.voxels_texture = color_handle;
            mat.extension.voxels_normal_texture = normal_handle;
            mat.extension.voxels_orm_texture = orm_handle;
        }

        loading.is_loaded = true;
        info!(
            "Custom PBR mipmapped voxel texture arrays (Base, Normal, ORM) prepared and assigned (19 layers)!"
        );
    }
}

fn sync_chunk_colliders(
    mut commands: Commands,
    player_query: Query<&Transform, With<crate::player::camera::Player>>,
    chunk_query: Query<
        (
            Entity,
            &Mesh3d,
            &Transform,
            Option<&bevy_rapier3d::prelude::Collider>,
        ),
        With<Chunk<noise_generator::NoiseGenerator>>,
    >,
    changed_chunk_query: Query<
        Entity,
        (
            Changed<Mesh3d>,
            With<Chunk<noise_generator::NoiseGenerator>>,
        ),
    >,
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

    let player_pos = player_query
        .iter()
        .next()
        .map(|t| t.translation)
        .unwrap_or(Vec3::ZERO);
    let changed_entities: rustc_hash::FxHashSet<Entity> = changed_chunk_query.iter().collect();

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
                if let Some(mesh) = meshes.get(mesh_handle)
                    && let Some(collider) = bevy_rapier3d::prelude::Collider::from_bevy_mesh(
                        mesh,
                        &bevy_rapier3d::prelude::ComputedColliderShape::TriMesh(Default::default()),
                    )
                {
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

fn setup(mut _commands: Commands) {
    // Setup logic for the new world system
}
