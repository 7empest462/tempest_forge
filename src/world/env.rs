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

fn setup_sky_dome(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut sky_materials: ResMut<Assets<SkyMaterial>>,
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
            ambient_light.color = Color::srgb(0.2, 0.3, 0.5);
            ambient_light.brightness = 40.0;
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
                light.illuminance = (moon_height.max(0.0) * 1000.0).clamp(0.0, 1000.0);
                light.color = Color::srgb(0.8, 0.8, 1.0);
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
