//! Rigid Body 2D GDExtension demo — impulse-based physics movement from Rust.
//!
//! Teaches:
//!
//! - Subclassing `RigidBody2D` and implementing `IRigidBody2D::physics_process`.
//! - Reading `Input::singleton()` for WASD directional input each physics frame.
//! - Applying forces via `apply_central_impulse` — the engine owns the velocity.
//! - Clamping velocity magnitude to enforce a max speed.
//! - Displaying the current speed on a `Label` child node.
//! - Exposing a `apply_explosion_impulse` func for external triggering.

use godot::classes::{IRigidBody2D, Input, Label, RigidBody2D};
use godot::prelude::*;

// ─── Extension entry point ───────────────────────────────────────────────────

struct RigidBody2DExt;

#[gdextension]
unsafe impl ExtensionLibrary for RigidBody2DExt {}

// ─── Pure functions ───────────────────────────────────────────────────────────

/// Clamps a 2D velocity vector so its magnitude does not exceed `max`.
///
/// Returns the clamped `(vx, vy)` components.
///
/// # Examples
/// ```
/// let (cx, cy) = rigid_body_2d::clamp_velocity(300.0, 400.0, 500.0);
/// let mag = (cx * cx + cy * cy).sqrt();
/// assert!(mag <= 500.0 + 1e-3);
///
/// // Already within limit — unchanged.
/// let (cx, cy) = rigid_body_2d::clamp_velocity(100.0, 0.0, 500.0);
/// assert!((cx - 100.0).abs() < 1e-4);
/// ```
pub fn clamp_velocity(vx: f32, vy: f32, max: f32) -> (f32, f32) {
    let mag_sq = vx * vx + vy * vy;
    if mag_sq <= max * max {
        return (vx, vy);
    }
    let mag = mag_sq.sqrt();
    let scale = max / mag;
    (vx * scale, vy * scale)
}

/// Returns a human-readable speed label string.
///
/// # Examples
/// ```
/// assert_eq!(rigid_body_2d::speed_label(123.4), "Speed: 123");
/// assert_eq!(rigid_body_2d::speed_label(0.0), "Speed: 0");
/// ```
pub fn speed_label(speed: f32) -> String {
    format!("Speed: {}", speed as i32)
}

/// Computes the impulse vector from boolean WASD inputs.
///
/// Returns `(fx, fy)` as a scaled direction vector.  Multiple keys can be
/// pressed simultaneously; the resulting direction is NOT normalised (raw sum).
///
/// # Examples
/// ```
/// // Only right pressed.
/// let (fx, fy) = rigid_body_2d::impulse_from_input(false, false, false, true, 300.0);
/// assert!((fx - 300.0).abs() < 1e-4);
/// assert!((fy).abs() < 1e-4);
///
/// // Up and right together.
/// let (fx, fy) = rigid_body_2d::impulse_from_input(true, false, false, true, 100.0);
/// assert!((fx - 100.0).abs() < 1e-4);
/// assert!((fy - (-100.0)).abs() < 1e-4);
///
/// // No input.
/// let (fx, fy) = rigid_body_2d::impulse_from_input(false, false, false, false, 300.0);
/// assert_eq!(fx, 0.0);
/// assert_eq!(fy, 0.0);
/// ```
pub fn impulse_from_input(
    up: bool,
    down: bool,
    left: bool,
    right: bool,
    strength: f32,
) -> (f32, f32) {
    let mut fx = 0.0f32;
    let mut fy = 0.0f32;
    if up {
        fy -= strength;
    }
    if down {
        fy += strength;
    }
    if left {
        fx -= strength;
    }
    if right {
        fx += strength;
    }
    (fx, fy)
}

// ─── PhysicsBody node ─────────────────────────────────────────────────────────

/// A `RigidBody2D` subclass controlled by WASD impulses each physics frame.
///
/// The Godot physics engine owns the velocity — this class only applies impulses
/// and clamps the magnitude.  Add a `CollisionShape2D` child and a `Label` child
/// named `"Label"` in the Godot editor before running.
#[derive(GodotClass)]
#[class(base=RigidBody2D)]
pub struct PhysicsBody {
    /// Force applied per physics frame when a direction key is held.
    #[export]
    impulse_strength: f32,
    /// Maximum allowed linear speed in pixels per second.
    #[export]
    max_speed: f32,
    base: Base<RigidBody2D>,
}

#[godot_api]
impl IRigidBody2D for PhysicsBody {
    fn init(base: Base<RigidBody2D>) -> Self {
        Self {
            impulse_strength: 300.0,
            max_speed: 400.0,
            base,
        }
    }

    fn ready(&mut self) {
        self.base_mut().set_gravity_scale(1.0);
        godot_print!(
            "[PhysicsBody] Ready — impulse_strength={}, max_speed={}",
            self.impulse_strength,
            self.max_speed
        );
    }

    fn physics_process(&mut self, delta: f64) {
        let input = Input::singleton();

        let up = input.is_action_pressed("ui_up");
        let down = input.is_action_pressed("ui_down");
        let left = input.is_action_pressed("ui_left");
        let right = input.is_action_pressed("ui_right");

        let strength = self.impulse_strength;
        let (fx, fy) = impulse_from_input(up, down, left, right, strength * delta as f32);

        if fx != 0.0 || fy != 0.0 {
            self.base_mut()
                .apply_central_impulse_ex()
                .impulse(Vector2::new(fx, fy))
                .done();
        }

        // Clamp velocity magnitude.
        let vel = self.base().get_linear_velocity();
        let max = self.max_speed;
        let (cx, cy) = clamp_velocity(vel.x, vel.y, max);
        self.base_mut().set_linear_velocity(Vector2::new(cx, cy));

        // Update speed label.
        let speed = (cx * cx + cy * cy).sqrt();
        let text = speed_label(speed);
        if let Some(mut label) = self.base_mut().try_get_node_as::<Label>("Label") {
            label.set_text(text.as_str());
        }
    }
}

#[godot_api]
impl PhysicsBody {
    /// Applies a large upward impulse to simulate an explosion knocking the body.
    #[func]
    pub fn apply_explosion_impulse(&mut self) {
        self.base_mut()
            .apply_central_impulse_ex()
            .impulse(Vector2::new(0.0, -500.0))
            .done();
        godot_print!("[PhysicsBody] Explosion impulse applied!");
    }
}

// ─── Tests ────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    // clamp_velocity ──────────────────────────────────────────────────────────

    #[test]
    fn clamp_velocity_within_limit_unchanged() {
        let (cx, cy) = clamp_velocity(100.0, 0.0, 500.0);
        assert!((cx - 100.0).abs() < 1e-4);
        assert!(cy.abs() < 1e-4);
    }

    #[test]
    fn clamp_velocity_exceeds_limit_scaled_down() {
        let (cx, cy) = clamp_velocity(300.0, 400.0, 500.0);
        let mag = (cx * cx + cy * cy).sqrt();
        assert!((mag - 500.0).abs() < 1e-2, "mag={}", mag);
    }

    #[test]
    fn clamp_velocity_exactly_at_limit_unchanged() {
        let (cx, cy) = clamp_velocity(500.0, 0.0, 500.0);
        assert!((cx - 500.0).abs() < 1e-4);
        assert!(cy.abs() < 1e-4);
    }

    #[test]
    fn clamp_velocity_zero_vector_unchanged() {
        let (cx, cy) = clamp_velocity(0.0, 0.0, 500.0);
        assert_eq!(cx, 0.0);
        assert_eq!(cy, 0.0);
    }

    #[test]
    fn clamp_velocity_preserves_direction() {
        let (cx, cy) = clamp_velocity(600.0, 0.0, 300.0);
        assert!((cx - 300.0).abs() < 1e-3);
        assert!(cy.abs() < 1e-4);
    }

    // speed_label ─────────────────────────────────────────────────────────────

    #[test]
    fn speed_label_zero() {
        assert_eq!(speed_label(0.0), "Speed: 0");
    }

    #[test]
    fn speed_label_truncates_decimal() {
        assert_eq!(speed_label(123.9), "Speed: 123");
    }

    #[test]
    fn speed_label_large_value() {
        assert_eq!(speed_label(400.0), "Speed: 400");
    }

    // impulse_from_input ──────────────────────────────────────────────────────

    #[test]
    fn impulse_no_input_is_zero() {
        let (fx, fy) = impulse_from_input(false, false, false, false, 300.0);
        assert_eq!(fx, 0.0);
        assert_eq!(fy, 0.0);
    }

    #[test]
    fn impulse_right_only() {
        let (fx, fy) = impulse_from_input(false, false, false, true, 300.0);
        assert!((fx - 300.0).abs() < 1e-4);
        assert!(fy.abs() < 1e-4);
    }

    #[test]
    fn impulse_up_only() {
        let (fx, fy) = impulse_from_input(true, false, false, false, 300.0);
        assert!(fx.abs() < 1e-4);
        assert!((fy - (-300.0)).abs() < 1e-4);
    }

    #[test]
    fn impulse_left_and_down_combine() {
        let (fx, fy) = impulse_from_input(false, true, true, false, 100.0);
        assert!((fx - (-100.0)).abs() < 1e-4);
        assert!((fy - 100.0).abs() < 1e-4);
    }

    #[test]
    fn impulse_up_and_right_combine() {
        let (fx, fy) = impulse_from_input(true, false, false, true, 100.0);
        assert!((fx - 100.0).abs() < 1e-4);
        assert!((fy - (-100.0)).abs() < 1e-4);
    }

    #[test]
    fn impulse_opposite_directions_cancel() {
        let (fx, fy) = impulse_from_input(true, true, false, false, 200.0);
        assert!(fx.abs() < 1e-4);
        assert!(fy.abs() < 1e-4);
    }
}
