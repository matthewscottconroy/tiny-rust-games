//! one-shot-collision: Demonstrates `move_and_collide` with `KinematicCollision2D`
//! inspection in Rust. The body bounces around the screen, reflecting off surfaces.

use godot::classes::{CharacterBody2D, ICharacterBody2D};
use godot::prelude::*;

struct OneShotCollisionExtension;
#[gdextension]
unsafe impl ExtensionLibrary for OneShotCollisionExtension {}

#[derive(GodotClass)]
#[class(base=CharacterBody2D)]
struct OneShotCollision {
    /// Initial movement speed (pixels per second).
    #[export]
    speed: f32,
    /// Downward gravitational acceleration (pixels per second²).
    #[export]
    gravity: f32,

    // Velocity stored as plain floats.
    vel_x: f32,
    vel_y: f32,
    bounce_count: i32,

    base: Base<CharacterBody2D>,
}

#[godot_api]
impl ICharacterBody2D for OneShotCollision {
    fn init(base: Base<CharacterBody2D>) -> Self {
        Self {
            speed: 200.0,
            gravity: 400.0,
            vel_x: 150.0, // initial diagonal launch
            vel_y: -200.0,
            bounce_count: 0,
            base,
        }
    }

    fn ready(&mut self) {
        godot_print!(
            "OneShotCollision ready (speed={}, gravity={})",
            self.speed,
            self.gravity
        );
    }

    fn physics_process(&mut self, delta: f64) {
        let dt = delta as f32;

        // Apply gravity
        self.vel_y = apply_gravity(self.vel_y, self.gravity, dt);

        let motion = Vector2::new(self.vel_x * dt, self.vel_y * dt);

        // Resolve collision: extract the normal while the borrow is active,
        // then drop the collision before mutating self fields.
        let collision_normal: Option<(f32, f32)> = {
            let result = self.base_mut().move_and_collide(motion);
            result.map(|c| {
                let n = c.get_normal();
                (n.x, n.y)
            })
        };
        if let Some((nx, ny)) = collision_normal {
            let (rvx, rvy) = reflect_velocity(self.vel_x, self.vel_y, nx, ny);
            self.vel_x = rvx;
            self.vel_y = rvy;
            self.bounce_count += 1;
            godot_print!(
                "Bounce #{} off normal ({:.2}, {:.2})",
                self.bounce_count,
                nx,
                ny
            );
        }
    }
}

#[godot_api]
impl OneShotCollision {
    /// Returns the total number of collisions / bounces so far.
    #[func]
    pub fn get_bounce_count(&self) -> i32 {
        self.bounce_count
    }
}

// ---------------------------------------------------------------------------
// Pure helpers
// ---------------------------------------------------------------------------

/// Reflect velocity (vx, vy) off a surface with normal (nx, ny).
/// Uses the formula: v' = v - 2 (v · n) n
pub fn reflect_velocity(vx: f32, vy: f32, nx: f32, ny: f32) -> (f32, f32) {
    let dot = vx * nx + vy * ny;
    (vx - 2.0 * dot * nx, vy - 2.0 * dot * ny)
}

/// Increase vertical velocity by gravity over `delta` seconds.
pub fn apply_gravity(vy: f32, gravity: f32, delta: f32) -> f32 {
    vy + gravity * delta
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reflect_downward_off_floor() {
        // Velocity pointing down, normal pointing up
        let (rvx, rvy) = reflect_velocity(0.0, 100.0, 0.0, -1.0);
        assert!(rvx.abs() < 0.001);
        assert!((rvy + 100.0).abs() < 0.001); // flipped
    }

    #[test]
    fn reflect_rightward_off_left_wall() {
        let (rvx, rvy) = reflect_velocity(100.0, 0.0, -1.0, 0.0);
        assert!((rvx + 100.0).abs() < 0.001);
        assert!(rvy.abs() < 0.001);
    }

    #[test]
    fn reflect_diagonal() {
        let (rvx, rvy) = reflect_velocity(1.0, 1.0, 0.0, -1.0);
        assert!((rvx - 1.0).abs() < 0.001);
        assert!((rvy + 1.0).abs() < 0.001);
    }

    #[test]
    fn gravity_increases_velocity() {
        let vy = apply_gravity(0.0, 400.0, 0.016);
        assert!(vy > 0.0);
        assert!((vy - 6.4).abs() < 0.01);
    }

    #[test]
    fn gravity_accumulates() {
        let vy1 = apply_gravity(0.0, 400.0, 0.5);
        let vy2 = apply_gravity(vy1, 400.0, 0.5);
        assert!((vy2 - 400.0).abs() < 0.01);
    }

    #[test]
    fn reflect_preserves_speed_on_flat_floor() {
        let speed_before = (50.0f32 * 50.0 + 100.0 * 100.0).sqrt();
        let (rvx, rvy) = reflect_velocity(50.0, 100.0, 0.0, -1.0);
        let speed_after = (rvx * rvx + rvy * rvy).sqrt();
        assert!((speed_before - speed_after).abs() < 0.001);
    }

    #[test]
    fn gravity_zero_delta_no_change() {
        let vy = apply_gravity(50.0, 400.0, 0.0);
        assert_eq!(vy, 50.0);
    }
}
