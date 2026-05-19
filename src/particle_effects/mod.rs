use bevy::prelude::*;
use bevy_hanabi::prelude::*;
use crate::player::combat::LaserHitEvent;

/// Particle effects plugin - manages particle emitters and effects
pub struct ParticleEffectsPlugin;

#[derive(Resource)]
pub struct LaserImpactEffect(pub Handle<EffectAsset>);

#[derive(Resource)]
pub struct ThrusterEffect(pub Handle<EffectAsset>);

impl Plugin for ParticleEffectsPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, (setup_laser_effect, setup_thruster_effect));
        app.add_systems(Update, spawn_laser_particles);
    }
}

fn setup_laser_effect(
    mut commands: Commands,
    mut effects: ResMut<Assets<EffectAsset>>,
) {
    let mut color_gradient = bevy_hanabi::Gradient::new();
    color_gradient.add_key(0.0, Vec4::new(1.0, 1.0, 1.0, 1.0)); // White hot
    color_gradient.add_key(0.2, Vec4::new(0.0, 1.0, 1.0, 1.0)); // Cyan glow
    color_gradient.add_key(1.0, Vec4::new(0.0, 0.5, 0.8, 0.0)); // Fade out

    let mut size_gradient = bevy_hanabi::Gradient::new();
    size_gradient.add_key(0.0, Vec3::splat(0.1));
    size_gradient.add_key(1.0, Vec3::splat(0.0));

    let mut module = Module::default();
    
    let init_pos = SetPositionSphereModifier {
        center: module.lit(Vec3::ZERO),
        radius: module.lit(0.05),
        dimension: ShapeDimension::Surface,
    };

    let init_vel = SetVelocitySphereModifier {
        center: module.lit(Vec3::ZERO),
        speed: module.lit(8.0),
    };

    // Diagnostics Scan
    // let _scan: bevy_hanabi::prelude::TYPE_SCAN = 1;
    let spawner = SpawnerSettings::once(60.0.into()).with_starts_active(true);

    let effect = EffectAsset::new(16384, spawner, module)
        .with_name("laser_impact")
        .init(init_pos)
        .init(init_vel)
        .render(ColorOverLifetimeModifier { gradient: color_gradient, ..default() })
        .render(SizeOverLifetimeModifier { gradient: size_gradient, screen_space_size: false });

    let handle = effects.add(effect);
    commands.insert_resource(LaserImpactEffect(handle));
}

fn spawn_laser_particles(
    mut commands: Commands,
    mut events: MessageReader<LaserHitEvent>,
    effect_res: Option<Res<LaserImpactEffect>>,
) {
    if let Some(effect) = effect_res {
        for event in events.read() {
            commands.spawn((
                ParticleEffect {
                    handle: effect.0.clone(),
                    ..default()
                },
                Transform::from_translation(event.position),
            ));
        }
    }
}

fn setup_thruster_effect(
    mut commands: Commands,
    mut effects: ResMut<Assets<EffectAsset>>,
) {
    let mut color_gradient = bevy_hanabi::Gradient::new();
    // Intense HDR Glowing cyan/blue plasma jet (super high-tech, matches the mech laser!)
    color_gradient.add_key(0.0, Vec4::new(3.0, 3.0, 3.0, 1.0)); // Intense HDR white core
    color_gradient.add_key(0.12, Vec4::new(0.0, 2.5, 3.0, 1.0)); // Glowing HDR vibrant cyan
    color_gradient.add_key(0.5, Vec4::new(0.0, 0.8, 3.0, 0.8)); // Glowing HDR electric blue
    color_gradient.add_key(1.0, Vec4::new(0.0, 0.0, 1.0, 0.0)); // Deep blue fade-out

    let mut size_gradient = bevy_hanabi::Gradient::new();
    size_gradient.add_key(0.0, Vec3::splat(0.35)); // Thick, intense jet nozzle flare
    size_gradient.add_key(0.4, Vec3::splat(0.18)); // Sizzling stream
    size_gradient.add_key(1.0, Vec3::splat(0.0));

    let mut module = Module::default();
    
    let init_pos = SetPositionSphereModifier {
        center: module.lit(Vec3::ZERO),
        radius: module.lit(0.02),
        dimension: ShapeDimension::Volume,
    };

    // Shoot downwards rapidly!
    let init_vel = SetAttributeModifier::new(Attribute::VELOCITY, module.lit(Vec3::new(0.0, -15.0, 0.0)));

    // Lifetime: slightly longer for a prominent plasma stream
    let init_lifetime = SetAttributeModifier::new(Attribute::LIFETIME, module.lit(0.22));

    // Continuous rate-based spawner - extremely dense for thick thruster jets!
    let spawner = SpawnerSettings::rate(1500.0.into()).with_starts_active(true);

    let effect = EffectAsset::new(32768, spawner, module)
        .with_name("mech_thruster")
        .init(init_pos)
        .init(init_vel)
        .init(init_lifetime)
        .render(ColorOverLifetimeModifier { gradient: color_gradient, ..default() })
        .render(SizeOverLifetimeModifier { gradient: size_gradient, screen_space_size: false });

    let handle = effects.add(effect);
    commands.insert_resource(ThrusterEffect(handle));
}
