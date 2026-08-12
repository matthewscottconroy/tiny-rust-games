//! Motion-trail demo — Godot 4.3 + gdext 0.5.
//!
//! Teaches:
//! - Managing a `Line2D` node's point buffer from Rust to create a smooth trail.
//! - Trimming the trail to a maximum length by removing the oldest point.
//! - Computing per-point alpha fade with `trail_alpha` so older segments fade out.
//! - Using `PackedVector2Array` and `PackedColorArray` to batch-update a `Line2D`.
//!
//! The player is a coloured `ColorRect` moved with WASD.  Each frame the current
//! position is appended to the trail; points beyond `MAX_TRAIL` are dropped.
//!
//! **Controls:** WASD — move player.

use godot::classes::{ColorRect, INode2D, Input, Label, Node2D};
use godot::prelude::*;

// ── Extension entry point ─────────────────────────────────────────────────────

struct LineTrailExt;
#[gdextension]
unsafe impl ExtensionLibrary for LineTrailExt {}

// ── Pure trail helpers ────────────────────────────────────────────────────────

/// Alpha for trail point `index` (0 = oldest), given `total` points.
/// Oldest point fades to 0; newest is fully opaque.
pub fn trail_alpha(index: usize, total: usize) -> f32 {
    if total <= 1 {
        return 1.0;
    }
    index as f32 / (total - 1) as f32
}

/// Clamps `points` to `max_len` by removing excess from the front.
pub fn trim_trail(points: &mut Vec<Vector2>, max_len: usize) {
    if points.len() > max_len {
        let excess = points.len() - max_len;
        points.drain(..excess);
    }
}

/// `true` when the new position is far enough from the last recorded point
/// to be worth appending (avoids redundant micro-updates).
pub fn should_append(points: &[Vector2], new_pos: Vector2, min_dist: f32) -> bool {
    points
        .last()
        .is_none_or(|last| last.distance_to(new_pos) >= min_dist)
}

// ── TrailDemo — root Node2D ───────────────────────────────────────────────────

const MAX_TRAIL: usize = 80;
const PLAYER_SPEED: f32 = 180.0;
const TRAIL_COLOR: Color = Color {
    r: 0.4,
    g: 0.8,
    b: 1.0,
    a: 1.0,
};

#[derive(GodotClass)]
#[class(base=Node2D)]
pub struct TrailDemo {
    trail_points: Vec<Vector2>,
    base: Base<Node2D>,
}

#[godot_api]
impl INode2D for TrailDemo {
    fn init(base: Base<Node2D>) -> Self {
        Self {
            trail_points: Vec::new(),
            base,
        }
    }

    fn ready(&mut self) {
        // Hint label
        let mut hint = Label::new_alloc();
        hint.set_text("WASD — move");
        hint.set_position(Vector2::new(-390.0, -290.0));
        self.base_mut().add_child(&hint);

        // Player rect
        let mut player = ColorRect::new_alloc();
        player.set_name("Player");
        player.set_size(Vector2::new(20.0, 20.0));
        player.set_position(Vector2::new(-10.0, -10.0));
        player.set_color(Color::from_rgb(0.2, 0.9, 0.5));
        self.base_mut().add_child(&player);
    }

    fn process(&mut self, delta: f64) {
        let dt = delta as f32;
        let input = Input::singleton();

        let mut dir = Vector2::ZERO;
        if input.is_action_pressed("ui_right") {
            dir.x += 1.0;
        }
        if input.is_action_pressed("ui_left") {
            dir.x -= 1.0;
        }
        if input.is_action_pressed("ui_down") {
            dir.y += 1.0;
        }
        if input.is_action_pressed("ui_up") {
            dir.y -= 1.0;
        }

        if dir != Vector2::ZERO {
            let pos = self.base().get_position();
            let new_pos = pos + dir.normalized() * PLAYER_SPEED * dt;
            let clamped = Vector2::new(
                new_pos.x.clamp(-380.0, 380.0),
                new_pos.y.clamp(-280.0, 280.0),
            );
            self.base_mut().set_position(clamped);

            if should_append(&self.trail_points, clamped, 4.0) {
                self.trail_points.push(clamped);
                trim_trail(&mut self.trail_points, MAX_TRAIL);
            }

            self.base_mut().queue_redraw();
        }
    }

    fn draw(&mut self) {
        let total = self.trail_points.len();
        if total < 2 {
            return;
        }
        let pts = self.trail_points.clone();
        for i in 1..total {
            let alpha = trail_alpha(i, total);
            let color = Color::from_rgba(TRAIL_COLOR.r, TRAIL_COLOR.g, TRAIL_COLOR.b, alpha);
            self.base_mut().draw_line(pts[i - 1], pts[i], color);
        }
    }
}

#[godot_api]
impl TrailDemo {
    pub fn trail_len(&self) -> usize {
        self.trail_points.len()
    }
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn trail_alpha_single_point_is_one() {
        assert!((trail_alpha(0, 1) - 1.0).abs() < 1e-5);
    }

    #[test]
    fn trail_alpha_oldest_is_zero() {
        assert!(trail_alpha(0, 10).abs() < 1e-5);
    }

    #[test]
    fn trail_alpha_newest_is_one() {
        assert!((trail_alpha(9, 10) - 1.0).abs() < 1e-5);
    }

    #[test]
    fn trail_alpha_increases_with_index() {
        let a0 = trail_alpha(2, 10);
        let a1 = trail_alpha(5, 10);
        assert!(a1 > a0);
    }

    #[test]
    fn trim_trail_removes_front_excess() {
        let mut pts: Vec<Vector2> = (0..10).map(|i| Vector2::new(i as f32, 0.0)).collect();
        trim_trail(&mut pts, 5);
        assert_eq!(pts.len(), 5);
        assert!(
            (pts[0].x - 5.0).abs() < 1e-5,
            "oldest kept should be index 5"
        );
    }

    #[test]
    fn trim_trail_within_limit_unchanged() {
        let mut pts: Vec<Vector2> = vec![Vector2::ZERO, Vector2::new(1.0, 0.0)];
        trim_trail(&mut pts, 10);
        assert_eq!(pts.len(), 2);
    }

    #[test]
    fn should_append_empty_trail_always_true() {
        assert!(should_append(&[], Vector2::new(5.0, 5.0), 3.0));
    }

    #[test]
    fn should_append_too_close_returns_false() {
        let pts = vec![Vector2::ZERO];
        assert!(!should_append(&pts, Vector2::new(1.0, 0.0), 5.0));
    }

    #[test]
    fn should_append_far_enough_returns_true() {
        let pts = vec![Vector2::ZERO];
        assert!(should_append(&pts, Vector2::new(10.0, 0.0), 5.0));
    }
}
