use bevy::prelude::*;
use bevy::asset::RenderAssetUsages;
use bevy::mesh::Indices;
use bevy::mesh::VertexAttributeValues;
use bevy::shader::ShaderRef;
use bevy::render::render_resource::*;
use bevy_voxel_world::prelude::*;
use crate::world::noise_generator::NoiseGenerator;
use crate::player::camera::PhysicsState;
use rand::RngExt;

pub struct WaterPlugin;

impl Plugin for WaterPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins(MaterialPlugin::<WaterMaterial>::default())
            .add_systems(Startup, setup_water)
            .add_systems(Update, (
                center_water_on_player,
                water_sim,
                animate_water_mesh,
                update_water_material,
                update_water_wall_mask,
                player_water_interaction,
                handle_mouse_clicks,
                apply_buoyancy,
                boat_ai,
            ));
    }
}

#[derive(Component)]
pub struct Boat;

#[derive(Component)]
pub struct Buoyant {
    pub force: f32,
}

#[derive(Component)]
pub struct WaterMesh {
    pub handle: Handle<Mesh>,
}

#[derive(Component)]
pub struct WaterSimData {
    pub height: Vec<f32>,
    pub flow_x: Vec<f32>,
    pub flow_y: Vec<f32>,
    pub wall_mask: Vec<bool>,
    pub last_disturbed_pos: Option<(usize, usize)>,
    pub size: f32, // World width/depth of the simulation plane
    pub grid_len: usize,
}

impl WaterSimData {
    pub fn new(grid_len: usize, size: f32) -> Self {
        let count = grid_len * grid_len;
        let mut sim = Self {
            height: vec![1.0; count],
            flow_x: vec![0.0; count],
            flow_y: vec![0.0; count],
            wall_mask: vec![false; count],
            last_disturbed_pos: None,
            size,
            grid_len,
        };
        
        // Setup default borders as walls
        for i in 0..grid_len {
            sim.set_wall(i, 0, true);
            sim.set_wall(i, grid_len - 1, true);
            sim.set_wall(0, i, true);
            sim.set_wall(grid_len - 1, i, true);
        }
        
        sim
    }

    #[inline]
    pub fn idx(&self, x: usize, y: usize) -> usize {
        x * self.grid_len + y
    }

    #[inline]
    pub fn get_height(&self, x: usize, y: usize) -> f32 {
        self.height[self.idx(x, y)]
    }

    #[inline]
    pub fn set_height(&mut self, x: usize, y: usize, val: f32) {
        let idx = self.idx(x, y);
        self.height[idx] = val;
    }

    #[inline]
    pub fn get_flow_x(&self, x: usize, y: usize) -> f32 {
        self.flow_x[self.idx(x, y)]
    }

    #[inline]
    pub fn set_flow_x(&mut self, x: usize, y: usize, val: f32) {
        let idx = self.idx(x, y);
        self.flow_x[idx] = val;
    }

    #[inline]
    pub fn get_flow_y(&self, x: usize, y: usize) -> f32 {
        self.flow_y[self.idx(x, y)]
    }

    #[inline]
    pub fn set_flow_y(&mut self, x: usize, y: usize, val: f32) {
        let idx = self.idx(x, y);
        self.flow_y[idx] = val;
    }

    #[inline]
    pub fn is_wall(&self, x: usize, y: usize) -> bool {
        self.wall_mask[self.idx(x, y)]
    }

    #[inline]
    pub fn set_wall(&mut self, x: usize, y: usize, val: bool) {
        let idx = self.idx(x, y);
        self.wall_mask[idx] = val;
    }
}

#[derive(Asset, TypePath, AsBindGroup, Debug, Clone)]
pub struct WaterMaterial {
    #[uniform(0)]
    pub color: Vec4,
    #[uniform(0)]
    pub time: f32,
    #[uniform(0)]
    pub camera_position: Vec3,
    #[uniform(0)]
    pub resolution: Vec2,
    #[uniform(0)]
    pub water_level: f32,
    #[uniform(0)]
    pub grid_scale: f32,
}

impl WaterMaterial {
    pub fn new(color: Color) -> Self {
        let c = color.to_linear();
        Self {
            color: Vec4::new(c.red, c.green, c.blue, c.alpha),
            time: 0.0,
            camera_position: Vec3::ZERO,
            resolution: Vec2::new(1920.0, 1080.0),
            water_level: 15.0,
            grid_scale: 256.0 / 128.0,
        }
    }
}

impl Material for WaterMaterial {
    fn fragment_shader() -> ShaderRef {
        "shaders/water_material.wgsl".into()
    }
    
    fn alpha_mode(&self) -> AlphaMode {
        AlphaMode::Blend
    }
}

pub fn create_water_mesh(size: f32, grid_size: usize) -> Mesh {
    let vertices_per_side = grid_size + 1;
    let mut positions = Vec::new();
    let mut normals = Vec::new();
    let mut uvs = Vec::new();
    let mut indices = Vec::new();

    let step = size / grid_size as f32;
    let half_size = size * 0.5;

    // Generate vertices
    for y in 0..vertices_per_side {
        for x in 0..vertices_per_side {
            let x_pos = (x as f32 * step) - half_size;
            let z_pos = (y as f32 * step) - half_size;

            positions.push([x_pos, 0.0, z_pos]);
            normals.push([0.0, 1.0, 0.0]);
            uvs.push([x as f32 / grid_size as f32, y as f32 / grid_size as f32]);
        }
    }

    // Generate indices
    for y in 0..grid_size {
        for x in 0..grid_size {
            let base = (y * vertices_per_side + x) as u32;

            // First triangle
            indices.push(base);
            indices.push(base + vertices_per_side as u32);
            indices.push(base + 1);

            // Second triangle
            indices.push(base + 1);
            indices.push(base + vertices_per_side as u32);
            indices.push(base + vertices_per_side as u32 + 1);
        }
    }

    let mut mesh = Mesh::new(PrimitiveTopology::TriangleList, RenderAssetUsages::default());
    mesh.insert_attribute(Mesh::ATTRIBUTE_POSITION, positions);
    mesh.insert_attribute(Mesh::ATTRIBUTE_NORMAL, normals);
    mesh.insert_attribute(Mesh::ATTRIBUTE_UV_0, uvs);
    mesh.insert_indices(Indices::U32(indices));
    mesh
}

fn setup_water(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut water_materials: ResMut<Assets<WaterMaterial>>,
) {
    let size = 512.0;
    let grid_len = 128;
    
    // Create high-fidelity water mesh
    let mesh_handle = meshes.add(create_water_mesh(size, grid_len));
    
    // Setup beautiful custom translucent WaterMaterial
    let material_handle = water_materials.add(WaterMaterial::new(Color::srgba(0.02, 0.32, 0.78, 0.78)));
    
    // Spawn the simulated water plane entity
    commands.spawn((
        Name::new("SimulatedWaterPlane"),
        Mesh3d(mesh_handle.clone()),
        MeshMaterial3d(material_handle),
        Transform::from_xyz(0.0, 15.1, 0.0), // Spawn at sea level
        WaterSimData::new(grid_len, size),
        WaterMesh { handle: mesh_handle },
    ));
}

fn water_sim(
    time: Res<Time>,
    mut query: Query<&mut WaterSimData>,
) {
    let delta_time = time.delta_secs().min(0.03); // Cap dt to prevent wave explosions
    let gravity: f32 = 12.0;
    let friction: f32 = 0.975; // Smooth damping coefficient - raised from 0.7 for beautiful, long-lived wave propagation!

    for mut water_data in query.iter_mut() {
        let grid_len = water_data.grid_len;

        // Clear border flows
        for i in 0..grid_len {
            water_data.set_flow_x(0, i, 0.0);
            water_data.set_flow_x(grid_len - 1, i, 0.0);
            water_data.set_flow_y(i, 0, 0.0);
            water_data.set_flow_y(i, grid_len - 1, 0.0);
        }

        // Calculate flow based on height difference
        for x in 0..grid_len {
            for y in 0..grid_len {
                if x > 0 {
                    let source_has_wall = water_data.is_wall(x - 1, y);
                    let dest_has_wall = water_data.is_wall(x, y);
                    let height_diff = water_data.get_height(x - 1, y) - water_data.get_height(x, y);

                    if !source_has_wall && !dest_has_wall {
                        let current_flow = water_data.get_flow_x(x, y);
                        let new_flow = current_flow * friction.powf(delta_time) + height_diff * gravity * delta_time;
                        water_data.set_flow_x(x, y, new_flow);
                    } else {
                        water_data.set_flow_x(x, y, 0.0);
                    }
                } else {
                    water_data.set_flow_x(x, y, 0.0);
                }

                if y > 0 {
                    let source_has_wall = water_data.is_wall(x, y - 1);
                    let dest_has_wall = water_data.is_wall(x, y);
                    let height_diff = water_data.get_height(x, y - 1) - water_data.get_height(x, y);

                    if !source_has_wall && !dest_has_wall {
                        let current_flow = water_data.get_flow_y(x, y);
                        let new_flow = current_flow * friction.powf(delta_time) + height_diff * gravity * delta_time;
                        water_data.set_flow_y(x, y, new_flow);
                    } else {
                        water_data.set_flow_y(x, y, 0.0);
                    }
                } else {
                    water_data.set_flow_y(x, y, 0.0);
                }
            }
        }

        // Outflow scaling to prevent grid cells draining below zero height
        for x in 0..grid_len {
            for y in 0..grid_len {
                if water_data.is_wall(x, y) {
                    continue;
                }

                let mut total_outflow = 0.0;
                total_outflow += 0.0f32.max(-water_data.get_flow_x(x, y));
                total_outflow += 0.0f32.max(-water_data.get_flow_y(x, y));

                if x < grid_len - 1 {
                    total_outflow += 0.0f32.max(water_data.get_flow_x(x + 1, y));
                }
                if y < grid_len - 1 {
                    total_outflow += 0.0f32.max(water_data.get_flow_y(x, y + 1));
                }

                let max_outflow = water_data.get_height(x, y) / delta_time;

                if total_outflow > 0.0 {
                    let scale = 1.0f32.min(max_outflow / total_outflow);
                    if water_data.get_flow_x(x, y) < 0.0 {
                        let val = water_data.get_flow_x(x, y) * scale;
                        water_data.set_flow_x(x, y, val);
                    }
                    if water_data.get_flow_y(x, y) < 0.0 {
                        let val = water_data.get_flow_y(x, y) * scale;
                        water_data.set_flow_y(x, y, val);
                    }
                    if x < grid_len - 1 && water_data.get_flow_x(x + 1, y) > 0.0 {
                        let val = water_data.get_flow_x(x + 1, y) * scale;
                        water_data.set_flow_x(x + 1, y, val);
                    }
                    if y < grid_len - 1 && water_data.get_flow_y(x, y + 1) > 0.0 {
                        let val = water_data.get_flow_y(x, y + 1) * scale;
                        water_data.set_flow_y(x, y + 1, val);
                    }
                }
            }
        }

        // Apply flows and update heights
        for x in 0..grid_len {
            for y in 0..grid_len {
                let mut height_change = 0.0;

                let can_receive_from_left = x > 0 && !water_data.is_wall(x - 1, y) && !water_data.is_wall(x, y);
                if can_receive_from_left {
                    height_change += water_data.get_flow_x(x, y);
                }

                let can_receive_from_top = y > 0 && !water_data.is_wall(x, y - 1) && !water_data.is_wall(x, y);
                if can_receive_from_top {
                    height_change += water_data.get_flow_y(x, y);
                }

                let can_flow_right = x < grid_len - 1 && !water_data.is_wall(x + 1, y);
                if can_flow_right {
                    height_change -= water_data.get_flow_x(x + 1, y);
                }

                let can_flow_bottom = y < grid_len - 1 && !water_data.is_wall(x, y + 1);
                if can_flow_bottom {
                    height_change -= water_data.get_flow_y(x, y + 1);
                }

                let current_height = water_data.get_height(x, y);
                let mut new_height = current_height + height_change * delta_time;

                new_height = new_height.max(0.1);

                if water_data.is_wall(x, y) {
                    new_height = 0.1;
                }

                water_data.set_height(x, y, new_height);
            }
        }
    }
}

fn animate_water_mesh(
    mut meshes: ResMut<Assets<Mesh>>,
    query: Query<(&WaterMesh, &WaterSimData)>,
) {
    for (water_mesh, water_data) in query.iter() {
        if let Some(mesh) = meshes.get_mut(&water_mesh.handle) {
            let grid_len = water_data.grid_len;
            let vertices_per_side = grid_len + 1;
            let grid_scale = water_data.size / grid_len as f32;

            // 1. Update Positions
            if let Some(vertex_attr) = mesh.attribute_mut(Mesh::ATTRIBUTE_POSITION) {
                if let VertexAttributeValues::Float32x3(positions) = vertex_attr {
                    for (i, pos) in positions.iter_mut().enumerate() {
                        let x_idx = i % vertices_per_side;
                        let y_idx = i / vertices_per_side;

                        let grid_x = x_idx.min(grid_len - 1);
                        let grid_y = y_idx.min(grid_len - 1);

                        // Height = height - 1.0 so rest is at Y=0.0 relative to spawning height (15.1)
                        pos[1] = water_data.get_height(grid_x, grid_y) - 1.0;
                    }
                }
            }

            // 2. Recalculate Normals (CPU)
            let positions_copy = if let Some(pos_attr) = mesh.attribute(Mesh::ATTRIBUTE_POSITION) {
                if let VertexAttributeValues::Float32x3(positions) = pos_attr {
                    Some(positions.clone())
                } else { None }
            } else { None };

            if let Some(positions) = positions_copy {
                if let Some(norm_attr) = mesh.attribute_mut(Mesh::ATTRIBUTE_NORMAL) {
                    if let VertexAttributeValues::Float32x3(normals) = norm_attr {
                        for i in 0..normals.len() {
                            let x = i % vertices_per_side;
                            let y = i / vertices_per_side;

                            let mut dx = 0.0;
                            let mut dy = 0.0;

                            if x > 0 && x < vertices_per_side - 1 {
                                let h_left = positions[i - 1][1];
                                let h_right = positions[i + 1][1];
                                dx = (h_right - h_left) / (2.0 * grid_scale);
                            }

                            if y > 0 && y < vertices_per_side - 1 {
                                let h_up = positions[i - vertices_per_side][1];
                                let h_down = positions[i + vertices_per_side][1];
                                dy = (h_down - h_up) / (2.0 * grid_scale);
                            }

                            let normal = Vec3::new(-dx, 1.0, -dy).normalize();
                            normals[i] = [normal.x, normal.y, normal.z];
                        }
                    }
                }
            }
        }
    }
}

fn update_water_material(
    time: Res<Time>,
    camera_query: Query<&Transform, With<Camera3d>>,
    windows: Query<&Window>,
    mut water_materials: ResMut<Assets<WaterMaterial>>,
    water_query: Query<&MeshMaterial3d<WaterMaterial>>,
) {
    let camera_position = camera_query.single().map(|t| t.translation).unwrap_or(Vec3::ZERO);
    let resolution = windows.single().map(|w| Vec2::new(w.width(), w.height())).unwrap_or(Vec2::new(1920.0, 1080.0));

    for mat_handle in water_query.iter() {
        if let Some(material) = water_materials.get_mut(&mat_handle.0) {
            material.time = time.elapsed_secs();
            material.camera_position = camera_position;
            material.resolution = resolution;
        }
    }
}

fn update_water_wall_mask(
    voxel_world: VoxelWorld<NoiseGenerator>,
    mut query: Query<(&mut WaterSimData, &Transform)>,
    time: Res<Time>,
    mut timer: Local<f32>,
) {
    // Throttle wall mask updates to 4 times per second for maximum rendering speed
    *timer += time.delta_secs();
    if *timer < 0.25 { return; }
    *timer = 0.0;

    for (mut water_data, transform) in query.iter_mut() {
        let grid_len = water_data.grid_len;
        let half_size = water_data.size * 0.5;
        let cell_size = water_data.size / grid_len as f32;
        let center = transform.translation;

        for x in 0..grid_len {
            for y in 0..grid_len {
                // Ensure border limits act as walls
                if x == 0 || x == grid_len - 1 || y == 0 || y == grid_len - 1 {
                    water_data.set_wall(x, y, true);
                    continue;
                }

                let world_x = center.x - half_size + (x as f32 * cell_size);
                let world_z = center.z - half_size + (y as f32 * cell_size);

                // Fetch voxel at sea level height (Y=15)
                let voxel_pos = IVec3::new(world_x.round() as i32, 15, world_z.round() as i32);
                let voxel = voxel_world.get_voxel(voxel_pos);

                let is_solid_terrain = match voxel {
                    WorldVoxel::Solid(mat_id) => mat_id != 1, // Any block other than voxel water is land
                    WorldVoxel::Air => false,
                    WorldVoxel::Unset => false,
                };

                water_data.set_wall(x, y, is_solid_terrain);
            }
        }
    }
}

fn player_water_interaction(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    time: Res<Time>,
    player_query: Query<(&Transform, &PhysicsState)>,
    mut water_query: Query<(&mut WaterSimData, &Transform)>,
) {
    let Ok((player_transform, player_physics)) = player_query.single() else { return };
    
    // Check if player is wading/swimming in water
    let in_water = player_transform.translation.y < 16.5;
    
    if player_physics.flying {
        // Player is flying: Check if close enough for rocket thrust to disturb surface!
        for (mut water_data, transform) in water_query.iter_mut() {
            let grid_len = water_data.grid_len;
            let half_size = water_data.size * 0.5;
            let center = transform.translation;

            let grid_center = Vec2::new(center.x, center.z);
            let w_height = get_water_height(player_transform.translation.x, player_transform.translation.z, grid_center, &water_data);
            let dist = player_transform.translation.y - w_height;

            // Highly realistic high-altitude rocket exhaust propagation! (up to 25.0 meters)
            if dist > -2.0 && dist < 25.0 {
                let dx = player_transform.translation.x - (center.x - half_size);
                let dz = player_transform.translation.z - (center.z - half_size);

                let grid_x = ((dx / water_data.size) * grid_len as f32) as i32;
                let grid_z = ((dz / water_data.size) * grid_len as f32) as i32;

                if grid_x >= 3 && grid_x < (grid_len - 3) as i32 &&
                   grid_z >= 3 && grid_z < (grid_len - 3) as i32 {
                    
                    // Stronger force the closer we are to the surface
                    let force = ((25.0 - dist.max(0.0)) / 25.0).clamp(0.0, 1.0);
                    let dt = time.delta_secs().min(0.03);
                    
                    // Apply downward displacement force to clear the center cell (creating a beautiful physical depression hollow)
                    let center_gx = grid_x as usize;
                    let center_gz = grid_z as usize;
                    if !water_data.is_wall(center_gx, center_gz) {
                        let h = water_data.get_height(center_gx, center_gz);
                        water_data.set_height(center_gx, center_gz, (h - force * 0.28 * dt).max(0.2));
                    }

                    // Directly inject physical OUTWARD flow velocity vectors to adjacent cells!
                    // This forces water outwards away from the player, generating gorgeous expanding wave ridges
                    let push_strength = force * 60.0 * (1.0 + 0.35 * (time.elapsed_secs() * 32.0).sin());

                    for nx in (grid_x - 3)..=(grid_x + 3) {
                        for nz in (grid_z - 3)..=(grid_z + 3) {
                            let gx_n = nx as usize;
                            let gz_n = nz as usize;
                            if !water_data.is_wall(gx_n, gz_n) {
                                let dir_x = (nx - grid_x) as f32;
                                let dir_z = (nz - grid_z) as f32;
                                let dist_g = (dir_x * dir_x + dir_z * dir_z).sqrt();
                                
                                if dist_g > 0.1 {
                                    let u_x = dir_x / dist_g;
                                    let u_z = dir_z / dist_g;
                                    let weight = (1.0 - dist_g * 0.28).max(0.0);
                                    
                                    // Inject velocity into the shallow water equations!
                                    let current_fx = water_data.get_flow_x(gx_n, gz_n);
                                    water_data.set_flow_x(gx_n, gz_n, current_fx + u_x * push_strength * weight * dt);

                                    let current_fy = water_data.get_flow_y(gx_n, gz_n);
                                    water_data.set_flow_y(gx_n, gz_n, current_fy + u_z * push_strength * weight * dt);
                                }
                            }
                        }
                    }

                    // Spawn physical, translucent water foam spray particles shooting outward
                    let mut rng = rand::rng();
                    // Scale particle spawn rate by proximity force
                    let spawn_chance = 0.12 + force * 0.28;
                    if rng.random_bool(spawn_chance as f64) {
                        let particle_count = if force > 0.6 { 2 } else { 1 };
                        for _ in 0..particle_count {
                            let angle = rng.random_range(0.0..std::f32::consts::TAU);
                            let speed = rng.random_range(2.5..6.0) * force;
                            let p_vel = Vec3::new(angle.cos() * speed, rng.random_range(0.8..3.5) * force, angle.sin() * speed);
                            let p_pos = Vec3::new(player_transform.translation.x, w_height, player_transform.translation.z) + Vec3::new(rng.random_range(-0.6..0.6), 0.1, rng.random_range(-0.6..0.6));
                            commands.spawn((
                                Mesh3d(meshes.add(Cuboid::from_size(Vec3::splat(rng.random_range(0.08..0.22))))),
                                MeshMaterial3d(materials.add(StandardMaterial {
                                    base_color: Color::srgba(0.88, 0.96, 1.0, 0.65), // Translucent foam
                                    emissive: LinearRgba::from(Color::srgb(0.0, 0.25, 0.35)), // High-tech light emission
                                    alpha_mode: AlphaMode::Blend,
                                    ..default()
                                })),
                                Transform::from_translation(p_pos),
                                crate::player::interaction::Particle {
                                    velocity: p_vel,
                                    lifetime: Timer::from_seconds(rng.random_range(0.5..0.9), TimerMode::Once),
                                },
                            ));
                        }
                    }
                }
            }
        }
    } else if in_water {
        let p_velocity = player_physics.velocity;
        let speed = p_velocity.length();
        if speed < 0.15 { return; }

        for (mut water_data, transform) in water_query.iter_mut() {
            let grid_len = water_data.grid_len;
            let half_size = water_data.size * 0.5;
            let center = transform.translation;

            let dx = player_transform.translation.x - (center.x - half_size);
            let dz = player_transform.translation.z - (center.z - half_size);

            let grid_x = ((dx / water_data.size) * grid_len as f32) as i32;
            let grid_z = ((dz / water_data.size) * grid_len as f32) as i32;

            if grid_x >= 1 && grid_x < (grid_len - 1) as i32 &&
               grid_z >= 1 && grid_z < (grid_len - 1) as i32 {
                let gx = grid_x as usize;
                let gz = grid_z as usize;

                let current_height = water_data.get_height(gx, gz);
                let ripple_amount = speed * 0.12;
                water_data.set_height(gx, gz, current_height + ripple_amount);

                // Smooth the ripple outwards to neighbors
                for nx in (gx - 1)..=(gx + 1) {
                    for nz in (gz - 1)..=(gz + 1) {
                        if !water_data.is_wall(nx, nz) {
                            let h = water_data.get_height(nx, nz);
                            water_data.set_height(nx, nz, h + ripple_amount * 0.4);
                        }
                    }
                }
            }
        }
    }
}

fn handle_mouse_clicks(
    mouse_button: Res<ButtonInput<MouseButton>>,
    camera_query: Query<(&Camera, &GlobalTransform)>,
    windows: Query<&Window>,
    mut water_query: Query<(&mut WaterSimData, &Transform)>,
) {
    if mouse_button.pressed(MouseButton::Left) {
        let Ok((camera, camera_transform)) = camera_query.single() else { return };
        let Ok(window) = windows.single() else { return };
        
        if let Some(cursor_position) = window.cursor_position() {
            if let Ok(ray) = camera.viewport_to_world(camera_transform, cursor_position) {
                // Calculate intersection with water sea level plane (y = 15.0)
                let t = (15.0 - ray.origin.y) / ray.direction.y;
                if t > 0.0 {
                    let hit_point = ray.origin + ray.direction * t;

                    for (mut water_data, transform) in water_query.iter_mut() {
                        let grid_len = water_data.grid_len;
                        let half_size = water_data.size * 0.5;
                        let center = transform.translation;

                        let dx = hit_point.x - (center.x - half_size);
                        let dz = hit_point.z - (center.z - half_size);

                        let grid_x = ((dx / water_data.size) * grid_len as f32) as i32;
                        let grid_z = ((dz / water_data.size) * grid_len as f32) as i32;

                        if grid_x >= 1 && grid_x < (grid_len - 1) as i32 &&
                           grid_z >= 1 && grid_z < (grid_len - 1) as i32 {
                            let gx = grid_x as usize;
                            let gz = grid_z as usize;

                            if !water_data.is_wall(gx, gz) {
                                let should_disturb = match water_data.last_disturbed_pos {
                                    Some((lx, lz)) => lx != gx || lz != gz,
                                    None => true,
                                };

                                if should_disturb {
                                    let h = water_data.get_height(gx, gz);
                                    water_data.set_height(gx, gz, h + 1.2);
                                    water_data.last_disturbed_pos = Some((gx, gz));
                                }
                            }
                        }
                    }
                }
            }
        }
    } else {
        for (mut water_data, _) in water_query.iter_mut() {
            water_data.last_disturbed_pos = None;
        }
    }
}

pub fn get_water_height(world_x: f32, world_z: f32, grid_center: Vec2, water_sim: &WaterSimData) -> f32 {
    let half_size = water_sim.size * 0.5;
    let x_offset = world_x - (grid_center.x - half_size);
    let z_offset = world_z - (grid_center.y - half_size);
    let grid_len = water_sim.grid_len;
    
    let grid_x_f = (x_offset / water_sim.size) * grid_len as f32;
    let grid_z_f = (z_offset / water_sim.size) * grid_len as f32;
    
    if grid_x_f >= 0.0 && grid_x_f < (grid_len - 1) as f32 &&
       grid_z_f >= 0.0 && grid_z_f < (grid_len - 1) as f32 {
        let x0 = grid_x_f.floor() as usize;
        let x1 = x0 + 1;
        let z0 = grid_z_f.floor() as usize;
        let z1 = z0 + 1;
        
        let tx = grid_x_f.fract();
        let tz = grid_z_f.fract();
        
        // Bilinear interpolation
        let h00 = water_sim.get_height(x0, z0);
        let h10 = water_sim.get_height(x1, z0);
        let h01 = water_sim.get_height(x0, z1);
        let h11 = water_sim.get_height(x1, z1);
        
        let h0 = h00 * (1.0 - tx) + h10 * tx;
        let h1 = h01 * (1.0 - tx) + h11 * tx;
        
        // Sea level Y is 15.0. At rest heights are 1.0, so offset is (interpolated_height - 1.0)
        15.0 + (h0 * (1.0 - tz) + h1 * tz - 1.0)
    } else {
        15.0 // Sea level default
    }
}

fn apply_buoyancy(
    time: Res<Time>,
    water_query: Query<(&WaterSimData, &Transform), Without<Buoyant>>,
    mut query: Query<(&mut Transform, Option<&mut PhysicsState>, &Buoyant), (With<Buoyant>, Without<WaterSimData>)>,
) {
    let dt = time.delta_secs();
    let Ok((water_sim, water_transform)) = water_query.single() else {
        return;
    };
    
    let grid_center = Vec2::new(water_transform.translation.x, water_transform.translation.z);

    for (mut transform, mut physics, buoyant) in query.iter_mut() {
        let pos = transform.translation;
        
        // Buoyancy check against the interpolated dynamic water height
        let w_height = get_water_height(pos.x, pos.z, grid_center, water_sim);
        if pos.y < w_height {
            if let Some(ref mut p) = physics {
                p.velocity.y += buoyant.force * dt;
                p.velocity.y = p.velocity.y.clamp(-1.0, 5.0);
            } else {
                transform.translation.y += buoyant.force * 0.1 * dt;
            }
        }
    }
}

fn boat_ai(
    time: Res<Time>,
    water_query: Query<(&WaterSimData, &Transform), Without<Boat>>,
    mut query: Query<(&mut Transform, &Boat)>,
) {
    let dt = time.delta_secs();
    let Ok((water_sim, water_transform)) = water_query.single() else {
        return;
    };
    let grid_center = Vec2::new(water_transform.translation.x, water_transform.translation.z);

    for (mut transform, _) in query.iter_mut() {
        let pos = transform.translation;
        let w_height = get_water_height(pos.x, pos.z, grid_center, water_sim);
        
        // Boat keeps floating nicely at the wave surface
        if pos.y < w_height + 0.2 {
            transform.translation.y += ((w_height + 0.1) - pos.y) * 4.0 * dt; 
        } else {
            // Gravity
            transform.translation.y -= 9.8 * dt;
        }
    }
}

fn center_water_on_player(
    player_query: Query<&Transform, (With<PhysicsState>, Without<WaterMesh>)>,
    mut water_query: Query<(&mut Transform, &mut WaterSimData), With<WaterMesh>>,
) {
    let Ok(player_transform) = player_query.single() else { return };
    let Ok((mut water_transform, mut water_data)) = water_query.single_mut() else { return };

    let grid_len = water_data.grid_len;
    let size = water_data.size;
    let cell_size = size / grid_len as f32;

    // Snap target position to grid cell increments
    let target_x = (player_transform.translation.x / cell_size).round() * cell_size;
    let target_z = (player_transform.translation.z / cell_size).round() * cell_size;

    let dx = target_x - water_transform.translation.x;
    let dz = target_z - water_transform.translation.z;

    let shift_x = (dx / cell_size).round() as i32;
    let shift_z = (dz / cell_size).round() as i32;

    if shift_x != 0 || shift_z != 0 {
        let mut new_height = vec![1.0; grid_len * grid_len];
        let mut new_flow_x = vec![0.0; grid_len * grid_len];
        let mut new_flow_y = vec![0.0; grid_len * grid_len];
        let mut new_wall_mask = vec![false; grid_len * grid_len];

        for x in 0..grid_len {
            for y in 0..grid_len {
                let old_x = x as i32 + shift_x;
                let old_y = y as i32 + shift_z;

                let new_idx = x * grid_len + y;

                if old_x >= 0 && old_x < grid_len as i32 && old_y >= 0 && old_y < grid_len as i32 {
                    let old_idx = (old_x as usize) * grid_len + (old_y as usize);
                    new_height[new_idx] = water_data.height[old_idx];
                    new_flow_x[new_idx] = water_data.flow_x[old_idx];
                    new_flow_y[new_idx] = water_data.flow_y[old_idx];
                    new_wall_mask[new_idx] = water_data.wall_mask[old_idx];
                }
            }
        }

        water_data.height = new_height;
        water_data.flow_x = new_flow_x;
        water_data.flow_y = new_flow_y;
        water_data.wall_mask = new_wall_mask;

        water_transform.translation.x = target_x;
        water_transform.translation.z = target_z;
    }
}