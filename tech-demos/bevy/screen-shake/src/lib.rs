//! Screen-shake — a reusable Bevy plugin using the trauma/decay pattern.
//!
//! This crate is a *building block*: drop [`ScreenShakePlugin`] into any Bevy
//! app with `app.add_plugins(ScreenShakePlugin)` and it manages a shakeable
//! camera. Tune it through the [`ScreenShakeConfig`] resource without editing
//! the plugin's internals.
//!
//! Key ideas:
//! - A [`Trauma`] resource holds a value in `[0.0, 1.0]`. Intensity is squared
//!   (`trauma²`) so high trauma feels violent while low trauma feels gentle.
//! - Two orthogonal sine waves at incommensurable frequencies produce an
//!   organic, non-repeating shake rather than a regular oscillation.
//! - Trauma decays by `trauma_decay` units per second; it never goes negative.
//!
//! **Controls:**
//! - **SPACE** — add trauma (additive, clamped to 1.0)
//! - **R** — reset trauma to zero immediately
//!
//! # Example
//! ```no_run
//! use bevy::prelude::*;
//! use screen_shake::ScreenShakePlugin;
//!
//! App::new()
//!     .add_plugins(DefaultPlugins)
//!     .add_plugins(ScreenShakePlugin)
//!     .run();
//! ```

use bevy::prelude::*;

/// Bundles every system and resource for the screen-shake feature.
///
/// Add it with `app.add_plugins(ScreenShakePlugin)`. It does **not** add
/// `DefaultPlugins`, so the host app stays in control of window and rendering
/// setup.
pub struct ScreenShakePlugin;

impl Plugin for ScreenShakePlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<ScreenShakeConfig>()
            .init_resource::<Trauma>()
            .add_systems(Startup, setup)
            .add_systems(Update, (handle_input, decay_trauma, apply_shake).chain());
    }
}

// --- Configuration ---

/// Tunable parameters for the shake. Override before adding the plugin, e.g.
/// `app.insert_resource(ScreenShakeConfig { max_offset: 80.0, ..default() })`.
#[derive(Resource, Clone, Copy, Debug)]
pub struct ScreenShakeConfig {
    /// Units of maximum camera offset applied at full trauma.
    pub max_offset: f32,
    /// Maximum roll in radians applied at full trauma.
    pub max_roll: f32,
    /// Trauma units lost per second.
    pub trauma_decay: f32,
    /// Trauma added per SPACE press.
    pub trauma_add: f32,
}

impl Default for ScreenShakeConfig {
    fn default() -> Self {
        Self {
            max_offset: 40.0,
            max_roll: 0.08,
            trauma_decay: 1.2,
            trauma_add: 0.4,
        }
    }
}

/// Holds the current trauma level in `[0.0, 1.0]`.
#[derive(Resource, Default)]
pub struct Trauma(pub f32);

/// Marker for the camera that receives the shake transform.
#[derive(Component)]
pub struct ShakeCamera;

/// Spawns the camera and a reference rectangle so the shake is visible.
fn setup(mut commands: Commands) {
    commands.spawn((Camera2d, ShakeCamera));

    // A grid of small squares as a reference background.
    for row in -3..=3_i32 {
        for col in -5..=5_i32 {
            let color = if (row + col) % 2 == 0 {
                Color::srgb(0.25, 0.25, 0.35)
            } else {
                Color::srgb(0.15, 0.15, 0.22)
            };
            commands.spawn((
                Sprite {
                    color,
                    custom_size: Some(Vec2::splat(72.0)),
                    ..default()
                },
                Transform::from_translation(Vec3::new(col as f32 * 80.0, row as f32 * 80.0, 0.0)),
            ));
        }
    }

    commands.spawn((
        Text::new("SPACE - add trauma\nR - reset"),
        TextFont {
            font_size: 22.0,
            ..default()
        },
        TextColor(Color::WHITE),
        Node {
            position_type: PositionType::Absolute,
            bottom: Val::Px(16.0),
            left: Val::Px(16.0),
            ..default()
        },
    ));
}

/// Increases trauma on SPACE; resets on R.
fn handle_input(
    input: Res<ButtonInput<KeyCode>>,
    config: Res<ScreenShakeConfig>,
    mut trauma: ResMut<Trauma>,
) {
    if input.just_pressed(KeyCode::Space) {
        trauma.0 = (trauma.0 + config.trauma_add).min(1.0);
    }
    if input.just_pressed(KeyCode::KeyR) {
        trauma.0 = 0.0;
    }
}

/// Reduces trauma toward zero over time.
fn decay_trauma(time: Res<Time>, config: Res<ScreenShakeConfig>, mut trauma: ResMut<Trauma>) {
    trauma.0 = (trauma.0 - config.trauma_decay * time.delta_secs()).max(0.0);
}

/// Offsets and rolls the camera proportional to `trauma²`.
fn apply_shake(
    trauma: Res<Trauma>,
    time: Res<Time>,
    config: Res<ScreenShakeConfig>,
    mut query: Query<&mut Transform, With<ShakeCamera>>,
) {
    let Ok(mut transform) = query.single_mut() else {
        return;
    };
    let (offset, roll) = shake_offset(
        trauma.0,
        time.elapsed_secs(),
        config.max_offset,
        config.max_roll,
    );
    transform.translation.x = offset.x;
    transform.translation.y = offset.y;
    transform.rotation = Quat::from_rotation_z(roll);
}

/// Returns `(translation_offset, roll_radians)` for a given trauma and time.
///
/// Uses quadratic intensity and two sine waves at prime-ratio frequencies so
/// the pattern never exactly repeats on a human-perceptible timescale.
pub fn shake_offset(trauma: f32, time_secs: f32, max_offset: f32, max_roll: f32) -> (Vec2, f32) {
    let intensity = trauma * trauma;
    let x = max_offset * intensity * (time_secs * 37.0).sin();
    let y = max_offset * intensity * (time_secs * 51.0).sin();
    let roll = max_roll * intensity * (time_secs * 29.0).sin();
    (Vec2::new(x, y), roll)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn cfg() -> ScreenShakeConfig {
        ScreenShakeConfig::default()
    }

    #[test]
    fn zero_trauma_produces_no_offset() {
        let c = cfg();
        let (offset, roll) = shake_offset(0.0, 1.0, c.max_offset, c.max_roll);
        assert_eq!(offset, Vec2::ZERO);
        assert_eq!(roll, 0.0);
    }

    #[test]
    fn full_trauma_offset_bounded_by_max_offset() {
        let c = cfg();
        // Sample many time points — the magnitude must never exceed max_offset.
        for i in 0..100 {
            let t = i as f32 * 0.1;
            let (offset, _) = shake_offset(1.0, t, c.max_offset, c.max_roll);
            assert!(
                offset.x.abs() <= c.max_offset + 1e-5,
                "x={} exceeds max_offset",
                offset.x
            );
            assert!(
                offset.y.abs() <= c.max_offset + 1e-5,
                "y={} exceeds max_offset",
                offset.y
            );
        }
    }

    #[test]
    fn full_trauma_roll_bounded_by_max_roll() {
        let c = cfg();
        for i in 0..100 {
            let t = i as f32 * 0.1;
            let (_, roll) = shake_offset(1.0, t, c.max_offset, c.max_roll);
            assert!(
                roll.abs() <= c.max_roll + 1e-5,
                "roll={} exceeds max_roll",
                roll
            );
        }
    }

    #[test]
    fn half_trauma_intensity_is_quarter_of_full() {
        let c = cfg();
        // At t=0 all sines are 0, so check at a non-zero time where sin≈1.
        // Use π/2 / 37 so that sin(t*37) ≈ 1.
        let t = std::f32::consts::FRAC_PI_2 / 37.0;
        let (full, _) = shake_offset(1.0, t, c.max_offset, c.max_roll);
        let (half, _) = shake_offset(0.5, t, c.max_offset, c.max_roll);
        // half trauma → 0.25 intensity → offset should be ~0.25 of full
        let ratio = half.x / full.x;
        assert!((ratio - 0.25).abs() < 1e-5, "ratio={}", ratio);
    }

    #[test]
    fn config_default_constants_are_positive() {
        let c = cfg();
        assert!(c.trauma_decay > 0.0);
        assert!(c.trauma_add > 0.0);
        assert!(c.max_offset > 0.0);
        assert!(c.max_roll > 0.0);
    }

    #[test]
    fn config_default_trauma_add_is_at_most_one() {
        assert!(cfg().trauma_add <= 1.0);
    }

    #[test]
    fn config_default_matches_documented_values() {
        let c = cfg();
        assert_eq!(c.max_offset, 40.0);
        assert_eq!(c.max_roll, 0.08);
        assert_eq!(c.trauma_decay, 1.2);
        assert_eq!(c.trauma_add, 0.4);
    }
}
