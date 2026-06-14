//! Procedural wall generation module for creating dynamic brick walls.

pub mod arch;
pub mod brick;
pub mod curve;
pub mod wall_constructor;

pub use arch::{
    AUTO_ARCH_ENDPOINT_DIST, ArchBrick, ArchOpening, MAX_ARCH_SPAN, MIN_ARCH_SPAN, WallEndpoint,
    find_arch_openings, generate_arch, voussoir_spawn_delay,
};
pub use brick::Brick;
pub use curve::Curve;
pub use wall_constructor::WallConstructor;

use crate::voxel::BlockType;
use crate::world::noise_generator::NoiseGenerator;
use bevy::prelude::*;
use bevy_voxel_world::prelude::VoxelWorld;
use rand::RngExt;

/// Plugin for procedural brick wall construction and destruction.
pub struct ProceduralWallsPlugin;

impl Plugin for ProceduralWallsPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<ProceduralWallBuilder>()
            .init_resource::<ArchRegistry>()
            .init_resource::<ProceduralWallPreviewCache>()
            .init_resource::<ProceduralWallAssets>()
            .add_systems(
                Update,
                (
                    update_wall_builder,
                    draw_wall_preview,
                    mine_procedural_bricks,
                    animate_brick_spawns,
                    carve_gateways,
                    detect_and_spawn_arches,
                ),
            );
    }
}

/// Active builder state for placing procedural wall curves
#[derive(Resource)]
pub struct ProceduralWallBuilder {
    /// Placed control points
    pub points: Vec<Vec3>,
    /// Selected height for the wall (adjustable dynamically!)
    pub height: f32,
}

/// Cache resource for real-time holographic brick/curve preview.
/// Saves CPU cycles by avoiding expensive voxel searches and curve resampling
/// when the control points and height are static.
#[derive(Resource, Default)]
pub struct ProceduralWallPreviewCache {
    pub points: Vec<Vec3>,
    pub height: f32,
    pub cached_bricks: Vec<Brick>,
    pub cached_voussoirs: Vec<ArchBrick>,
}

impl Default for ProceduralWallBuilder {
    fn default() -> Self {
        Self {
            points: Vec::new(),
            height: 2.4, // Default wall height
        }
    }
}

/// Animates a brick scaling and dropping down upon spawning
#[derive(Component)]
pub struct BrickSpawnAnimation {
    pub target_translation: Vec3,
    pub target_scale: Vec3,
    pub delay: f32,
    pub elapsed: f32,
    pub duration: f32,
}

/// Marker component for each individual generated wall brick entity
#[derive(Component)]
pub struct ProceduralBrick;

/// Marker component for voussoir (arch) bricks — sub-type of ProceduralBrick.
/// Both components are present on arch bricks, so mining/health work automatically.
#[derive(Component)]
pub struct ProceduralArchBrick {
    /// ID linking all voussoirs in the same arch together.
    pub arch_id: u64,
}

/// Tracks all currently-live auto-detected arch openings so we don't re-spawn
/// them every frame.
#[derive(Resource, Default)]
pub struct ArchRegistry {
    /// Each entry is (left_foot_xz, right_foot_xz, root_entity).
    pub arches: Vec<(bevy::math::Vec2, bevy::math::Vec2, Entity)>,
}

/// Cached mesh and material assets for procedural walls and their particles
#[derive(Resource)]
pub struct ProceduralWallAssets {
    pub unit_cube: Handle<Mesh>,
    pub spark_mesh: Handle<Mesh>,
    pub spark_material: Handle<StandardMaterial>,
    pub dust_mesh: Handle<Mesh>,
    pub dust_material: Handle<StandardMaterial>,
    pub splinter_mesh: Handle<Mesh>,
    pub splinter_material: Handle<StandardMaterial>,
}

impl FromWorld for ProceduralWallAssets {
    fn from_world(world: &mut World) -> Self {
        let mut meshes = world.resource_mut::<Assets<Mesh>>();
        let unit_cube = meshes.add(Cuboid::from_size(Vec3::ONE));
        let spark_mesh = meshes.add(Cuboid::from_size(Vec3::splat(0.08)));
        let dust_mesh = meshes.add(Cuboid::from_size(Vec3::splat(0.12)));
        let splinter_mesh = meshes.add(Cuboid::from_size(Vec3::splat(0.1)));

        let mut materials = world.resource_mut::<Assets<StandardMaterial>>();
        let spark_material = materials.add(StandardMaterial {
            base_color: Color::from(bevy::color::palettes::css::GOLD),
            emissive: LinearRgba::from(Color::srgb(1.0, 0.8, 0.0)),
            ..default()
        });
        let dust_material = materials.add(StandardMaterial {
            base_color: Color::srgba(0.8, 0.75, 0.7, 0.45),
            alpha_mode: AlphaMode::Blend,
            ..default()
        });
        let splinter_material = materials.add(StandardMaterial {
            base_color: Color::srgba(0.6, 0.4, 0.35, 1.0),
            ..default()
        });

        Self {
            unit_cube,
            spark_mesh,
            spark_material,
            dust_mesh,
            dust_material,
            splinter_mesh,
            splinter_material,
        }
    }
}

/// Handles curve point placement, undo, cancellation, and wall finalization.
fn update_wall_builder(
    mut builder: ResMut<ProceduralWallBuilder>,
    placement: Res<crate::player::interaction::PlacementState>,
    selection: Res<crate::player::interaction::SelectedBlock>,
    mouse_input: Res<ButtonInput<MouseButton>>,
    keys: Res<ButtonInput<KeyCode>>,
    ui_state: Res<crate::ui::UiState>,
    mut commands: Commands,
    mut materials: ResMut<Assets<StandardMaterial>>,
    asset_server: Res<AssetServer>,
    voxel_world: VoxelWorld<NoiseGenerator>,
    gamepads: Query<&Gamepad>,
    procedural_wall_assets: Res<ProceduralWallAssets>,
) {
    // Only active when the Procedural Wall building option is selected and UI is closed
    if placement.current_block != BlockType::ProceduralWall
        || ui_state.show_inventory
        || ui_state.show_pause_menu
    {
        return;
    }

    // Get current hover position above targeted voxel face
    let hover_point = if let (Some(pos), Some(normal)) = (selection.position, selection.normal) {
        Some((pos + normal).as_vec3() + Vec3::new(0.5, 0.0, 0.5))
    } else {
        None
    };

    let mut gamepad_place = false;
    let mut gamepad_undo = false;
    let mut gamepad_cancel = false;
    let mut gamepad_height_up = false;
    let mut gamepad_height_down = false;
    let mut gamepad_build = false;

    for gamepad in gamepads.iter() {
        if gamepad.just_pressed(GamepadButton::LeftTrigger2) {
            gamepad_place = true;
        }
        if gamepad.just_pressed(GamepadButton::LeftTrigger) {
            gamepad_undo = true;
        }
        if gamepad.just_pressed(GamepadButton::East) {
            gamepad_cancel = true;
        }
        if gamepad.pressed(GamepadButton::DPadRight) {
            gamepad_height_up = true;
        }
        if gamepad.pressed(GamepadButton::DPadLeft) {
            gamepad_height_down = true;
        }
        if gamepad.just_pressed(GamepadButton::RightTrigger2) {
            gamepad_build = true;
        }
    }

    // 1. Place curve control point (Right-Click or LT)
    if (mouse_input.just_pressed(MouseButton::Right) || gamepad_place)
        && let Some(pt) = hover_point
    {
        builder.points.push(pt);

        // Visual feedback spark
        let mut rng = rand::rng();
        for _ in 0..4 {
            commands.spawn((
                Mesh3d(procedural_wall_assets.spark_mesh.clone()),
                MeshMaterial3d(procedural_wall_assets.spark_material.clone()),
                Transform::from_translation(pt),
                crate::player::interaction::Particle {
                    velocity: Vec3::new(
                        rng.random_range(-1.5..1.5),
                        rng.random_range(1.5..3.5),
                        rng.random_range(-1.5..1.5),
                    ),
                    lifetime: Timer::from_seconds(0.4, TimerMode::Once),
                },
            ));
        }
    }

    // 2. Undo last point (Backspace, Delete, or LB)
    if keys.just_pressed(KeyCode::Backspace) || keys.just_pressed(KeyCode::Delete) || gamepad_undo {
        builder.points.pop();
    }

    // 3. Cancel build (Escape or B Button)
    if keys.just_pressed(KeyCode::Escape) || gamepad_cancel {
        builder.points.clear();
    }

    // 4. Dynamic height adjustments (Up/Down arrow keys or D-Pad Right/Left)
    if keys.pressed(KeyCode::ArrowUp) || gamepad_height_up {
        builder.height = (builder.height + 0.04).min(6.0); // Maximum 6.0m high
    }
    if keys.pressed(KeyCode::ArrowDown) || gamepad_height_down {
        builder.height = (builder.height - 0.04).max(0.4); // Minimum 0.4m high (at least 1 row)
    }

    // 5. Confirm and build wall (Enter/Return or RT)
    if (keys.just_pressed(KeyCode::Enter)
        || keys.just_pressed(KeyCode::NumpadEnter)
        || gamepad_build)
        && builder.points.len() >= 2
    {
        // =========================================================================
        // ACTIVE TEXTURE CONFIGURATION (Choose your cozy wall style here!):
        // - "textures/solid_stone.png"     -> Real Single Solid Stone Granite Blocks (Active Default)
        // - "textures/solid_brick.png"     -> Real Single Solid Baked Red Clay Bricks
        // - "textures/solid_limestone.png" -> Real Single Solid Cream Limestone Blocks
        // =========================================================================
        let active_texture = "textures/solid_stone.png";

        let raw_curve = Curve::from(builder.points.clone()).smooth(2);
        // Ensure even segment lengths for beautiful bricks
        let resampled_curve = raw_curve.resample(0.8);
        let bricks = WallConstructor::from_curve(&resampled_curve, builder.height, |pos| {
            crate::world::manager::find_ground_height(pos, &voxel_world).unwrap_or(pos.y)
        });

        let mut rng = fastrand::Rng::new();

        for (idx, brick) in bricks.iter().enumerate() {
            // Determine base color tint depending on selected block style
            let (base_r, base_g, base_b) = match active_texture {
                "textures/solid_stone.png" => (0.62, 0.62, 0.64), // Raw chiseled granite gray stone
                "textures/solid_brick.png" => (0.76, 0.44, 0.30), // Earthy terracotta baked clay
                "textures/solid_limestone.png" => (0.85, 0.82, 0.74), // Warm medieval cream limestone
                _ => (0.62, 0.48, 0.42),                              // Default brown
            };

            // Organic shade-by-shade block variation for natural, hand-laid masonry look
            let r_off = (rng.f32() - 0.5) * 0.12;
            let g_off = (rng.f32() - 0.5) * 0.10;
            let b_off = (rng.f32() - 0.5) * 0.10;
            let brick_color = Color::srgba(
                (base_r + r_off).clamp(0.0, 1.0),
                (base_g + g_off).clamp(0.0, 1.0),
                (base_b + b_off).clamp(0.0, 1.0),
                1.0,
            );

            // Determine mortar color depending on selected block style
            let mortar_color = match active_texture {
                "textures/solid_stone.png" => Color::srgb(0.78, 0.78, 0.76), // Cement/concrete gray mortar
                "textures/solid_brick.png" => Color::srgb(0.88, 0.86, 0.82), // Warm creamy off-white mortar
                "textures/solid_limestone.png" => Color::srgb(0.68, 0.66, 0.62), // Sandstone dark gray mortar
                _ => Color::srgb(0.80, 0.80, 0.80),
            };

            // Spawn parent container (invisible pivot, handles physics and mining damage)
            let brick_pos = brick.transform.translation;
            let stagger_delay = brick.pivot_uv.y * 0.35 + brick.pivot_uv.x * 0.15; // Stagger from bottom-left to top-right
            commands
                .spawn((
                    ProceduralBrick,
                    crate::player::combat::Hittable,
                    crate::player::combat::Health::new(35.0),
                    brick.transform.with_scale(Vec3::splat(0.01)), // Start near-zero to animate in (avoids parry3d BVH panic with zero-scale colliders)
                    // Collider is added after the drop-and-bounce spawn animation completes to avoid heavy BVH refitting/stuttering every frame!
                    BrickSpawnAnimation {
                        target_translation: brick.transform.translation,
                        target_scale: brick.transform.scale,
                        delay: stagger_delay,
                        elapsed: 0.0,
                        duration: 0.42,
                    },
                    Visibility::default(),
                    InheritedVisibility::default(),
                ))
                .with_children(|parent| {
                    // Dynamic adjacency checking: detect if there are neighboring bricks on any side.
                    // If a neighbor is missing (e.g. at wall boundaries, around doors, or skipped on the top row),
                    // the corresponding face is exposed to air and must be capped with full stone and recessed mortar.
                    let has_left_neighbor = bricks.iter().enumerate().any(|(i, other)| {
                        i != idx
                            && (other.pivot_uv.y - brick.pivot_uv.y).abs() < 0.01
                            && (brick.pivot_uv.x
                                - brick.bounds_uv.x / 2.0
                                - (other.pivot_uv.x + other.bounds_uv.x / 2.0))
                                .abs()
                                < 0.02
                    });
                    let has_right_neighbor = bricks.iter().enumerate().any(|(i, other)| {
                        i != idx
                            && (other.pivot_uv.y - brick.pivot_uv.y).abs() < 0.01
                            && (other.pivot_uv.x
                                - other.bounds_uv.x / 2.0
                                - (brick.pivot_uv.x + brick.bounds_uv.x / 2.0))
                                .abs()
                                < 0.02
                    });
                    let has_top_neighbor = bricks.iter().enumerate().any(|(i, other)| {
                        i != idx
                            && other.pivot_uv.y > brick.pivot_uv.y
                            && (other.pivot_uv.y
                                - other.bounds_uv.y / 2.0
                                - (brick.pivot_uv.y + brick.bounds_uv.y / 2.0))
                                .abs()
                                < 0.02
                            && (other.pivot_uv.x - brick.pivot_uv.x).abs()
                                < (brick.bounds_uv.x + other.bounds_uv.x) / 2.0 - 0.01
                    });
                    let has_bottom_neighbor = bricks.iter().enumerate().any(|(i, other)| {
                        i != idx
                            && other.pivot_uv.y < brick.pivot_uv.y
                            && (brick.pivot_uv.y
                                - brick.bounds_uv.y / 2.0
                                - (other.pivot_uv.y + other.bounds_uv.y / 2.0))
                                .abs()
                                < 0.02
                            && (other.pivot_uv.x - brick.pivot_uv.x).abs()
                                < (brick.bounds_uv.x + other.bounds_uv.x) / 2.0 - 0.01
                    });

                    let is_left = !has_left_neighbor;
                    let is_right = !has_right_neighbor;
                    let is_top = !has_top_neighbor;
                    let is_bottom = !has_bottom_neighbor;

                    // Calculate Stone visual shrinkage: do not shrink edges exposed to the outer borders or empty space!
                    let left_shrink = if is_left { 0.0 } else { 0.02 };
                    let right_shrink = if is_right { 0.0 } else { 0.02 };
                    let bottom_shrink = if is_bottom { 0.0 } else { 0.02 };
                    let top_shrink = if is_top { 0.0 } else { 0.02 };

                    let rel_x = ((brick.transform.scale.x - (left_shrink + right_shrink))
                        / brick.transform.scale.x)
                        .max(0.1);
                    let rel_y = ((brick.transform.scale.y - (bottom_shrink + top_shrink))
                        / brick.transform.scale.y)
                        .max(0.1);

                    // Offsets must be in parent-local space (divided by parent scale) so stone actually reaches the boundary edge!
                    let trans_x = (left_shrink - right_shrink) / (2.0 * brick.transform.scale.x);
                    let trans_y = (bottom_shrink - top_shrink) / (2.0 * brick.transform.scale.y);

                    // Child 1: The textured, organically colored stone/brick block (shifted flush with borders)
                    parent.spawn((
                        Mesh3d(procedural_wall_assets.unit_cube.clone()),
                        MeshMaterial3d(materials.add(StandardMaterial {
                            base_color: brick_color,
                            base_color_texture: Some(asset_server.load(active_texture)),
                            perceptual_roughness: 0.88,
                            metallic: 0.02,
                            ..default()
                        })),
                        Transform {
                            translation: Vec3::new(trans_x, trans_y, 0.0), // Flush offset, centered depth
                            scale: Vec3::new(rel_x, rel_y, 1.05), // Slightly thicker depth to cover both faces
                            ..default()
                        },
                    ));

                    // Calculate Mortar joint shrinkage: pull back from outer boundary edges to prevent sticking out!
                    let mortar_left_inset = if is_left { 0.04 } else { 0.0 };
                    let mortar_right_inset = if is_right { 0.04 } else { 0.0 };
                    let mortar_bottom_inset = if is_bottom { 0.04 } else { 0.0 };
                    let mortar_top_inset = if is_top { 0.04 } else { 0.0 };

                    let mortar_rel_x =
                        1.02 - (mortar_left_inset + mortar_right_inset) / brick.transform.scale.x;
                    let mortar_rel_y =
                        1.02 - (mortar_top_inset + mortar_bottom_inset) / brick.transform.scale.y;

                    // Mortar offsets also in parent-local space
                    let mortar_trans_x =
                        (mortar_left_inset - mortar_right_inset) / (2.0 * brick.transform.scale.x);
                    let mortar_trans_y =
                        (mortar_bottom_inset - mortar_top_inset) / (2.0 * brick.transform.scale.y);

                    // Child 2: The solid colored recessed mortar backing (pulled back from outer edges)
                    parent.spawn((
                        Mesh3d(procedural_wall_assets.unit_cube.clone()),
                        MeshMaterial3d(materials.add(StandardMaterial {
                            base_color: mortar_color,
                            perceptual_roughness: 0.95, // Mortar is very rough
                            metallic: 0.0,
                            ..default()
                        })),
                        Transform {
                            translation: Vec3::new(mortar_trans_x, mortar_trans_y, 0.0), // Centered mortar fill
                            scale: Vec3::new(mortar_rel_x.max(0.1), mortar_rel_y.max(0.1), 0.80),
                            ..default()
                        },
                    ));
                });

            // Spawn satisfying "construction dust/sparks" along the wall
            if rng.f32() < 0.25 {
                commands.spawn((
                    Mesh3d(procedural_wall_assets.dust_mesh.clone()),
                    MeshMaterial3d(procedural_wall_assets.dust_material.clone()),
                    Transform::from_translation(brick_pos),
                    crate::player::interaction::Particle {
                        velocity: Vec3::new(rng.f32() - 0.5, rng.f32() * 1.5, rng.f32() - 0.5),
                        lifetime: Timer::from_seconds(0.7, TimerMode::Once),
                    },
                ));
            }
        }

        builder.points.clear();
    }
}

// ---------------------------------------------------------------------------
// Arch voussoir spawn helper (shared by carve_gateways and detect_and_spawn_arches)
// ---------------------------------------------------------------------------

fn spawn_arch_voussoirs(
    opening: &ArchOpening,
    arch_id: u64,
    active_texture: &'static str,
    parent: Option<Entity>,
    commands: &mut Commands,
    materials: &mut ResMut<Assets<StandardMaterial>>,
    asset_server: &Res<AssetServer>,
    procedural_wall_assets: &Res<ProceduralWallAssets>,
) {
    let voussoirs = generate_arch(opening);
    if voussoirs.is_empty() {
        return;
    }

    let mut rng = fastrand::Rng::new();

    // Slightly darker than regular wall bricks to visually differentiate the arch
    let (base_r, base_g, base_b) = match active_texture {
        "textures/solid_stone.png" => (0.54, 0.54, 0.56),
        "textures/solid_brick.png" => (0.67, 0.39, 0.26),
        "textures/solid_limestone.png" => (0.75, 0.72, 0.65),
        _ => (0.54, 0.42, 0.37),
    };
    let mortar_color = match active_texture {
        "textures/solid_stone.png" => Color::srgb(0.78, 0.78, 0.76),
        "textures/solid_brick.png" => Color::srgb(0.88, 0.86, 0.82),
        "textures/solid_limestone.png" => Color::srgb(0.68, 0.66, 0.62),
        _ => Color::srgb(0.80, 0.80, 0.80),
    };

    for voussoir in &voussoirs {
        let r_off = (rng.f32() - 0.5) * 0.10;
        let g_off = (rng.f32() - 0.5) * 0.08;
        let b_off = (rng.f32() - 0.5) * 0.08;
        let brick_color = Color::srgba(
            (base_r + r_off).clamp(0.0, 1.0),
            (base_g + g_off).clamp(0.0, 1.0),
            (base_b + b_off).clamp(0.0, 1.0),
            1.0,
        );

        let delay = voussoir_spawn_delay(voussoir.arc_t);
        let target_translation = voussoir.transform.translation;
        let target_scale = voussoir.transform.scale;

        let child = commands
            .spawn((
                ProceduralBrick,
                ProceduralArchBrick { arch_id },
                crate::player::combat::Hittable,
                crate::player::combat::Health::new(45.0), // arch bricks are slightly tougher
                voussoir.transform.with_scale(Vec3::splat(0.01)),
                // Collider is added after the spawn animation completes to avoid heavy BVH refitting/stuttering every frame!
                BrickSpawnAnimation {
                    target_translation,
                    target_scale,
                    delay,
                    elapsed: 0.0,
                    duration: 0.42,
                },
                Visibility::default(),
                InheritedVisibility::default(),
            ))
            .with_children(|parent| {
                // Stone face
                parent.spawn((
                    Mesh3d(procedural_wall_assets.unit_cube.clone()),
                    MeshMaterial3d(materials.add(StandardMaterial {
                        base_color: brick_color,
                        base_color_texture: Some(asset_server.load(active_texture)),
                        perceptual_roughness: 0.90,
                        metallic: 0.01,
                        ..default()
                    })),
                    Transform {
                        translation: Vec3::ZERO,
                        scale: Vec3::new(1.02, 1.02, 1.05),
                        ..default()
                    },
                ));
                // Mortar backing
                parent.spawn((
                    Mesh3d(procedural_wall_assets.unit_cube.clone()),
                    MeshMaterial3d(materials.add(StandardMaterial {
                        base_color: mortar_color,
                        perceptual_roughness: 0.95,
                        metallic: 0.0,
                        ..default()
                    })),
                    Transform {
                        translation: Vec3::ZERO,
                        scale: Vec3::new(0.96, 0.96, 0.80),
                        ..default()
                    },
                ));
            })
            .id();

        if let Some(p) = parent {
            commands.entity(p).add_child(child);
        }
    }
}

/// Renders a real-time holographic brick/curve blueprint projection in the game world
fn draw_wall_preview(
    mut gizmos: Gizmos,
    builder: Res<ProceduralWallBuilder>,
    placement: Res<crate::player::interaction::PlacementState>,
    selection: Res<crate::player::interaction::SelectedBlock>,
    voxel_world: VoxelWorld<NoiseGenerator>,
    mut cache: ResMut<ProceduralWallPreviewCache>,
) {
    if placement.current_block != BlockType::ProceduralWall || builder.points.is_empty() {
        return;
    }

    // Connect placed control points with vivid gold indicators
    for (i, &pt) in builder.points.iter().enumerate() {
        gizmos.sphere(pt, 0.15, Color::srgb(1.0, 0.82, 0.0));

        if i > 0 {
            gizmos.line(builder.points[i - 1], pt, Color::srgb(1.0, 0.82, 0.0));
        }
    }

    // Connect the last point to the player's potential next point (guide line)
    let hover_point = if let (Some(pos), Some(normal)) = (selection.position, selection.normal) {
        Some((pos + normal).as_vec3() + Vec3::new(0.5, 0.0, 0.5))
    } else {
        None
    };

    if let (Some(&last_pt), Some(next_pt)) = (builder.points.last(), hover_point) {
        gizmos.line(last_pt, next_pt, Color::srgba(1.0, 0.82, 0.0, 0.45));
    }

    // Invalidate/rebuild cache if builder points or height changed
    let cache_valid =
        cache.points == builder.points && (cache.height - builder.height).abs() < 0.001;
    if !cache_valid {
        cache.points = builder.points.clone();
        cache.height = builder.height;
        cache.cached_bricks.clear();
        cache.cached_voussoirs.clear();

        if builder.points.len() >= 2 {
            let raw_curve = Curve::from(builder.points.clone()).smooth(2);
            let resampled_curve = raw_curve.resample(0.8);
            cache.cached_bricks =
                WallConstructor::from_curve(&resampled_curve, builder.height, |pos| {
                    crate::world::manager::find_ground_height(pos, &voxel_world).unwrap_or(pos.y)
                });

            // Draw holographic arch preview arcs over any detected gap between the
            // preview wall endpoints and nearby existing bricks / the wall itself.
            if let (Some(&first_pt), Some(&last_pt)) = (
                resampled_curve.points.first(),
                resampled_curve.points.last(),
            ) {
                let span_xz = Vec2::new(last_pt.x - first_pt.x, last_pt.z - first_pt.z);
                let span = span_xz.length();
                if (MIN_ARCH_SPAN..=MAX_ARCH_SPAN).contains(&span) {
                    let left_y = crate::world::manager::find_ground_height(first_pt, &voxel_world)
                        .unwrap_or(first_pt.y)
                        + builder.height;
                    let right_y = crate::world::manager::find_ground_height(last_pt, &voxel_world)
                        .unwrap_or(last_pt.y)
                        + builder.height;
                    let opening = ArchOpening {
                        left_foot: first_pt.with_y(left_y),
                        right_foot: last_pt.with_y(right_y),
                    };
                    cache.cached_voussoirs = generate_arch(&opening);
                }
            }
        }
    }

    // Render holographic translucent brick layout projection from cache
    for brick in &cache.cached_bricks {
        gizmos.primitive_3d(
            &Cuboid::new(
                (brick.transform.scale.x - 0.04).max(0.1),
                (brick.transform.scale.y - 0.04).max(0.1),
                (brick.transform.scale.z - 0.04).max(0.1),
            ),
            Isometry3d::new(brick.transform.translation, brick.transform.rotation),
            Color::srgba(0.9, 0.65, 0.1, 0.42),
        );
    }

    // Render holographic arch preview voussoirs from cache
    for v in &cache.cached_voussoirs {
        gizmos.primitive_3d(
            &Cuboid::new(
                (v.transform.scale.x - 0.02).max(0.05),
                (v.transform.scale.y - 0.02).max(0.05),
                (v.transform.scale.z - 0.02).max(0.05),
            ),
            Isometry3d::new(v.transform.translation, v.transform.rotation),
            Color::srgba(0.4, 0.8, 1.0, 0.35), // cyan-ish arch ghost
        );
    }
}

/// Enables the Pickaxe to damage and mine individual generated wall bricks.
fn mine_procedural_bricks(
    mut commands: Commands,
    mouse_input: Res<ButtonInput<MouseButton>>,
    weapon: Res<crate::player::combat::WeaponState>,
    camera_query: Query<(&Camera, &GlobalTransform), With<Camera3d>>,
    mut brick_query: Query<
        (
            Entity,
            &GlobalTransform,
            &mut crate::player::combat::Health,
            &Transform,
        ),
        With<ProceduralBrick>,
    >,
    mut inventory: ResMut<crate::player::interaction::Inventory>,
    time: Res<Time>,
    ui_state: Res<crate::ui::UiState>,
    procedural_wall_assets: Res<ProceduralWallAssets>,
) {
    if ui_state.show_inventory || ui_state.show_pause_menu {
        return;
    }
    if !mouse_input.pressed(MouseButton::Left)
        || *weapon != crate::player::combat::WeaponState::Pickaxe
    {
        return;
    }

    let Ok((camera, camera_transform)) = camera_query.single() else {
        return;
    };
    let viewport_size = camera
        .logical_viewport_size()
        .unwrap_or(Vec2::new(1920.0, 1080.0));
    let ray = camera
        .viewport_to_world(camera_transform, viewport_size / 2.0)
        .unwrap();

    // Raycast check: Find the closest brick within 6.0 meters
    let mut closest_brick: Option<(Entity, Mut<crate::player::combat::Health>, Vec3, Vec3)> = None;
    let mut closest_t = 6.0;

    for (entity, global_transform, health, transform) in brick_query.iter_mut() {
        let pos = global_transform.translation();
        let to_brick = pos - ray.origin;
        let forward_vec = Vec3::from(ray.direction);
        let t = to_brick.dot(forward_vec);

        if t > 0.0 && t < closest_t {
            let closest_point = ray.origin + forward_vec * t;
            let dist = closest_point.distance(pos);
            let bound_radius = transform.scale.max_element() * 0.72;

            if dist < bound_radius {
                closest_t = t;
                closest_brick = Some((entity, health, pos, transform.scale));
            }
        }
    }

    if let Some((entity, mut health, pos, scale)) = closest_brick {
        // Mine efficiency based on pickaxe tier
        let efficiency = if inventory.has_gold_pickaxe {
            4.0
        } else if inventory.has_iron_pickaxe {
            2.0
        } else {
            1.0
        };

        health.hp -= time.delta_secs() * 32.0 * efficiency;

        // Spark particles
        let mut rng = rand::rng();
        if rng.random_bool(0.18) {
            let p_pos = pos
                + Vec3::new(
                    rng.random_range(-0.1..0.1),
                    rng.random_range(-0.1..0.1),
                    rng.random_range(-0.1..0.1),
                );
            commands.spawn((
                Mesh3d(procedural_wall_assets.spark_mesh.clone()),
                MeshMaterial3d(procedural_wall_assets.spark_material.clone()),
                Transform::from_translation(p_pos).with_scale(Vec3::splat(0.5)), // 0.08 * 0.5 = 0.04
                crate::player::interaction::Particle {
                    velocity: Vec3::new(
                        rng.random_range(-1.2..1.2),
                        rng.random_range(1.5..3.5),
                        rng.random_range(-1.2..1.2),
                    ),
                    lifetime: Timer::from_seconds(0.35, TimerMode::Once),
                },
            ));
        }

        if health.hp <= 0.0 {
            commands.entity(entity).despawn();

            // Add 1 standard block type brick back to resources inventory!
            *inventory.resources.entry(BlockType::Brick).or_insert(0) += 1;
            println!(
                "Collected: Brick (Total: {})",
                inventory.resources[&BlockType::Brick]
            );

            // Splinter particles
            for _ in 0..5 {
                let p_pos = pos
                    + Vec3::new(
                        rng.random_range(-scale.x / 2.0..scale.x / 2.0),
                        rng.random_range(-scale.y / 2.0..scale.y / 2.0),
                        rng.random_range(-scale.z / 2.0..scale.z / 2.0),
                    );
                commands.spawn((
                    Mesh3d(procedural_wall_assets.splinter_mesh.clone()),
                    MeshMaterial3d(procedural_wall_assets.splinter_material.clone()),
                    Transform::from_translation(p_pos),
                    crate::player::interaction::Particle {
                        velocity: Vec3::new(
                            rng.random_range(-2.0..2.0),
                            rng.random_range(1.5..4.0),
                            rng.random_range(-2.0..2.0),
                        ),
                        lifetime: Timer::from_seconds(
                            0.8 + rng.random_range(0.0..0.4),
                            TimerMode::Once,
                        ),
                    },
                ));
            }
        }
    }
}

/// Smoothly animates bricks dropping from above and scaling up with spring bounce.
fn animate_brick_spawns(
    time: Res<Time>,
    mut query: Query<(Entity, &mut Transform, &mut BrickSpawnAnimation)>,
    mut commands: Commands,
    procedural_wall_assets: Res<ProceduralWallAssets>,
) {
    let dt = time.delta_secs();
    for (entity, mut transform, mut anim) in query.iter_mut() {
        if anim.delay > 0.0 {
            anim.delay -= dt;
            continue;
        }

        anim.elapsed += dt;
        let progress = (anim.elapsed / anim.duration).clamp(0.0, 1.0);

        // Beautiful elastic spring overshoot/bounce landing formula!
        let bounce = 1.0 - (1.0 - progress).powi(3) * (1.0 - progress * 2.8);

        // Animate translation dropping down from 2.0 meters above
        let height_offset = (1.0 - progress).powi(2) * 2.0;
        transform.translation = anim.target_translation + Vec3::Y * height_offset;

        // Animate scale using the spring bounce
        transform.scale = anim.target_scale * bounce.max(0.0);

        if progress >= 1.0 {
            transform.translation = anim.target_translation;
            transform.scale = anim.target_scale;
            commands.entity(entity).remove::<BrickSpawnAnimation>();

            // Insert collider only after the scaling/translation animation finishes to ensure static BVH updates
            commands
                .entity(entity)
                .insert(bevy_rapier3d::prelude::Collider::cuboid(
                    anim.target_scale.x / 2.0,
                    anim.target_scale.y / 2.0,
                    anim.target_scale.z / 2.0,
                ));

            // Satisfying dust/landing particles!
            let mut rng = rand::rng();
            for _ in 0..2 {
                commands.spawn((
                    Mesh3d(procedural_wall_assets.dust_mesh.clone()),
                    MeshMaterial3d(procedural_wall_assets.dust_material.clone()),
                    Transform::from_translation(anim.target_translation),
                    crate::player::interaction::Particle {
                        velocity: Vec3::new(
                            rng.random_range(-1.0..1.0),
                            rng.random_range(0.2..1.2),
                            rng.random_range(-1.0..1.0),
                        ),
                        lifetime: Timer::from_seconds(0.5, TimerMode::Once),
                    },
                ));
            }
        }
    }
}

/// Dynamically carves castle doors/gateways into existing procedural brick walls.
fn carve_gateways(
    mut commands: Commands,
    keys: Res<ButtonInput<KeyCode>>,
    ui_state: Res<crate::ui::UiState>,
    camera_query: Query<(&Camera, &GlobalTransform), With<Camera3d>>,
    brick_query: Query<(Entity, &GlobalTransform, &Transform), With<ProceduralBrick>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    asset_server: Res<AssetServer>,
    procedural_wall_assets: Res<ProceduralWallAssets>,
) {
    if ui_state.show_inventory || ui_state.show_pause_menu {
        return;
    }

    // Press 'G' key while looking at a brick to carve a gateway
    if !keys.just_pressed(KeyCode::KeyG) {
        return;
    }

    let Ok((camera, camera_transform)) = camera_query.single() else {
        return;
    };
    let viewport_size = camera
        .logical_viewport_size()
        .unwrap_or(Vec2::new(1920.0, 1080.0));
    let ray = camera
        .viewport_to_world(camera_transform, viewport_size / 2.0)
        .unwrap();

    // Raycast: find targeted brick within 6.0 meters
    let mut targeted_brick = None;
    let mut closest_t = 6.0;

    for (entity, global_transform, transform) in brick_query.iter() {
        let pos = global_transform.translation();
        let to_brick = pos - ray.origin;
        let forward_vec = Vec3::from(ray.direction);
        let t = to_brick.dot(forward_vec);

        if t > 0.0 && t < closest_t {
            let closest_point = ray.origin + forward_vec * t;
            let dist = closest_point.distance(pos);
            let bound_radius = transform.scale.max_element() * 0.72;

            if dist < bound_radius {
                closest_t = t;
                targeted_brick = Some((entity, pos, transform.rotation));
            }
        }
    }

    if let Some((_entity, carve_pos, carve_rot)) = targeted_brick {
        let active_texture = "textures/solid_stone.png";
        // Calculate the actual ground and top boundaries of the brick column
        let mut lowest_y = carve_pos.y;
        let mut highest_y = carve_pos.y;
        let mut lowest_scale_y = 0.4;
        let mut highest_scale_y = 0.4;

        for (_, global_transform, transform) in brick_query.iter() {
            let pos = global_transform.translation();
            let horizontal_dist =
                Vec2::new(pos.x, pos.z).distance(Vec2::new(carve_pos.x, carve_pos.z));
            if horizontal_dist < 1.4 {
                if pos.y < lowest_y {
                    lowest_y = pos.y;
                    lowest_scale_y = transform.scale.y;
                }
                if pos.y > highest_y {
                    highest_y = pos.y;
                    highest_scale_y = transform.scale.y;
                }
            }
        }

        let ground_y = lowest_y - lowest_scale_y / 2.0;
        let top_y = highest_y + highest_scale_y / 2.0;
        let door_height = (top_y - ground_y).clamp(1.8, 6.0);

        // Despawn all bricks within 1.2m horizontally and up to highest_y vertically to carve a gateway
        let mut rng = rand::rng();
        for (entity, global_transform, _) in brick_query.iter() {
            let pos = global_transform.translation();
            let horizontal_dist =
                Vec2::new(pos.x, pos.z).distance(Vec2::new(carve_pos.x, carve_pos.z));

            if horizontal_dist < 1.2 && pos.y <= top_y + 0.1 {
                commands.entity(entity).despawn();

                // Spark dust particles at each cleared brick
                for _ in 0..2 {
                    commands.spawn((
                        Mesh3d(procedural_wall_assets.dust_mesh.clone()),
                        MeshMaterial3d(procedural_wall_assets.dust_material.clone()),
                        Transform::from_translation(pos),
                        crate::player::interaction::Particle {
                            velocity: Vec3::new(
                                rng.random_range(-2.0..2.0),
                                rng.random_range(1.5..4.0),
                                rng.random_range(-2.0..2.0),
                            ),
                            lifetime: Timer::from_seconds(0.6, TimerMode::Once),
                        },
                    ));
                }
            }
        }

        // Spawn a beautiful, double-hinged medieval wooden castle gate centered in the archway!
        let mut gate_pos = carve_pos;
        gate_pos.y = ground_y;

        commands
            .spawn((
                Transform::from_translation(gate_pos).with_rotation(carve_rot),
                Visibility::default(),
                InheritedVisibility::default(),
            ))
            .with_children(|gate| {
                // Left Door Hinge (offset at left end of opening: -1.2m local X)
                gate.spawn((
                    crate::player::interaction::Door {
                        open: false,
                        hinge_side: -1.0,
                    },
                    Transform::from_xyz(-1.2, 0.0, 0.0),
                    Visibility::default(),
                    InheritedVisibility::default(),
                ))
                .with_children(|hinge| {
                    hinge.spawn((
                        Mesh3d(procedural_wall_assets.unit_cube.clone()),
                        MeshMaterial3d(materials.add(StandardMaterial {
                            base_color: Color::srgb(0.38, 0.22, 0.12), // Medieval dark oak wood
                            perceptual_roughness: 0.85,
                            ..default()
                        })),
                        Transform::from_xyz(0.6, door_height / 2.0, 0.0).with_scale(Vec3::new(
                            1.2,
                            door_height,
                            0.12,
                        )), // Center door panel between hinge and opening center
                        bevy_rapier3d::prelude::Collider::cuboid(0.6, door_height / 2.0, 0.06),
                    ));
                });

                // Right Door Hinge (offset at right end of opening: 1.2m local X)
                gate.spawn((
                    crate::player::interaction::Door {
                        open: false,
                        hinge_side: 1.0,
                    },
                    Transform::from_xyz(1.2, 0.0, 0.0),
                    Visibility::default(),
                    InheritedVisibility::default(),
                ))
                .with_children(|hinge| {
                    hinge.spawn((
                        Mesh3d(procedural_wall_assets.unit_cube.clone()),
                        MeshMaterial3d(materials.add(StandardMaterial {
                            base_color: Color::srgb(0.38, 0.22, 0.12), // Medieval dark oak wood
                            perceptual_roughness: 0.85,
                            ..default()
                        })),
                        Transform::from_xyz(-0.6, door_height / 2.0, 0.0).with_scale(Vec3::new(
                            1.2,
                            door_height,
                            0.12,
                        )), // Center door panel between hinge and opening center
                        bevy_rapier3d::prelude::Collider::cuboid(0.6, door_height / 2.0, 0.06),
                    ));
                });
            });

        // -----------------------------------------------------------------------
        // Spawn a semicircular arch above the carved opening
        // -----------------------------------------------------------------------
        // Compute impost positions accounting for wall rotation
        let rot_mat = bevy::math::Mat3::from_quat(carve_rot);
        let left_offset = rot_mat * Vec3::new(-1.2, 0.0, 0.0);
        let right_offset = rot_mat * Vec3::new(1.2, 0.0, 0.0);
        let arch_opening = ArchOpening {
            left_foot: carve_pos.with_y(top_y) + left_offset,
            right_foot: carve_pos.with_y(top_y) + right_offset,
        };

        // Unique ID for this arch (use top_y + position hash as rough unique key)
        let arch_id = (carve_pos.x.to_bits() as u64)
            .wrapping_add((carve_pos.z.to_bits() as u64) << 32)
            .wrapping_add(top_y.to_bits() as u64);

        spawn_arch_voussoirs(
            &arch_opening,
            arch_id,
            active_texture,
            None,
            &mut commands,
            &mut materials,
            &asset_server,
            &procedural_wall_assets,
        );

        println!("Carved Gate/Archway in Procedural Wall — arch spawned above!");
    }
}

// ---------------------------------------------------------------------------
// Auto-arch: detect close wall endpoints and bridge with an arch
// ---------------------------------------------------------------------------

/// Scans all `ProceduralBrick` entities each frame, finds wall endpoints that
/// are close to each other but unconnected, and spawns bridging arches.
fn detect_and_spawn_arches(
    mut commands: Commands,
    mut arch_registry: ResMut<ArchRegistry>,
    brick_query: Query<
        (
            Entity,
            &GlobalTransform,
            &Transform,
            Option<&BrickSpawnAnimation>,
        ),
        (With<ProceduralBrick>, Without<ProceduralArchBrick>),
    >,
    root_query: Query<Entity>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    asset_server: Res<AssetServer>,
    time: Res<Time>,
    procedural_wall_assets: Res<ProceduralWallAssets>,
    mut local_state: Local<(f32, usize)>, // (timer, last_brick_count)
) {
    let current_brick_count = brick_query.iter().count();
    let count_changed = current_brick_count != local_state.1;

    local_state.0 += time.delta_secs();

    // Throttle checks to twice a second (0.5s) unless the brick count has changed
    if !count_changed && local_state.0 < 0.5 {
        return;
    }

    local_state.0 = 0.0;
    local_state.1 = current_brick_count;

    #[derive(Clone, Copy)]
    struct BrickInfo {
        pos: Vec3,
        transform: Transform,
    }

    // Gather all brick positions and transforms
    let mut bricks = Vec::with_capacity(current_brick_count);
    for (_entity, _gt, transform, opt_anim) in brick_query.iter() {
        let pos = if let Some(anim) = opt_anim {
            anim.target_translation
        } else {
            transform.translation
        };
        bricks.push(BrickInfo {
            pos,
            transform: *transform,
        });
    }

    if bricks.len() < 2 {
        return;
    }

    // 1. Group bricks into stable vertical columns by horizontal position (tolerance: 0.1m)
    // Optimized grouping: Loop in reverse and use distance_squared to avoid sqrt calls
    let mut columns: Vec<Vec<BrickInfo>> = Vec::new();
    for brick in bricks {
        let brick_xz = Vec2::new(brick.pos.x, brick.pos.z);
        let mut found = false;
        for col in columns.iter_mut().rev() {
            let col_xz = Vec2::new(col[0].pos.x, col[0].pos.z);
            if col_xz.distance_squared(brick_xz) < 0.01 {
                // 0.1m * 0.1m = 0.01
                col.push(brick);
                found = true;
                break;
            }
        }
        if !found {
            columns.push(vec![brick]);
        }
    }

    // 2. Find the highest brick of each column to represent the column tops
    let mut column_tops: Vec<(Vec3, Transform)> = Vec::new();
    for col in &columns {
        if let Some(highest_brick) = col.iter().max_by(|a, b| {
            a.pos
                .y
                .partial_cmp(&b.pos.y)
                .unwrap_or(std::cmp::Ordering::Equal)
        }) {
            column_tops.push((highest_brick.pos, highest_brick.transform));
        }
    }

    // 3. Detect true wall endpoints using stable wall-tangent projection.
    // A column is in the middle of a wall if it has neighbor columns in both directions
    // along the wall's local tangent vector. If it lacks a neighbor on either side, it is an endpoint.
    let mut endpoints: Vec<WallEndpoint> = Vec::new();
    for &(ct_pos, ct_transform) in &column_tops {
        let ct_xz = Vec2::new(ct_pos.x, ct_pos.z);
        let tangent = ct_transform.rotation * Vec3::X;

        let mut has_forward = false;
        let mut has_backward = false;

        for &(other_pos, _) in &column_tops {
            if other_pos == ct_pos {
                continue;
            }
            let other_xz = Vec2::new(other_pos.x, other_pos.z);
            // Optimized distance check using distance_squared (1.2m * 1.2m = 1.44)
            if ct_xz.distance_squared(other_xz) < 1.44 {
                let disp = other_pos - ct_pos;
                let dot = disp.dot(tangent);

                if dot > 0.15 {
                    has_forward = true;
                } else if dot < -0.15 {
                    has_backward = true;
                }
            }
        }

        if !(has_forward && has_backward) {
            endpoints.push(WallEndpoint {
                top_center: ct_pos,
                bottom_center: Vec3::new(ct_pos.x, 0.0, ct_pos.z),
                is_right_end: false,
            });
        }
    }

    // 4. Prune existing arches that are no longer supported by current endpoints.
    // An arch is valid only if there is still an endpoint near its left foot AND an endpoint near its right foot.
    let mut active_arches = Vec::new();
    for (left_xz, right_xz, root) in arch_registry.arches.drain(..) {
        let has_left = endpoints
            .iter()
            .any(|ep| Vec2::new(ep.top_center.x, ep.top_center.z).distance_squared(left_xz) < 0.36); // 0.6m * 0.6m = 0.36
        let has_right = endpoints.iter().any(|ep| {
            Vec2::new(ep.top_center.x, ep.top_center.z).distance_squared(right_xz) < 0.36
        });

        if has_left && has_right {
            active_arches.push((left_xz, right_xz, root));
        } else {
            // Despawn the orphaned arch and all of its voussoir children!
            if root_query.contains(root) {
                commands.entity(root).despawn();
            }
        }
    }
    arch_registry.arches = active_arches;

    // 5. Find candidate arch openings from active endpoints.
    let openings = find_arch_openings(&endpoints);

    let active_texture = "textures/solid_stone.png";

    for opening in openings {
        let left_xz = Vec2::new(opening.left_foot.x, opening.left_foot.z);
        let right_xz = Vec2::new(opening.right_foot.x, opening.right_foot.z);

        // Check if an arch for this opening already exists in the registry.
        let already_registered = arch_registry.arches.iter().any(|(l, r, _)| {
            l.distance_squared(left_xz) < 0.36 && r.distance_squared(right_xz) < 0.36
        });
        if already_registered {
            continue;
        }

        // Unique ID from foot positions.
        let arch_id = (opening.left_foot.x.to_bits() as u64)
            .wrapping_add((opening.left_foot.z.to_bits() as u64) << 16)
            .wrapping_add((opening.right_foot.x.to_bits() as u64) << 32)
            .wrapping_add((opening.right_foot.z.to_bits() as u64) << 48);

        // Spawn a root entity at Identity so children world translations are correct!
        let root = commands
            .spawn((
                Transform::default(),
                Visibility::default(),
                InheritedVisibility::default(),
            ))
            .id();

        spawn_arch_voussoirs(
            &opening,
            arch_id,
            active_texture,
            Some(root),
            &mut commands,
            &mut materials,
            &asset_server,
            &procedural_wall_assets,
        );

        arch_registry.arches.push((left_xz, right_xz, root));
    }
}
