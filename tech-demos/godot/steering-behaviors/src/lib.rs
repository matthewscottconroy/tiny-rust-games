//! Steering Behaviors GDExtension demo — autonomous movement AI from Rust.
//!
//! Demonstrates three classic steering behaviors:
//!
//! - **Seek** (behavior 0) — accelerate directly toward the mouse cursor.
//! - **Flee** (behavior 1) — accelerate directly away from the mouse cursor.
//! - **Wander** (behavior 2) — random walk with a smoothly jittering wander angle.
//!
//! The chosen behavior is selectable via the exported `behavior` field.
//! `physics_process()` computes the steering force each fixed tick and applies
//! it to `CharacterBody2D::move_and_slide()`.

use godot::classes::{CharacterBody2D, ICharacterBody2D, Label};
use godot::prelude::*;

// ─── Extension entry point ───────────────────────────────────────────────────

struct SteeringBehaviorsExt;

#[gdextension]
unsafe impl ExtensionLibrary for SteeringBehaviorsExt {}

// ─── Pure functions ───────────────────────────────────────────────────────────

/// Returns a normalised velocity vector pointing from `(ox, oy)` toward
/// `(tx, ty)` scaled to `speed`.  Returns `(0, 0)` when the agent is already
/// at the target.
///
/// # Examples
/// ```
/// let (vx, vy) = steering_behaviors::seek(0.0, 0.0, 3.0, 4.0, 5.0);
/// assert!((vx - 3.0).abs() < 1e-4);
/// assert!((vy - 4.0).abs() < 1e-4);
/// ```
pub fn seek(ox: f32, oy: f32, tx: f32, ty: f32, speed: f32) -> (f32, f32) {
    let dx = tx - ox;
    let dy = ty - oy;
    let len = (dx * dx + dy * dy).sqrt();
    if len < 1e-6 {
        (0.0, 0.0)
    } else {
        (dx / len * speed, dy / len * speed)
    }
}

/// Returns a normalised velocity vector pointing away from `(tx, ty)` scaled
/// to `speed`.
///
/// # Examples
/// ```
/// let (vx, vy) = steering_behaviors::flee(0.0, 0.0, 3.0, 4.0, 5.0);
/// assert!((vx - (-3.0)).abs() < 1e-4);
/// assert!((vy - (-4.0)).abs() < 1e-4);
/// ```
pub fn flee(ox: f32, oy: f32, tx: f32, ty: f32, speed: f32) -> (f32, f32) {
    seek(tx, ty, ox, oy, speed) // reversed
}

/// Clamps the magnitude of `(vx, vy)` to at most `max`, preserving direction.
///
/// # Examples
/// ```
/// let (x, y) = steering_behaviors::limit_magnitude(3.0, 4.0, 2.5);
/// let len = (x * x + y * y).sqrt();
/// assert!((len - 2.5).abs() < 1e-5);
///
/// // Already within max — unchanged.
/// let (x2, y2) = steering_behaviors::limit_magnitude(1.0, 0.0, 5.0);
/// assert!((x2 - 1.0).abs() < 1e-5);
/// ```
pub fn limit_magnitude(vx: f32, vy: f32, max: f32) -> (f32, f32) {
    let len = (vx * vx + vy * vy).sqrt();
    if len < 1e-9 || len <= max {
        (vx, vy)
    } else {
        (vx / len * max, vy / len * max)
    }
}

/// Returns a wander target offset from the agent's local forward axis.
///
/// Projects a circle of `radius` at `distance` in front of the agent (along
/// the positive-x local axis) and then picks a point on that circle
/// determined by `angle`.
///
/// # Examples
/// ```
/// let (x, y) = steering_behaviors::wander_target(0.0, 10.0, 30.0);
/// // x should be distance + 0 (cos 0 * radius) = 40
/// assert!((x - 40.0).abs() < 1e-4);
/// // y should be sin(0) * radius = 0
/// assert!(y.abs() < 1e-4);
/// ```
pub fn wander_target(angle: f32, radius: f32, distance: f32) -> (f32, f32) {
    let x = distance + angle.cos() * radius;
    let y = angle.sin() * radius;
    (x, y)
}

// ─── SteeringAgent node ───────────────────────────────────────────────────────

/// A `CharacterBody2D` that applies steering behaviors toward or from the
/// mouse cursor, with an optional wandering mode.
#[derive(GodotClass)]
#[class(base=CharacterBody2D)]
pub struct SteeringAgent {
    /// Active behavior: 0 = seek, 1 = flee, 2 = wander.
    #[export]
    behavior: i32,
    /// Maximum movement speed (pixels per second).
    #[export]
    max_speed: f32,
    /// Maximum steering force magnitude.
    #[export]
    max_force: f32,
    /// Current wander angle (radians); jitters each physics frame.
    wander_angle: f32,
    base: Base<CharacterBody2D>,
}

#[godot_api]
impl ICharacterBody2D for SteeringAgent {
    fn init(base: Base<CharacterBody2D>) -> Self {
        Self {
            behavior: 0,
            max_speed: 150.0,
            max_force: 300.0,
            wander_angle: 0.0,
            base,
        }
    }

    fn ready(&mut self) {
        godot_print!(
            "[SteeringAgent] Ready — behavior: {}, speed: {}",
            self.behavior,
            self.max_speed
        );
    }

    fn physics_process(&mut self, delta: f64) {
        let pos = self.base().get_global_position();
        let mouse = self.base().get_global_mouse_position();
        let speed = self.max_speed;
        let max_force = self.max_force;

        let (mut dvx, mut dvy) = match self.behavior {
            1 => flee(pos.x, pos.y, mouse.x, mouse.y, speed),
            2 => {
                // Jitter wander angle.
                self.wander_angle += (delta as f32) * 2.0;
                let (wx, wy) = wander_target(self.wander_angle, 40.0, 80.0);
                // wx/wy is in local space; use directly as world-offset direction.
                let len = (wx * wx + wy * wy).sqrt();
                if len > 1e-6 {
                    (wx / len * speed, wy / len * speed)
                } else {
                    (0.0, 0.0)
                }
            }
            _ => seek(pos.x, pos.y, mouse.x, mouse.y, speed),
        };

        // Limit to max_force.
        let (fx, fy) = limit_magnitude(dvx, dvy, max_force);
        dvx = fx;
        dvy = fy;

        // Blend with current velocity and apply.
        let current_vel = self.base().get_velocity();
        let new_vx = (current_vel.x + dvx * delta as f32).clamp(-speed, speed);
        let new_vy = (current_vel.y + dvy * delta as f32).clamp(-speed, speed);
        self.base_mut()
            .set_velocity(Vector2::new(new_vx, new_vy));
        self.base_mut().move_and_slide();

        // Update label.
        let label_name = match self.behavior {
            1 => "Flee",
            2 => "Wander",
            _ => "Seek",
        };
        if let Some(mut label) = self.base().try_get_node_as::<Label>("BehaviorLabel") {
            label.set_text(label_name);
        }
    }
}

// ─── Tests ────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    // seek

    #[test]
    fn seek_produces_correct_velocity() {
        let (vx, vy) = seek(0.0, 0.0, 3.0, 4.0, 5.0);
        assert!((vx - 3.0).abs() < 1e-4);
        assert!((vy - 4.0).abs() < 1e-4);
    }

    #[test]
    fn seek_zero_distance_returns_zero() {
        let (vx, vy) = seek(5.0, 5.0, 5.0, 5.0, 10.0);
        assert!(vx.abs() < 1e-6);
        assert!(vy.abs() < 1e-6);
    }

    #[test]
    fn seek_horizontal() {
        let (vx, vy) = seek(0.0, 0.0, 10.0, 0.0, 3.0);
        assert!((vx - 3.0).abs() < 1e-4);
        assert!(vy.abs() < 1e-4);
    }

    // flee

    #[test]
    fn flee_produces_opposite_direction() {
        let (vx, vy) = flee(0.0, 0.0, 3.0, 4.0, 5.0);
        assert!((vx - (-3.0)).abs() < 1e-4);
        assert!((vy - (-4.0)).abs() < 1e-4);
    }

    #[test]
    fn flee_speed_correct() {
        let (vx, vy) = flee(0.0, 0.0, 10.0, 0.0, 7.0);
        assert!((vx - (-7.0)).abs() < 1e-4);
        assert!(vy.abs() < 1e-4);
    }

    // limit_magnitude

    #[test]
    fn limit_magnitude_exceeds_max() {
        let (x, y) = limit_magnitude(3.0, 4.0, 2.5);
        let len = (x * x + y * y).sqrt();
        assert!((len - 2.5).abs() < 1e-5);
    }

    #[test]
    fn limit_magnitude_within_max_unchanged() {
        let (x, y) = limit_magnitude(1.0, 0.0, 5.0);
        assert!((x - 1.0).abs() < 1e-5);
        assert!(y.abs() < 1e-5);
    }

    #[test]
    fn limit_magnitude_zero_vector() {
        let (x, y) = limit_magnitude(0.0, 0.0, 5.0);
        assert!(x.abs() < 1e-9);
        assert!(y.abs() < 1e-9);
    }

    // wander_target

    #[test]
    fn wander_target_angle_zero() {
        let (x, y) = wander_target(0.0, 10.0, 30.0);
        assert!((x - 40.0).abs() < 1e-4);
        assert!(y.abs() < 1e-4);
    }

    #[test]
    fn wander_target_half_pi() {
        let (x, y) = wander_target(std::f32::consts::FRAC_PI_2, 10.0, 0.0);
        // distance=0, so x=cos(pi/2)*10 ≈ 0, y=sin(pi/2)*10 = 10
        assert!(x.abs() < 1e-4);
        assert!((y - 10.0).abs() < 1e-4);
    }
}
