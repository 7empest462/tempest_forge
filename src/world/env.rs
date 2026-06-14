use bevy::prelude::*;
use std::f32::consts::PI;

pub struct EnvironmentPlugin;

impl Plugin for EnvironmentPlugin {
    fn build(&self, app: &mut App) {
        app.insert_resource(TimeOfDay::default())
            .add_systems(Update, update_time)
            .add_systems(Update, update_sun)
            .add_systems(Update, update_sky);
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

fn update_time(time: Res<Time>, mut time_of_day: ResMut<TimeOfDay>) {
    time_of_day.hour += time.delta_secs() * time_of_day.speed;
    if time_of_day.hour >= 24.0 {
        time_of_day.hour -= 24.0;
    }
}

fn update_sun(
    time_of_day: Res<TimeOfDay>,
    mut sun_query: Query<&mut Transform, (With<Sun>, Without<Moon>)>,
    mut moon_query: Query<&mut Transform, (With<Moon>, Without<Sun>)>,
    mut light_query: Query<(&mut DirectionalLight, Option<&Sun>, Option<&Moon>)>,
) {
    let angle = (time_of_day.hour / 24.0) * 2.0 * PI - PI / 2.0;

    // Sun position
    if let Ok(mut transform) = sun_query.single_mut() {
        transform.rotation = Quat::from_rotation_x(angle);
    }

    // Moon position (opposite of sun)
    if let Ok(mut transform) = moon_query.single_mut() {
        transform.rotation = Quat::from_rotation_x(angle + PI);
    }

    // Light intensities based on height
    for (mut light, sun, moon) in light_query.iter_mut() {
        if sun.is_some() {
            let sun_height = angle.sin();
            light.illuminance = (sun_height.max(-0.1) * 80000.0).clamp(0.0, 80000.0);

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

fn update_sky(time_of_day: Res<TimeOfDay>, mut clear_color: ResMut<ClearColor>) {
    let hour = time_of_day.hour;

    // Simple linear interpolation between key times
    let sky_color = if !(5.0..=20.0).contains(&hour) {
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

    clear_color.0 = sky_color;
}
