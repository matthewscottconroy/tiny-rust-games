//! Player Movement GDExtension demo — CharacterBody2D with WASD/arrow-key movement,
//! gravity, and jumping.
//!
//! Teaches: `physics_process`, `get_velocity`/`set_velocity`, `move_and_slide`,
//! `is_on_floor`, exported tunable properties, and extracting pure functions for
//! testability.

use godot::classes::{CharacterBody2D, ICharacterBody2D, Input};
use godot::prelude::*;

// ─── Extension entry point ────────────────────────────────────────────────────

/// Declares this crate as a GDExtension library.
struct PlayerMovementExt;

#[gdextension]
unsafe impl ExtensionLibrary for PlayerMovementExt {}

// ─── Player node ──────────────────────────────────────────────────────────────

/// A `CharacterBody2D` subclass with WASD/arrow-key movement, gravity, and jumping.
///
/// Add a `CollisionShape2D` child and assign a shape in the Godot editor, then
/// run the scene to control the player with the arrow keys or WASD. `Space` or
/// the gamepad South button triggers a jump.
///
/// All movement parameters are exported so they can be tuned in the inspector
/// without recompiling.
#[derive(GodotClass)]
#[class(base=CharacterBody2D)]
pub struct Player {
    /// Horizontal movement speed in pixels per second.
    #[export]
    speed: f64,
    /// Downward acceleration applied each frame when the player is airborne.
    #[export]
    gravity: f64,
    /// Initial vertical velocity applied when the player jumps (negative = up).
    #[export]
    jump_velocity: f64,
    base: Base<CharacterBody2D>,
}

#[godot_api]
impl ICharacterBody2D for Player {
    fn init(base: Base<CharacterBody2D>) -> Self {
        Self {
            speed: 200.0,
            gravity: 980.0,
            jump_velocity: -400.0,
            base,
        }
    }

    fn ready(&mut self) {
        godot_print!("Player ready — speed={}, gravity={}, jump_velocity={}",
            self.speed, self.gravity, self.jump_velocity);
    }

    fn physics_process(&mut self, delta: f64) {
        let input = Input::singleton();
        let mut vel = self.base().get_velocity();
        let on_floor = self.base().is_on_floor();

        // Gravity
        const TERMINAL_VELOCITY: f32 = 1200.0;
        vel.y = apply_gravity(vel.y, self.gravity as f32, delta as f32, TERMINAL_VELOCITY);

        // Jump
        if on_floor && input.is_action_just_pressed("ui_accept") {
            vel.y = self.jump_velocity as f32;
        }

        // Horizontal movement
        let dir = input.get_axis("ui_left", "ui_right");
        vel.x = compute_x_velocity(dir, self.speed as f32);

        self.base_mut().set_velocity(vel);
        self.base_mut().move_and_slide();
    }
}

#[godot_api]
impl Player {
    /// Returns `true` if the player's horizontal speed exceeds 1.0 px/s.
    #[func]
    pub fn is_moving(&self) -> bool {
        let vel = self.base().get_velocity();
        let fraction = get_speed_fraction(vel.x, self.speed as f32);
        is_moving_threshold(fraction as f64, 1.0 / self.speed)
    }

    /// Returns the player's current horizontal speed as a fraction of max speed,
    /// clamped to `[0.0, 1.0]`.
    #[func]
    pub fn get_speed_fraction(&self) -> f64 {
        let vel = self.base().get_velocity();
        get_speed_fraction(vel.x, self.speed as f32) as f64
    }
}

// ─── Pure functions ───────────────────────────────────────────────────────────

/// Applies gravity to a y-velocity: returns `vel_y + gravity * delta`, clamped
/// to `terminal` so the body never falls faster than terminal velocity.
pub fn apply_gravity(vel_y: f32, gravity: f32, delta: f32, terminal: f32) -> f32 {
    (vel_y + gravity * delta).min(terminal)
}

/// Returns the new x-velocity given a direction input in `[-1, 1]` and a target
/// speed. Passing `0.0` as direction stops horizontal movement immediately.
pub fn compute_x_velocity(direction: f32, speed: f32) -> f32 {
    direction * speed
}

/// Returns `true` if `speed_fraction` (|vel_x| / max_speed) exceeds `threshold`.
pub fn is_moving_threshold(speed_fraction: f64, threshold: f64) -> bool {
    speed_fraction > threshold
}

/// Returns |vel_x| / speed clamped to `[0.0, 1.0]`.
pub fn get_speed_fraction(vel_x: f32, speed: f32) -> f32 {
    if speed == 0.0 {
        return 0.0;
    }
    (vel_x.abs() / speed).clamp(0.0, 1.0)
}

// ─── Tests ────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    // apply_gravity ────────────────────────────────────────────────────────────

    #[test]
    fn apply_gravity_increases_downward_velocity() {
        let result = apply_gravity(0.0, 980.0, 0.016, 1200.0);
        assert!(result > 0.0, "gravity should increase vel_y");
        assert!((result - 980.0_f32 * 0.016_f32).abs() < 1e-3);
    }

    #[test]
    fn apply_gravity_clamps_to_terminal_velocity() {
        let result = apply_gravity(1190.0, 980.0, 1.0, 1200.0);
        assert_eq!(result, 1200.0, "should be clamped to terminal velocity");
    }

    #[test]
    fn apply_gravity_already_at_terminal() {
        let result = apply_gravity(1200.0, 980.0, 0.016, 1200.0);
        assert_eq!(result, 1200.0);
    }

    #[test]
    fn apply_gravity_large_delta_still_clamped() {
        let result = apply_gravity(0.0, 980.0, 100.0, 1200.0);
        assert_eq!(result, 1200.0);
    }

    // compute_x_velocity ───────────────────────────────────────────────────────

    #[test]
    fn compute_x_velocity_full_right() {
        assert_eq!(compute_x_velocity(1.0, 200.0), 200.0);
    }

    #[test]
    fn compute_x_velocity_full_left() {
        assert_eq!(compute_x_velocity(-1.0, 200.0), -200.0);
    }

    #[test]
    fn compute_x_velocity_no_input_stops() {
        assert_eq!(compute_x_velocity(0.0, 200.0), 0.0);
    }

    #[test]
    fn compute_x_velocity_partial_direction() {
        let result = compute_x_velocity(0.5, 200.0);
        assert_eq!(result, 100.0);
    }

    // is_moving_threshold ──────────────────────────────────────────────────────

    #[test]
    fn is_moving_threshold_above_threshold_is_true() {
        assert!(is_moving_threshold(0.1, 0.005));
    }

    #[test]
    fn is_moving_threshold_at_zero_is_false() {
        assert!(!is_moving_threshold(0.0, 0.005));
    }

    #[test]
    fn is_moving_threshold_exactly_at_threshold_is_false() {
        assert!(!is_moving_threshold(0.005, 0.005));
    }

    // get_speed_fraction ───────────────────────────────────────────────────────

    #[test]
    fn get_speed_fraction_full_speed() {
        assert_eq!(get_speed_fraction(200.0, 200.0), 1.0);
    }

    #[test]
    fn get_speed_fraction_half_speed() {
        assert!((get_speed_fraction(100.0, 200.0) - 0.5).abs() < 1e-6);
    }

    #[test]
    fn get_speed_fraction_over_max_clamped_to_one() {
        assert_eq!(get_speed_fraction(300.0, 200.0), 1.0);
    }

    #[test]
    fn get_speed_fraction_negative_vel_uses_abs() {
        assert!((get_speed_fraction(-100.0, 200.0) - 0.5).abs() < 1e-6);
    }

    #[test]
    fn get_speed_fraction_zero_speed_guard() {
        assert_eq!(get_speed_fraction(100.0, 0.0), 0.0);
    }

    #[test]
    fn get_speed_fraction_zero_vel() {
        assert_eq!(get_speed_fraction(0.0, 200.0), 0.0);
    }
}
