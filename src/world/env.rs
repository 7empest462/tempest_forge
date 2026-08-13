use bevy::prelude::*;
use bevy::render::render_resource::AsBindGroup;
use bevy::shader::ShaderRef;
use std::f32::consts::PI;

pub struct EnvironmentPlugin;

impl Plugin for EnvironmentPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins(MaterialPlugin::<SkyMaterial>::default())
            .insert_resource(TimeOfDay::default())
            .add_systems(Startup, setup_sky_dome)
            .add_systems(
                Update,
                (
                    update_time,
                    update_sun,
                    update_sky,
                    update_sky_dome,
                    update_light_shadows,
                ),
            );
    }
}

#[derive(Asset, TypePath, AsBindGroup, Debug, Clone)]
pub struct SkyMaterial {
    #[uniform(0)]
    pub color: Vec4,
    #[uniform(0)]
    pub time: f32,
    #[uniform(0)]
    pub cloudiness: f32,
    #[uniform(0)]
    pub is_alien: f32,
}

impl Material for SkyMaterial {
    fn fragment_shader() -> ShaderRef {
        "shaders/sky_material.wgsl".into()
    }

    fn alpha_mode(&self) -> AlphaMode {
        AlphaMode::Opaque
    }
}

#[derive(Resource)]
pub struct TimeOfDay {
    pub hour: f32, // 0.0 to 24.0
    pub speed: f32,
}

impl Default for TimeOfDay {
    fn default() -> Self {
        Self {
            hour: 10.0,
            speed: 0.01,
        } // Reduced speed for much longer days
    }
}

#[derive(Component)]
pub struct Sun;

#[derive(Component)]
pub struct Moon;

#[derive(Component)]
pub struct SkyDome;

#[derive(Component)]
pub struct MoonBody;

fn generate_moon_crater_texture() -> Image {
    let width = 256;
    let height = 256;
    let mut data = vec![0u8; width * height * 4];

    // Seeded crater centers (cx, cy, radius, depth)
    let craters = [
        (60.0, 70.0, 22.0, 0.7),
        (160.0, 140.0, 30.0, 0.8),
        (180.0, 60.0, 16.0, 0.6),
        (90.0, 180.0, 25.0, 0.75),
        (120.0, 100.0, 14.0, 0.5),
        (210.0, 190.0, 18.0, 0.65),
        (40.0, 160.0, 12.0, 0.5),
        (140.0, 30.0, 20.0, 0.7),
        (80.0, 120.0, 10.0, 0.4),
        (190.0, 110.0, 15.0, 0.55),
    ];

    for y in 0..height {
        for x in 0..width {
            let fx = x as f32;
            let fy = y as f32;

            // Base silvery lunar dust
            let mut base_r = 180.0f32;
            let mut base_g = 195.0f32;
            let mut base_b = 220.0f32;

            // Lunar maria noise (darker basalt basins)
            let maria = ((fx * 0.03).sin() + (fy * 0.03).cos()) * 0.5 + 0.5;
            if maria > 0.65 {
                base_r *= 0.65;
                base_g *= 0.70;
                base_b *= 0.75;
            }

            // Apply craters with raised bright rims
            for &(cx, cy, radius, depth) in &craters {
                let dist = ((fx - cx).powi(2) + (fy - cy).powi(2)).sqrt();
                if dist < radius {
                    let norm_d = dist / radius;
                    if norm_d > 0.82 {
                        // Bright raised crater rim highlight
                        base_r = (base_r * 1.4).min(255.0);
                        base_g = (base_g * 1.4).min(255.0);
                        base_b = (base_b * 1.4).min(255.0);
                    } else {
                        // Darkened crater floor basin
                        let factor = 1.0 - (1.0 - norm_d) * depth * 0.7;
                        base_r *= factor;
                        base_g *= factor;
                        base_b *= factor;
                    }
                }
            }

            let idx = (y * width + x) * 4;
            data[idx] = base_r.clamp(0.0, 255.0) as u8;
            data[idx + 1] = base_g.clamp(0.0, 255.0) as u8;
            data[idx + 2] = base_b.clamp(0.0, 255.0) as u8;
            data[idx + 3] = 255;
        }
    }

    Image::new_fill(
        bevy::render::render_resource::Extent3d {
            width: width as u32,
            height: height as u32,
            depth_or_array_layers: 1,
        },
        bevy::render::render_resource::TextureDimension::D2,
        &data,
        bevy::render::render_resource::TextureFormat::Rgba8UnormSrgb,
        bevy::asset::RenderAssetUsages::RENDER_WORLD | bevy::asset::RenderAssetUsages::MAIN_WORLD,
    )
}

fn generate_moon_ring_texture() -> Image {
    let width = 256;
    let height = 256;
    let center = 128.0f32;
    let mut data = vec![0u8; width * height * 4];

    for y in 0..height {
        for x in 0..width {
            let fx = x as f32 - center;
            let fy = y as f32 - center;
            let dist = (fx * fx + fy * fy).sqrt();

            let norm_d = dist / center;
            let mut alpha = 0.0f32;
            let mut r = 180.0f32;
            let mut g = 220.0f32;
            let mut b = 255.0f32;

            if (0.35..=0.92).contains(&norm_d) {
                // Concentric ring gaps & dense bands
                let band = (norm_d * 40.0).sin();
                if band > -0.2 {
                    alpha = (band * 0.5 + 0.5) * 0.85;
                    if (0.6..=0.75).contains(&norm_d) {
                        // Dense bright silver-cyan core ring band
                        r = 210.0;
                        g = 240.0;
                        b = 255.0;
                        alpha = 0.95;
                    }
                }
            }

            let idx = (y * width + x) * 4;
            data[idx] = r as u8;
            data[idx + 1] = g as u8;
            data[idx + 2] = b as u8;
            data[idx + 3] = (alpha * 255.0).clamp(0.0, 255.0) as u8;
        }
    }

    Image::new_fill(
        bevy::render::render_resource::Extent3d {
            width: width as u32,
            height: height as u32,
            depth_or_array_layers: 1,
        },
        bevy::render::render_resource::TextureDimension::D2,
        &data,
        bevy::render::render_resource::TextureFormat::Rgba8UnormSrgb,
        bevy::asset::RenderAssetUsages::RENDER_WORLD | bevy::asset::RenderAssetUsages::MAIN_WORLD,
    )
}

fn setup_sky_dome(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut sky_materials: ResMut<Assets<SkyMaterial>>,
    mut standard_materials: ResMut<Assets<StandardMaterial>>,
    mut images: ResMut<Assets<Image>>,
) {
    // Large sphere inside the far culling plane (1000m)
    let sphere_mesh = meshes.add(Sphere::new(750.0).mesh().ico(5).unwrap());

    let sky_material = sky_materials.add(SkyMaterial {
        color: Vec4::new(0.4, 0.6, 1.0, 1.0),
        time: 0.0,
        cloudiness: 0.0,
        is_alien: 0.0,
    });

    commands.spawn((
        Name::new("SkyDome"),
        SkyDome,
        Mesh3d(sphere_mesh),
        MeshMaterial3d(sky_material),
        Transform::from_scale(Vec3::splat(-1.0)), // Flip normals inwards
    ));

    // Spawn 3D Cratered Moon Body with Planetary Ring System
    let crater_image = generate_moon_crater_texture();
    let ring_image = generate_moon_ring_texture();

    let crater_handle = images.add(crater_image);
    let ring_handle = images.add(ring_image);

    let moon_sphere_mesh = meshes.add(Sphere::new(28.0).mesh().ico(5).unwrap());
    let moon_ring_mesh = meshes.add(Plane3d::default().mesh().size(120.0, 120.0));

    let moon_material = standard_materials.add(StandardMaterial {
        base_color_texture: Some(crater_handle),
        emissive: LinearRgba::from(Color::srgb(0.7, 0.8, 1.0)),
        perceptual_roughness: 0.85,
        metallic: 0.05,
        ..default()
    });

    let ring_material = standard_materials.add(StandardMaterial {
        base_color_texture: Some(ring_handle),
        emissive: LinearRgba::from(Color::srgb(0.6, 0.8, 1.2)),
        alpha_mode: AlphaMode::Blend,
        cull_mode: None, // Double-sided ring rendering
        perceptual_roughness: 0.2,
        metallic: 0.1,
        ..default()
    });

    commands
        .spawn((
            Name::new("MoonBody"),
            MoonBody,
            Mesh3d(moon_sphere_mesh),
            MeshMaterial3d(moon_material),
            Transform::from_xyz(0.0, 600.0, 0.0),
        ))
        .with_children(|parent| {
            parent.spawn((
                Name::new("MoonRing"),
                Mesh3d(moon_ring_mesh),
                MeshMaterial3d(ring_material),
                Transform::from_rotation(Quat::from_euler(EulerRot::XYZ, 0.65, 0.2, 0.4)),
            ));
        });
}

fn update_time(time: Res<Time>, mut time_of_day: ResMut<TimeOfDay>) {
    time_of_day.hour += time.delta_secs() * time_of_day.speed;
    if time_of_day.hour >= 24.0 {
        time_of_day.hour -= 24.0;
    }
}

fn update_light_shadows(
    time_of_day: Res<TimeOfDay>,
    mut lights: Query<(&mut DirectionalLight, Option<&Sun>, Option<&Moon>)>,
) {
    let hour = time_of_day.hour;
    let is_day = (6.0..18.0).contains(&hour);

    for (mut light, sun, moon) in lights.iter_mut() {
        if sun.is_some() {
            light.shadows_enabled = is_day;
        } else if moon.is_some() {
            light.shadows_enabled = !is_day;
        }
    }
}
fn update_sun(
    time_of_day: Res<TimeOfDay>,
    weather: Option<Res<super::weather::WeatherManager>>,
    sun_query: Option<
        Single<
            &mut Transform,
            (
                With<Sun>,
                Without<Moon>,
                Without<crate::player::camera::Player>,
            ),
        >,
    >,
    moon_query: Option<
        Single<
            &mut Transform,
            (
                With<Moon>,
                Without<Sun>,
                Without<crate::player::camera::Player>,
            ),
        >,
    >,
    mut light_query: Query<(&mut DirectionalLight, Option<&Sun>, Option<&Moon>)>,
    player_query: Option<
        Single<
            &Transform,
            (
                With<crate::player::camera::Player>,
                Without<Sun>,
            ),
        >,
    >,
    mut ambient_light: ResMut<GlobalAmbientLight>,
) {
    let angle = (time_of_day.hour / 24.0) * 2.0 * PI - PI / 2.0;

    let is_alien = if let Some(player_transform) = player_query {
        player_transform.translation.x >= 5000.0
    } else {
        false
    };

    if is_alien {
        ambient_light.color = Color::srgb(0.45, 0.35, 0.65);
        ambient_light.brightness = 3500.0;
    } else {
        let sun_height = angle.sin();
        if sun_height > 0.0 {
            ambient_light.color = Color::srgb(0.9, 0.95, 1.0);
            ambient_light.brightness = (120.0 + sun_height * 300.0).clamp(80.0, 420.0);
        } else {
            ambient_light.color = Color::srgb(0.35, 0.45, 0.65);
            ambient_light.brightness = 350.0;
        }
    }

    // Sun position
    if is_alien {
        if let Some(mut transform) = sun_query {
            let sun_dir1 = Vec3::new(0.6, 0.8, -0.4).normalize();
            transform.look_to(-sun_dir1, Vec3::Y);
        }
        if let Some(mut transform) = moon_query {
            let sun_dir2 = Vec3::new(-0.7, 0.65, 0.3).normalize();
            transform.look_to(-sun_dir2, Vec3::Y);
        }
    } else {
        if let Some(mut transform) = sun_query {
            transform.rotation = Quat::from_rotation_x(angle);
        }
        if let Some(mut transform) = moon_query {
            transform.rotation = Quat::from_rotation_x(angle + PI);
        }
    }

    // Light intensities based on height
    for (mut light, sun, moon) in light_query.iter_mut() {
        if is_alien {
            if sun.is_some() {
                // Primary Golden Sun on Alien Planet (always illuminated)
                light.illuminance = 45000.0;
                light.color = Color::srgb(1.0, 0.75, 0.4);
            } else if moon.is_some() {
                // Secondary Cyan Sun on Alien Planet (always illuminated)
                light.illuminance = 25000.0;
                light.color = Color::srgb(0.4, 0.85, 1.3);
            }
        } else {
            // Normal Dimension Sun/Moon Logic
            if sun.is_some() {
                let sun_height = angle.sin();
                let mut illuminance = (sun_height.max(-0.1) * 80000.0).clamp(0.0, 80000.0);

                // Dim light based on cloudiness (up to 85% dimming in heavy storms)
                if let Some(ref w) = weather {
                    illuminance *= 1.0 - w.cloudiness * 0.85;
                }
                light.illuminance = illuminance;

                // Dawn/Dusk tint
                if sun_height > -0.1 && sun_height < 0.2 {
                    let t = (sun_height + 0.1) / 0.3;
                    light.color = Color::srgb(1.0, 0.6 + 0.4 * t, 0.4 + 0.6 * t);
                } else {
                    light.color = Color::WHITE;
                }
            } else if moon.is_some() {
                let moon_height = (angle + PI).sin();
                light.illuminance = (moon_height.max(0.0) * 8000.0).clamp(0.0, 8000.0);
                light.color = Color::srgb(0.8, 0.85, 1.0);
            }
        }
    }
}

fn update_sky(
    time_of_day: Res<TimeOfDay>,
    weather: Option<Res<super::weather::WeatherManager>>,
    mut clear_color: ResMut<ClearColor>,
    player_query: Option<Single<&Transform, With<crate::player::camera::Player>>>,
) {
    let is_alien = if let Some(player_transform) = player_query {
        player_transform.translation.x >= 5000.0
    } else {
        false
    };

    let hour = time_of_day.hour;

    // Simple linear interpolation between key times
    let mut sky_color = if is_alien {
        // Deep Space Color
        Color::srgb(0.02, 0.01, 0.08)
    } else if !(5.0..=20.0).contains(&hour) {
        // Night
        Color::srgb(0.02, 0.02, 0.05)
    } else if hour < 7.0 {
        // Dawn
        let t = (hour - 5.0) / 2.0;
        Color::srgb(0.4 * t + 0.02, 0.3 * t + 0.02, 0.2 * t + 0.05)
    } else if hour < 18.0 {
        // Day
        Color::srgb(0.4, 0.6, 1.0)
    } else {
        // Dusk
        let t = (hour - 18.0) / 2.0;
        Color::srgb(
            0.4 * (1.0 - t) + 0.02,
            0.3 * (1.0 - t) + 0.02,
            0.1 * (1.0 - t) + 0.05,
        )
    };

    // Blend sky color towards dark storm grey based on cloudiness
    if !is_alien && let Some(ref w) = weather {
        let storm_color = LinearRgba::new(0.12, 0.14, 0.18, 1.0);
        let current_rgba = LinearRgba::from(sky_color);
        let blended = current_rgba * (1.0 - w.cloudiness) + storm_color * w.cloudiness;
        sky_color = Color::from(blended);
    }

    clear_color.0 = sky_color;
}

fn update_sky_dome(
    time: Res<Time>,
    weather: Option<Res<super::weather::WeatherManager>>,
    player_query: Option<Single<&Transform, With<crate::player::camera::Player>>>,
    sky_dome_query: Option<
        Single<&mut Transform, (With<SkyDome>, Without<crate::player::camera::Player>)>,
    >,
    mut sky_materials: ResMut<Assets<SkyMaterial>>,
    sky_dome_mesh_query: Option<Single<&MeshMaterial3d<SkyMaterial>, With<SkyDome>>>,
    time_of_day: Res<TimeOfDay>,
    moon_body_query: Option<
        Single<
            &mut Transform,
            (
                With<MoonBody>,
                Without<SkyDome>,
                Without<crate::player::camera::Player>,
            ),
        >,
    >,
) {
    // Center sky dome on player
    let Some(player_transform) = player_query else {
        return;
    };
    let Some(mut sky_dome_transform) = sky_dome_query else {
        return;
    };

    // Maintain inward-facing flip scale
    sky_dome_transform.translation = player_transform.translation;
    sky_dome_transform.scale = Vec3::splat(-1.0);

    // Orbit 3D Ringed Moon around player along celestial arc
    if let Some(mut moon_transform) = moon_body_query {
        let angle = (time_of_day.hour / 24.0) * 2.0 * PI - PI / 2.0;
        let moon_angle = angle + PI;
        let moon_dir = Vec3::new(0.0, moon_angle.sin(), moon_angle.cos()).normalize();

        moon_transform.translation = player_transform.translation + moon_dir * 600.0;
        moon_transform.rotation = Quat::from_rotation_y(time.elapsed_secs() * 0.03);
    }

    // Update uniforms
    let Some(mat_handle) = sky_dome_mesh_query else {
        return;
    };
    if let Some(mat) = sky_materials.get_mut(&mat_handle.0) {
        mat.time = time.elapsed_secs();

        let is_alien = if player_transform.translation.x >= 5000.0 {
            1.0
        } else {
            0.0
        };
        mat.is_alien = is_alien;

        let mut cloudiness = 0.0;
        if let Some(ref w) = weather {
            cloudiness = w.cloudiness;
        }
        mat.cloudiness = cloudiness;

        let hour = time_of_day.hour;
        let mut sky_color = if is_alien > 0.5 {
            Color::srgb(0.02, 0.01, 0.08)
        } else if !(5.0..=20.0).contains(&hour) {
            // Night
            Color::srgb(0.015, 0.015, 0.04)
        } else if hour < 7.0 {
            // Dawn
            let t = (hour - 5.0) / 2.0;
            Color::srgb(0.4 * t + 0.015, 0.3 * t + 0.015, 0.2 * t + 0.04)
        } else if hour < 18.0 {
            // Day
            Color::srgb(0.4, 0.6, 1.0)
        } else {
            // Dusk
            let t = (hour - 18.0) / 2.0;
            Color::srgb(
                0.4 * (1.0 - t) + 0.015,
                0.3 * (1.0 - t) + 0.015,
                0.1 * (1.0 - t) + 0.04,
            )
        };

        if is_alien <= 0.5
            && let Some(ref w) = weather
        {
            let storm_color = LinearRgba::new(0.12, 0.14, 0.18, 1.0);
            let current_rgba = LinearRgba::from(sky_color);
            let blended = current_rgba * (1.0 - w.cloudiness) + storm_color * w.cloudiness;
            sky_color = Color::from(blended);
        }

        let c = LinearRgba::from(sky_color);
        mat.color = Vec4::new(c.red, c.green, c.blue, c.alpha);
    }
}
