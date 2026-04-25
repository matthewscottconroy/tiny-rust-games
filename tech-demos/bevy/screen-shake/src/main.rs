//! Screen-shake demo — trauma/decay pattern.
//!
//! Key ideas:
//! - A `Trauma` resource holds a value in `[0.0, 1.0]`.  Intensity is squared
//!   (`trauma²`) so high trauma feels violent while low trauma feels gentle.
//! - Two orthogonal sine waves at incommensurable frequencies produce an
//!   organic, non-repeating shake rather than a regular oscillation.
//! - Trauma decays by `TRAUMA_DECAY` units per second; it never goes negative.
//!
//! **Controls:**
//! - **SPACE** — add trauma (additive, clamped to 1.0)
//! - **R** — reset trauma to zero immediately

use bevy::prelude::*;

/// Units of maximum camera offset applied at full trauma.
const MAX_OFFSET: f32 = 40.0;
/// Maximum roll in radians applied at full trauma.
const MAX_ROLL: f32 = 0.08;
/// Trauma units lost per second.
const TRAUMA_DECAY: f32 = 1.2;
/// Trauma added per SPACE press.
const TRAUMA_ADD: f32 = 0.4;

fn main() {
    App::new()
        .add_plugins(DefaultPlugins.set(WindowPlugin {
            primary_window: Some(Window {
                title: "Screen Shake — SPACE to shake, R to reset".to_string(),
                resolution: (800.0, 500.0).into(),
                ..default()
            }),
            ..default()
        }))
        .init_resource::<Trauma>()
        .add_systems(Startup, setup)
        .add_systems(Update, (handle_input, decay_trauma, apply_shake).chain())
        .run();
}

/// Holds the current trauma level in `[0.0, 1.0]`.
#[derive(Resource, Default)]
struct Trauma(f32);

/// Marker for the camera that receives the shake transform.
#[derive(Component)]
struct ShakeCamera;

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
                Sprite { color, custom_size: Some(Vec2::splat(72.0)), ..default() },
                Transform::from_translation(Vec3::new(col as f32 * 80.0, row as f32 * 80.0, 0.0)),
            ));
        }
    }

    commands.spawn((
        Text::new("SPACE — add trauma\nR — reset"),
        TextFont { font_size: 22.0, ..default() },
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
fn handle_input(input: Res<ButtonInput<KeyCode>>, mut trauma: ResMut<Trauma>) {
    if input.just_pressed(KeyCode::Space) {
        trauma.0 = (trauma.0 + TRAUMA_ADD).min(1.0);
    }
    if input.just_pressed(KeyCode::KeyR) {
        trauma.0 = 0.0;
    }
}

/// Reduces trauma toward zero over time.
fn decay_trauma(time: Res<Time>, mut trauma: ResMut<Trauma>) {
    trauma.0 = (trauma.0 - TRAUMA_DECAY * time.delta_secs()).max(0.0);
}

/// Offsets and rolls the camera proportional to `trauma²`.
fn apply_shake(
    trauma: Res<Trauma>,
    time: Res<Time>,
    mut query: Query<&mut Transform, With<ShakeCamera>>,
) {
    let Ok(mut transform) = query.get_single_mut() else { return };
    let (offset, roll) = shake_offset(trauma.0, time.elapsed_secs());
    transform.translation.x = offset.x;
    transform.translation.y = offset.y;
    transform.rotation = Quat::from_rotation_z(roll);
}

/// Returns `(translation_offset, roll_radians)` for a given trauma and time.
///
/// Uses quadratic intensity and two sine waves at prime-ratio frequencies so
/// the pattern never exactly repeats on a human-perceptible timescale.
pub fn shake_offset(trauma: f32, time_secs: f32) -> (Vec2, f32) {
    let intensity = trauma * trauma;
    let x = MAX_OFFSET * intensity * (time_secs * 37.0).sin();
    let y = MAX_OFFSET * intensity * (time_secs * 51.0).sin();
    let roll = MAX_ROLL * intensity * (time_secs * 29.0).sin();
    (Vec2::new(x, y), roll)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn zero_trauma_produces_no_offset() {
        let (offset, roll) = shake_offset(0.0, 1.0);
        assert_eq!(offset, Vec2::ZERO);
        assert_eq!(roll, 0.0);
    }

    #[test]
    fn full_trauma_offset_bounded_by_max_offset() {
        // Sample many time points — the magnitude must never exceed MAX_OFFSET.
        for i in 0..100 {
            let t = i as f32 * 0.1;
            let (offset, _) = shake_offset(1.0, t);
            assert!(offset.x.abs() <= MAX_OFFSET + 1e-5, "x={} exceeds MAX_OFFSET", offset.x);
            assert!(offset.y.abs() <= MAX_OFFSET + 1e-5, "y={} exceeds MAX_OFFSET", offset.y);
        }
    }

    #[test]
    fn full_trauma_roll_bounded_by_max_roll() {
        for i in 0..100 {
            let t = i as f32 * 0.1;
            let (_, roll) = shake_offset(1.0, t);
            assert!(roll.abs() <= MAX_ROLL + 1e-5, "roll={} exceeds MAX_ROLL", roll);
        }
    }

    #[test]
    fn half_trauma_intensity_is_quarter_of_full() {
        // At t=0 all sines are 0, so check at a non-zero time where sin≈1.
        // Use π/2 / 37 so that sin(t*37) ≈ 1.
        let t = std::f32::consts::FRAC_PI_2 / 37.0;
        let (full, _) = shake_offset(1.0, t);
        let (half, _) = shake_offset(0.5, t);
        // half trauma → 0.25 intensity → offset should be ~0.25 of full
        let ratio = half.x / full.x;
        assert!((ratio - 0.25).abs() < 1e-5, "ratio={}", ratio);
    }

    #[test]
    fn trauma_decay_constants_are_positive() {
        assert!(TRAUMA_DECAY > 0.0);
        assert!(TRAUMA_ADD > 0.0);
        assert!(MAX_OFFSET > 0.0);
        assert!(MAX_ROLL > 0.0);
    }

    #[test]
    fn trauma_add_is_at_most_one() {
        assert!(TRAUMA_ADD <= 1.0);
    }
}
