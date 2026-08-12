//! Bullet-pattern (danmaku) — a reusable Bevy plugin.
//!
//! This crate is a *building block*: add [`BulletPatternPlugin`] to any Bevy app
//! with `app.add_plugins(BulletPatternPlugin)` and it spawns a bullet emitter
//! that cycles through three danmaku patterns. Tune it through the
//! [`BulletPatternConfig`] resource without editing the plugin's internals.
//!
//! Key ideas:
//! - [`radial_directions`] produces N evenly-spaced unit vectors for a burst.
//! - [`spiral_angle`] advances an angle by a fixed rate each frame for a spiral.
//! - [`aimed_direction`] points toward a target position.
//! - Three emitter patterns — radial burst, continuous spiral, aimed — switch
//!   with 1/2/3 keys.
//! - Bullets are simple `Sprite` entities with a [`Velocity`] component and a
//!   lifetime countdown. Despawning happens in `tick_bullets`.
//!
//! **Controls:** 1/2/3 — change pattern; bullets auto-fire.
//!
//! # Example
//! ```no_run
//! use bevy::prelude::*;
//! use bullet_pattern::BulletPatternPlugin;
//!
//! App::new()
//!     .add_plugins(DefaultPlugins)
//!     .add_plugins(BulletPatternPlugin)
//!     .run();
//! ```

use bevy::prelude::*;
use std::f32::consts::TAU;

/// Bundles every system and resource for the bullet-pattern feature.
///
/// Add it with `app.add_plugins(BulletPatternPlugin)`. It does **not** add
/// `DefaultPlugins`, so the host app stays in control of window and rendering
/// setup.
pub struct BulletPatternPlugin;

impl Plugin for BulletPatternPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<BulletPatternConfig>()
            .init_resource::<Emitter>()
            .add_systems(Startup, setup)
            .add_systems(Update, (handle_input, fire_bullets, tick_bullets).chain());
    }
}

// --- Configuration ---

/// Tunable parameters for the feature. Override before adding the plugin, e.g.
/// `app.insert_resource(BulletPatternConfig { bullet_speed: 400.0, ..default() })`.
#[derive(Resource, Clone, Copy, Debug)]
pub struct BulletPatternConfig {
    /// Bullet travel speed in world units per second.
    pub bullet_speed: f32,
    /// How long (seconds) a bullet lives before despawning.
    pub max_lifetime: f32,
    /// Number of bullets in a radial burst.
    pub radial_count: usize,
    /// Seconds between bursts.
    pub fire_rate: f32,
    /// Radians the spiral advances per burst.
    pub spiral_rate: f32,
}

impl Default for BulletPatternConfig {
    fn default() -> Self {
        Self {
            bullet_speed: 220.0,
            max_lifetime: 3.0,
            radial_count: 12,
            fire_rate: 0.18,
            spiral_rate: 0.22,
        }
    }
}

// ── Pure pattern functions ────────────────────────────────────────────────────

/// Returns `count` evenly-spaced unit directions starting at `offset_angle`.
pub fn radial_directions(count: usize, offset_angle: f32) -> Vec<Vec2> {
    (0..count)
        .map(|i| {
            let a = offset_angle + i as f32 * TAU / count as f32;
            Vec2::new(a.cos(), a.sin())
        })
        .collect()
}

/// Advances `current_angle` by `rate` radians and wraps it to `[0, TAU)`.
pub fn spiral_angle(current_angle: f32, rate: f32) -> f32 {
    (current_angle + rate).rem_euclid(TAU)
}

/// Returns the unit vector from `from` pointing at `to`.
/// Falls back to `Vec2::Y` when the points are coincident.
pub fn aimed_direction(from: Vec2, to: Vec2) -> Vec2 {
    let d = to - from;
    if d.length_squared() < 1e-6 {
        Vec2::Y
    } else {
        d.normalize()
    }
}

/// Maps bullet lifetime `[0, max]` to a colour: bright white → dim orange.
pub fn bullet_color(lifetime: f32, max_lifetime: f32) -> Color {
    let t = (lifetime / max_lifetime).clamp(0.0, 1.0);
    Color::srgb(1.0, 0.5 + 0.5 * t, t)
}

// ── ECS ───────────────────────────────────────────────────────────────────────

/// Which firing pattern the emitter is currently using.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Pattern {
    /// Bullets fired evenly in all directions at once.
    Radial,
    /// A rotating arm of bullets, sweeping as it fires.
    Spiral,
    /// A tight spread fired toward the player.
    Aimed,
}

/// The single bullet emitter: current pattern, fire cooldown, and spiral phase.
#[derive(Resource)]
pub struct Emitter {
    /// The active firing pattern.
    pub pattern: Pattern,
    /// Countdown until the next burst (seconds).
    pub fire_timer: f32,
    /// Current spiral angle (radians).
    pub spiral_angle: f32,
}

impl Default for Emitter {
    fn default() -> Self {
        Self {
            pattern: Pattern::Radial,
            fire_timer: 0.0,
            spiral_angle: 0.0,
        }
    }
}

/// A live bullet with a remaining lifetime in seconds.
#[derive(Component)]
pub struct Bullet {
    /// Remaining lifetime in seconds before despawn.
    pub lifetime: f32,
}

/// 2D linear velocity component.
#[derive(Component)]
pub struct Velocity(pub Vec2);

#[derive(Component)]
struct PatternLabel;

fn setup(mut commands: Commands) {
    commands.spawn(Camera2d);

    commands.spawn((
        Text::new("1: radial   2: spiral   3: aimed"),
        TextFont {
            font_size: 15.0,
            ..default()
        },
        TextColor(Color::srgb(0.55, 0.55, 0.55)),
        Node {
            position_type: PositionType::Absolute,
            top: Val::Px(8.0),
            left: Val::Px(10.0),
            ..default()
        },
    ));

    commands.spawn((
        Text::new("Pattern: Radial"),
        TextFont {
            font_size: 16.0,
            ..default()
        },
        TextColor(Color::srgb(1.0, 0.75, 0.2)),
        PatternLabel,
        Node {
            position_type: PositionType::Absolute,
            top: Val::Px(28.0),
            left: Val::Px(10.0),
            ..default()
        },
    ));

    // Emitter visual
    commands.spawn((
        Sprite {
            color: Color::srgb(0.3, 0.9, 0.5),
            custom_size: Some(Vec2::splat(16.0)),
            ..default()
        },
        Transform::from_xyz(0.0, 0.0, 1.0),
    ));
}

fn handle_input(
    keys: Res<ButtonInput<KeyCode>>,
    mut emitter: ResMut<Emitter>,
    mut label: Query<&mut Text, With<PatternLabel>>,
) {
    if keys.just_pressed(KeyCode::Digit1) {
        emitter.pattern = Pattern::Radial;
    }
    if keys.just_pressed(KeyCode::Digit2) {
        emitter.pattern = Pattern::Spiral;
    }
    if keys.just_pressed(KeyCode::Digit3) {
        emitter.pattern = Pattern::Aimed;
    }

    if emitter.is_changed()
        && let Ok(mut t) = label.single_mut()
    {
        *t = Text::new(format!("Pattern: {:?}", emitter.pattern));
    }
}

fn fire_bullets(
    time: Res<Time>,
    config: Res<BulletPatternConfig>,
    mut emitter: ResMut<Emitter>,
    windows: Query<&Window>,
    mut commands: Commands,
) {
    emitter.fire_timer -= time.delta_secs();
    if emitter.fire_timer > 0.0 {
        return;
    }
    emitter.fire_timer = config.fire_rate;

    let target: Vec2 = windows
        .single()
        .ok()
        .and_then(|w| w.cursor_position())
        .map(|c| Vec2::new(c.x - 400.0, -(c.y - 300.0)))
        .unwrap_or(Vec2::new(200.0, 0.0));

    let dirs: Vec<Vec2> = match emitter.pattern {
        Pattern::Radial => radial_directions(config.radial_count, 0.0),
        Pattern::Spiral => {
            emitter.spiral_angle = spiral_angle(emitter.spiral_angle, config.spiral_rate);
            radial_directions(3, emitter.spiral_angle)
        }
        Pattern::Aimed => vec![aimed_direction(Vec2::ZERO, target)],
    };

    for dir in dirs {
        commands.spawn((
            Sprite {
                color: Color::srgb(1.0, 1.0, 0.8),
                custom_size: Some(Vec2::splat(6.0)),
                ..default()
            },
            Transform::from_xyz(0.0, 0.0, 0.0),
            Bullet {
                lifetime: config.max_lifetime,
            },
            Velocity(dir * config.bullet_speed),
        ));
    }
}

fn tick_bullets(
    time: Res<Time>,
    config: Res<BulletPatternConfig>,
    mut commands: Commands,
    mut bullets: Query<(Entity, &mut Bullet, &mut Transform, &mut Sprite, &Velocity)>,
) {
    let dt = time.delta_secs();
    for (entity, mut bullet, mut tf, mut sprite, vel) in &mut bullets {
        bullet.lifetime -= dt;
        if bullet.lifetime <= 0.0 {
            commands.entity(entity).despawn();
            continue;
        }
        tf.translation += vel.0.extend(0.0) * dt;
        sprite.color = bullet_color(bullet.lifetime, config.max_lifetime);
    }
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn radial_directions_correct_count() {
        assert_eq!(radial_directions(8, 0.0).len(), 8);
    }

    #[test]
    fn radial_directions_are_unit_length() {
        for d in radial_directions(12, 0.0) {
            assert!((d.length() - 1.0).abs() < 1e-5, "not unit: {d}");
        }
    }

    #[test]
    fn radial_directions_evenly_spaced() {
        let dirs = radial_directions(4, 0.0);
        // Adjacent angles should differ by TAU/4
        let a0 = dirs[0].y.atan2(dirs[0].x);
        let a1 = dirs[1].y.atan2(dirs[1].x);
        assert!((a1 - a0 - TAU / 4.0).abs() < 1e-5);
    }

    #[test]
    fn radial_directions_offset_angle_applied() {
        let dirs = radial_directions(1, std::f32::consts::FRAC_PI_2);
        assert!((dirs[0].x).abs() < 1e-5);
        assert!((dirs[0].y - 1.0).abs() < 1e-5);
    }

    #[test]
    fn spiral_angle_advances() {
        let a = spiral_angle(0.0, 0.1);
        assert!((a - 0.1).abs() < 1e-5);
    }

    #[test]
    fn spiral_angle_wraps_at_tau() {
        let a = spiral_angle(TAU - 0.05, 0.1);
        assert!(a < 0.1, "should wrap: {a}");
    }

    #[test]
    fn aimed_direction_points_at_target() {
        let dir = aimed_direction(Vec2::ZERO, Vec2::new(1.0, 0.0));
        assert!((dir.x - 1.0).abs() < 1e-5 && dir.y.abs() < 1e-5);
    }

    #[test]
    fn aimed_direction_is_unit() {
        let dir = aimed_direction(Vec2::new(3.0, 4.0), Vec2::new(-1.0, 2.0));
        assert!((dir.length() - 1.0).abs() < 1e-5);
    }

    #[test]
    fn aimed_direction_coincident_fallback_is_up() {
        let dir = aimed_direction(Vec2::ZERO, Vec2::ZERO);
        assert_eq!(dir, Vec2::Y);
    }

    #[test]
    fn bullet_color_full_lifetime_has_white_tint() {
        let c = bullet_color(3.0, 3.0);
        assert!((c.to_linear().red - 1.0).abs() < 1e-3);
    }

    #[test]
    fn bullet_color_zero_lifetime_is_dimmer() {
        let c0 = bullet_color(0.0, 3.0);
        let c1 = bullet_color(3.0, 3.0);
        assert!(c0.to_linear().blue < c1.to_linear().blue);
    }

    #[test]
    fn config_default_matches_documented_values() {
        let c = BulletPatternConfig::default();
        assert_eq!(c.bullet_speed, 220.0);
        assert_eq!(c.radial_count, 12);
    }

    #[test]
    fn emitter_default_starts_radial() {
        let e = Emitter::default();
        assert_eq!(e.pattern, Pattern::Radial);
    }
}
