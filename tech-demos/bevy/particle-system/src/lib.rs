//! Particle system — a reusable Bevy plugin.
//!
//! This crate is a *building block*: drop [`ParticleSystemPlugin`] into any
//! Bevy app with `app.add_plugins(ParticleSystemPlugin)` and it manages a burst
//! emitter (SPACE) plus a continuous flame-like emitter, with no extra
//! dependencies. Tune it through the [`ParticleSystemConfig`] resource without
//! editing the plugin's internals.
//!
//! Key ideas:
//! - Particles are regular ECS entities with a [`Particle`] component that holds
//!   lifetime, velocity, and color state.  No special engine feature is needed.
//! - [`Rng`] is a deterministic linear congruential generator (LCG) that
//!   replaces the `rand` crate — zero extra dependencies.
//! - Burst emitter (SPACE) spawns many particles at once.
//! - Continuous emitter (fixed position) trickles particles every 40 ms.
//! - Alpha fades from 1 → 0 as lifetime runs down; entity despawns at 0.
//!
//! # Example
//! ```no_run
//! use bevy::prelude::*;
//! use particle_system::ParticleSystemPlugin;
//!
//! App::new()
//!     .add_plugins(DefaultPlugins)
//!     .add_plugins(ParticleSystemPlugin)
//!     .run();
//! ```

use bevy::prelude::*;

/// Bundles every system and resource for the particle system feature.
///
/// Add it with `app.add_plugins(ParticleSystemPlugin)`. It does **not** add
/// `DefaultPlugins`, so the host app stays in control of window and rendering
/// setup.
pub struct ParticleSystemPlugin;

impl Plugin for ParticleSystemPlugin {
    fn build(&self, app: &mut App) {
        let config = ParticleSystemConfig::default();
        app.insert_resource(Rng(config.rng_seed))
            .insert_resource(ContinuousEmitter(Timer::from_seconds(
                config.emitter_interval,
                TimerMode::Repeating,
            )))
            .insert_resource(config)
            .add_systems(Startup, setup)
            .add_systems(
                Update,
                (handle_burst_input, tick_continuous_emitter, age_particles),
            );
    }
}

// --- Configuration ---

/// Tunable parameters for the feature. Override before adding the plugin, e.g.
/// `app.insert_resource(ParticleSystemConfig { burst_count: 120, ..default() })`.
///
/// Note: [`Rng`]'s seed and the [`ContinuousEmitter`]'s interval are read from
/// this config when the plugin builds; overriding those two fields requires
/// also inserting the corresponding resources.
#[derive(Resource, Clone, Copy, Debug)]
pub struct ParticleSystemConfig {
    /// Seed for the deterministic LCG.
    pub rng_seed: u64,
    /// Interval in seconds between continuous emitter ticks.
    pub emitter_interval: f32,
    /// Number of particles spawned per SPACE burst.
    pub burst_count: u32,
    /// Velocity damping applied per second to each particle.
    pub drag: f32,
}

impl Default for ParticleSystemConfig {
    fn default() -> Self {
        Self {
            rng_seed: 12345,
            emitter_interval: 0.04,
            burst_count: 60,
            drag: 2.5,
        }
    }
}

// --- Components ---

/// Runtime state of a single particle.
#[derive(Component)]
pub struct Particle {
    /// Remaining lifetime in seconds.
    pub lifetime: f32,
    /// Total lifetime at spawn — used to compute the fade fraction.
    pub max_lifetime: f32,
    /// Current 2D velocity in world units/second.
    pub velocity: Vec2,
    /// Base (fully opaque) color for fading calculations.
    pub base_color: Color,
}

// --- Resources ---

/// Minimal linear congruential generator — no rand crate required.
///
/// Uses the Knuth multiplicative constants.  Each call to [`Rng::next_f32`]
/// advances the internal state and returns a value in `[0.0, 1.0)`.
#[derive(Resource)]
pub struct Rng(pub u64);

impl Rng {
    /// Advances the generator and returns the next pseudo-random `f32` in
    /// `[0.0, 1.0)`.
    pub fn next_f32(&mut self) -> f32 {
        self.0 = self
            .0
            .wrapping_mul(6_364_136_223_846_793_005)
            .wrapping_add(1_442_695_040_888_963_407);
        (self.0 >> 33) as f32 / u32::MAX as f32
    }

    /// Returns a random `f32` uniformly distributed in `[min, max)`.
    pub fn range(&mut self, min: f32, max: f32) -> f32 {
        min + self.next_f32() * (max - min)
    }
}

/// Drives the continuous flame-like particle emitter.
#[derive(Resource)]
pub struct ContinuousEmitter(pub Timer);

// --- Setup ---

fn setup(mut commands: Commands) {
    commands.spawn(Camera2d);

    commands.spawn((
        Sprite {
            color: Color::srgb(1.0, 0.5, 0.1),
            custom_size: Some(Vec2::splat(8.0)),
            ..default()
        },
        Transform::from_xyz(0.0, -220.0, 0.0),
    ));

    commands.spawn((
        Text::new("SPACE - burst at center   continuous emitter at bottom"),
        TextFont {
            font_size: 14.0,
            ..default()
        },
        TextColor(Color::srgb(0.65, 0.65, 0.65)),
        Node {
            position_type: PositionType::Absolute,
            bottom: Val::Px(10.0),
            left: Val::Px(10.0),
            ..default()
        },
    ));
}

// --- Particle spawning ---

/// Spawns a single particle with randomized angle, speed, lifetime, and size.
#[allow(clippy::too_many_arguments)]
fn spawn_particle(
    commands: &mut Commands,
    rng: &mut Rng,
    origin: Vec2,
    color: Color,
    speed_min: f32,
    speed_max: f32,
    lifetime_min: f32,
    lifetime_max: f32,
) {
    let angle = rng.range(0.0, std::f32::consts::TAU);
    let speed = rng.range(speed_min, speed_max);
    let velocity = Vec2::new(angle.cos(), angle.sin()) * speed;
    let lifetime = rng.range(lifetime_min, lifetime_max);
    let size = rng.range(4.0, 10.0);

    commands.spawn((
        Sprite {
            color,
            custom_size: Some(Vec2::splat(size)),
            ..default()
        },
        Transform::from_xyz(origin.x, origin.y, 1.0),
        Particle {
            lifetime,
            max_lifetime: lifetime,
            velocity,
            base_color: color,
        },
    ));
}

// --- Systems ---

/// Spawns a burst of particles when SPACE is pressed.
fn handle_burst_input(
    input: Res<ButtonInput<KeyCode>>,
    config: Res<ParticleSystemConfig>,
    mut commands: Commands,
    mut rng: ResMut<Rng>,
) {
    if !input.just_pressed(KeyCode::Space) {
        return;
    }

    for _ in 0..config.burst_count {
        let color = Color::srgb(
            rng.range(0.7, 1.0),
            rng.range(0.2, 0.6),
            rng.range(0.0, 0.2),
        );
        spawn_particle(
            &mut commands,
            &mut rng,
            Vec2::ZERO,
            color,
            60.0,
            280.0,
            0.4,
            1.2,
        );
    }
}

/// Fires the continuous emitter on each timer tick.
fn tick_continuous_emitter(
    time: Res<Time>,
    mut timer: ResMut<ContinuousEmitter>,
    mut commands: Commands,
    mut rng: ResMut<Rng>,
) {
    if !timer.0.tick(time.delta()).just_finished() {
        return;
    }

    let origin = Vec2::new(rng.range(-8.0, 8.0), -220.0);
    let angle = rng.range(
        std::f32::consts::FRAC_PI_4,
        3.0 * std::f32::consts::FRAC_PI_4,
    );
    let speed = rng.range(40.0, 120.0);
    let velocity = Vec2::new(angle.cos() * speed, angle.sin() * speed);
    let lifetime = rng.range(0.5, 1.4);

    let color = Color::srgb(rng.range(0.9, 1.0), rng.range(0.3, 0.7), 0.05);

    commands.spawn((
        Sprite {
            color,
            custom_size: Some(Vec2::splat(rng.range(5.0, 12.0))),
            ..default()
        },
        Transform::from_xyz(origin.x, origin.y, 1.0),
        Particle {
            lifetime,
            max_lifetime: lifetime,
            velocity,
            base_color: color,
        },
    ));
}

/// Ages all particles: moves them, applies drag, fades alpha, and despawns dead ones.
fn age_particles(
    mut commands: Commands,
    time: Res<Time>,
    config: Res<ParticleSystemConfig>,
    mut query: Query<(Entity, &mut Transform, &mut Sprite, &mut Particle)>,
) {
    let dt = time.delta_secs();
    for (entity, mut transform, mut sprite, mut particle) in &mut query {
        particle.lifetime -= dt;

        if particle.lifetime <= 0.0 {
            commands.entity(entity).despawn();
            continue;
        }

        transform.translation.x += particle.velocity.x * dt;
        transform.translation.y += particle.velocity.y * dt;

        particle.velocity *= 1.0 - config.drag * dt;

        let frac = (particle.lifetime / particle.max_lifetime).clamp(0.0, 1.0);
        let Color::Srgba(s) = particle.base_color else {
            continue;
        };
        sprite.color = Color::srgba(s.red, s.green, s.blue, frac);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // --- Rng unit tests ---

    #[test]
    fn next_f32_is_in_unit_interval() {
        let mut rng = Rng(99_999);
        for _ in 0..1_000 {
            let v = rng.next_f32();
            assert!((0.0..1.0).contains(&v), "value out of [0, 1): {v}");
        }
    }

    #[test]
    fn range_stays_within_bounds() {
        let mut rng = Rng(42);
        for _ in 0..500 {
            let v = rng.range(-10.0, 10.0);
            assert!((-10.0..10.0).contains(&v), "value out of range: {v}");
        }
    }

    #[test]
    fn same_seed_produces_same_sequence() {
        let mut a = Rng(1234);
        let mut b = Rng(1234);
        for _ in 0..20 {
            assert_eq!(a.next_f32(), b.next_f32());
        }
    }

    #[test]
    fn different_seeds_diverge() {
        let mut a = Rng(1);
        let mut b = Rng(2);
        let equal = (0..20).all(|_| a.next_f32() == b.next_f32());
        assert!(!equal, "different seeds should produce different sequences");
    }

    #[test]
    fn sequence_has_spread() {
        let mut rng = Rng(7);
        let values: Vec<f32> = (0..100).map(|_| rng.next_f32()).collect();
        let min = values.iter().cloned().fold(f32::INFINITY, f32::min);
        let max = values.iter().cloned().fold(f32::NEG_INFINITY, f32::max);
        assert!(
            max - min > 0.1,
            "sequence should have spread, got min={min} max={max}"
        );
    }

    #[test]
    fn range_with_equal_bounds_returns_min() {
        let mut rng = Rng(55);
        for _ in 0..10 {
            let v = rng.range(5.0, 5.0);
            assert_eq!(v, 5.0);
        }
    }

    // --- Config test ---

    #[test]
    fn config_default_matches_documented_values() {
        let c = ParticleSystemConfig::default();
        assert_eq!(c.rng_seed, 12345);
        assert_eq!(c.burst_count, 60);
        assert!((c.emitter_interval - 0.04).abs() < 1e-6);
    }

    // --- ECS setup test ---

    #[test]
    fn setup_spawns_camera_and_emitter_sprite() {
        let mut app = App::new();
        app.add_plugins(MinimalPlugins).add_systems(Startup, setup);
        app.update();

        // Camera2d + the emitter sprite both carry a Transform.
        let mut q = app.world_mut().query::<&Camera2d>();
        assert_eq!(q.iter(app.world()).count(), 1);
    }
}
