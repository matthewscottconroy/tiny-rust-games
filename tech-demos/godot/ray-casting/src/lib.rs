//! ray-casting: Demonstrates Godot 2D physics ray-casting from Rust.
//! Draws a line from the node origin toward the mouse cursor, stopping at the
//! first physics body hit.

use godot::classes::{INode2D, Node2D, PhysicsRayQueryParameters2D};
use godot::prelude::*;

struct RayCastingExtension;
#[gdextension]
unsafe impl ExtensionLibrary for RayCastingExtension {}

#[derive(GodotClass)]
#[class(base=Node2D)]
struct RayCasting {
    /// Maximum ray length in pixels when nothing is hit.
    #[export]
    ray_length: f32,

    // Last frame results stored as plain floats (Option emulated with sentinel).
    hit: bool,
    hit_x: f32,
    hit_y: f32,
    hit_nx: f32,
    hit_ny: f32,

    base: Base<Node2D>,
}

#[godot_api]
impl INode2D for RayCasting {
    fn init(base: Base<Node2D>) -> Self {
        Self {
            ray_length: 300.0,
            hit: false,
            hit_x: 0.0,
            hit_y: 0.0,
            hit_nx: 0.0,
            hit_ny: 0.0,
            base,
        }
    }

    fn ready(&mut self) {
        godot_print!("RayCasting ready (ray_length={})", self.ray_length);
    }

    fn process(&mut self, _delta: f64) {
        // Get mouse position in viewport coordinates.
        let mouse_pos = self
            .base()
            .get_viewport()
            .expect("viewport missing")
            .get_mouse_position();

        let origin = self.base().get_global_position();
        let (dx, dy) = normalize_2d(mouse_pos.x - origin.x, mouse_pos.y - origin.y);
        let (ex, ey) = ray_endpoint(origin.x, origin.y, dx, dy, self.ray_length);
        let end = Vector2::new(ex, ey);

        // Build ray query
        let mut params = PhysicsRayQueryParameters2D::new_gd();
        params.set_from(origin);
        params.set_to(end);

        let space_state = self
            .base()
            .get_world_2d()
            .expect("world_2d missing")
            .get_direct_space_state()
            .expect("space state missing");

        let result = space_state.clone().intersect_ray(&params);
        if result.is_empty() {
            self.hit = false;
        } else {
            self.hit = true;
            let pos: Vector2 = result
                .get("position")
                .map(|v| v.to::<Vector2>())
                .unwrap_or(end);
            let normal: Vector2 = result
                .get("normal")
                .map(|v| v.to::<Vector2>())
                .unwrap_or(Vector2::new(0.0, -1.0));
            self.hit_x = pos.x;
            self.hit_y = pos.y;
            self.hit_nx = normal.x;
            self.hit_ny = normal.y;
        }

        self.base_mut().queue_redraw();
    }

    fn draw(&mut self) {
        let origin = Vector2::ZERO; // draw() uses local space
        let (end_x, end_y) = if self.hit {
            // Convert global hit point to local
            let global_origin = self.base().get_global_position();
            (self.hit_x - global_origin.x, self.hit_y - global_origin.y)
        } else {
            // Draw full ray length along last mouse direction (approximation)
            (self.ray_length, 0.0)
        };
        let end = Vector2::new(end_x, end_y);

        let color = if self.hit {
            Color::from_rgb(1.0, 0.2, 0.2)
        } else {
            Color::from_rgb(0.2, 1.0, 0.2)
        };

        self.base_mut().draw_line(origin, end, color);
    }
}

// ---------------------------------------------------------------------------
// Pure helpers
// ---------------------------------------------------------------------------

/// Compute the endpoint of a ray given origin, normalised direction, and length.
pub fn ray_endpoint(ox: f32, oy: f32, dx: f32, dy: f32, len: f32) -> (f32, f32) {
    (ox + dx * len, oy + dy * len)
}

/// Normalise a 2-D vector. Returns (0, 0) if the input is near-zero.
pub fn normalize_2d(dx: f32, dy: f32) -> (f32, f32) {
    let len = (dx * dx + dy * dy).sqrt();
    if len < 1e-6 {
        (0.0, 0.0)
    } else {
        (dx / len, dy / len)
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ray_endpoint_right() {
        let (ex, ey) = ray_endpoint(0.0, 0.0, 1.0, 0.0, 300.0);
        assert!((ex - 300.0).abs() < 0.001);
        assert!(ey.abs() < 0.001);
    }

    #[test]
    fn ray_endpoint_offset_origin() {
        let (ex, ey) = ray_endpoint(100.0, 50.0, 0.0, 1.0, 200.0);
        assert!((ex - 100.0).abs() < 0.001);
        assert!((ey - 250.0).abs() < 0.001);
    }

    #[test]
    fn normalize_unit_right() {
        let (nx, ny) = normalize_2d(5.0, 0.0);
        assert!((nx - 1.0).abs() < 0.001);
        assert!(ny.abs() < 0.001);
    }

    #[test]
    fn normalize_diagonal() {
        let (nx, ny) = normalize_2d(1.0, 1.0);
        let len = (nx * nx + ny * ny).sqrt();
        assert!((len - 1.0).abs() < 0.001);
    }

    #[test]
    fn normalize_zero_vector() {
        let (nx, ny) = normalize_2d(0.0, 0.0);
        assert_eq!(nx, 0.0);
        assert_eq!(ny, 0.0);
    }

    #[test]
    fn ray_endpoint_negative_dir() {
        let (ex, ey) = ray_endpoint(0.0, 0.0, -1.0, 0.0, 100.0);
        assert!((ex + 100.0).abs() < 0.001);
        assert!(ey.abs() < 0.001);
    }
}
