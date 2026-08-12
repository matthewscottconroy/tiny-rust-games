//! Camera Follow demo — a Camera2D that smoothly lerps toward the first node
//! in the "player" group, respecting a configurable dead zone.
//!
//! Teaches: a `Camera2D` that lerps toward a target found by group lookup, with a dead
//! zone so small movements do not jitter the view.

use godot::classes::{Camera2D, ICamera2D};
use godot::prelude::*;

struct CameraFollowExtension;
#[gdextension]
unsafe impl ExtensionLibrary for CameraFollowExtension {}

#[derive(GodotClass)]
#[class(base=Camera2D)]
struct CameraFollow {
    #[export]
    follow_speed: f32,
    #[export]
    dead_zone: f32,

    base: Base<Camera2D>,
}

#[godot_api]
impl ICamera2D for CameraFollow {
    fn init(base: Base<Camera2D>) -> Self {
        Self {
            follow_speed: 5.0,
            dead_zone: 20.0,
            base,
        }
    }

    fn process(&mut self, delta: f64) {
        let players = self.base().get_tree().get_nodes_in_group("player");
        if players.is_empty() {
            return;
        }

        // Get player position
        let player_pos = match players.get(0) {
            Some(node) => {
                if let Ok(node2d) = node.try_cast::<Node2D>() {
                    node2d.get_position()
                } else {
                    return;
                }
            }
            None => return,
        };

        let cam_pos = self.base().get_position();
        let dist = distance(cam_pos.x, cam_pos.y, player_pos.x, player_pos.y);

        if !in_dead_zone(dist, self.dead_zone) {
            let speed = self.follow_speed;
            let new_x = smooth_follow(cam_pos.x, player_pos.x, speed, delta as f32);
            let new_y = smooth_follow(cam_pos.y, player_pos.y, speed, delta as f32);
            self.base_mut().set_position(Vector2::new(new_x, new_y));
        }
    }
}

// ---------------------------------------------------------------------------
// Pure helper functions
// ---------------------------------------------------------------------------

/// Exponential smoothing: moves `from` toward `to` at rate `speed` per second.
pub fn smooth_follow(from: f32, to: f32, speed: f32, delta: f32) -> f32 {
    from + (to - from) * (1.0 - (-speed * delta).exp())
}

/// Euclidean distance between two 2D points.
pub fn distance(ax: f32, ay: f32, bx: f32, by: f32) -> f32 {
    let dx = bx - ax;
    let dy = by - ay;
    (dx * dx + dy * dy).sqrt()
}

/// Returns true when `dist` is within the dead zone (no movement needed).
pub fn in_dead_zone(dist: f32, dz: f32) -> bool {
    dist <= dz
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn distance_zero_same_point() {
        assert!((distance(3.0, 4.0, 3.0, 4.0)).abs() < 1e-6);
    }

    #[test]
    fn distance_pythagorean() {
        assert!((distance(0.0, 0.0, 3.0, 4.0) - 5.0).abs() < 1e-5);
    }

    #[test]
    fn distance_symmetric() {
        assert!((distance(1.0, 2.0, 5.0, 6.0) - distance(5.0, 6.0, 1.0, 2.0)).abs() < 1e-6);
    }

    #[test]
    fn in_dead_zone_exactly_at_boundary() {
        assert!(in_dead_zone(20.0, 20.0));
    }

    #[test]
    fn in_dead_zone_inside() {
        assert!(in_dead_zone(5.0, 20.0));
    }

    #[test]
    fn in_dead_zone_outside() {
        assert!(!in_dead_zone(21.0, 20.0));
    }

    #[test]
    fn smooth_follow_moves_toward_target() {
        let result = smooth_follow(0.0, 100.0, 5.0, 0.1);
        assert!(result > 0.0 && result < 100.0);
    }

    #[test]
    fn smooth_follow_already_at_target() {
        let result = smooth_follow(50.0, 50.0, 5.0, 0.1);
        assert!((result - 50.0).abs() < 1e-5);
    }

    #[test]
    fn smooth_follow_large_delta_approaches_target() {
        let result = smooth_follow(0.0, 100.0, 5.0, 10.0);
        assert!(
            result > 99.0,
            "should be near target with large delta: {result}"
        );
    }
}
