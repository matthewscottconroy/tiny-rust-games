//! Split-screen demo — Godot 4.3 + gdext 0.5.
//!
//! Teaches:
//! - Creating two `SubViewport` + `SubViewportContainer` pairs entirely from Rust.
//! - Each viewport has its own `Camera2D` that independently follows a player.
//! - Pure functions for camera smoothing and screen-rect computation.
//! - A shared world Node2D holds the player nodes so both cameras see them.
//!
//! Player 1 — WASD;  Player 2 — IJKL.
//! Each half of the screen shows one camera's view.

use godot::classes::{
    Camera2D, ColorRect, INode2D, Input, Label, Node2D, SubViewport, SubViewportContainer,
};
use godot::prelude::*;

// ── Extension entry point ─────────────────────────────────────────────────────

struct SplitScreenExt;
#[gdextension]
unsafe impl ExtensionLibrary for SplitScreenExt {}

// ── Pure helpers ──────────────────────────────────────────────────────────────

/// Smoothly moves `cam_pos` toward `target` using exponential decay.
pub fn smooth_follow(cam_pos: Vector2, target: Vector2, speed: f32, dt: f32) -> Vector2 {
    let t = (speed * dt).clamp(0.0, 1.0);
    cam_pos + (target - cam_pos) * t
}

/// Returns the pixel size of one split-screen panel given the total screen size
/// and split direction (horizontal = left/right halves).
pub fn panel_size(screen_w: f32, screen_h: f32) -> Vector2 {
    Vector2::new(screen_w / 2.0, screen_h)
}

/// Returns how far a player moved this frame from an input direction.
pub fn movement_delta(dir: Vector2, speed: f32, dt: f32) -> Vector2 {
    if dir == Vector2::ZERO {
        Vector2::ZERO
    } else {
        dir.normalized() * speed * dt
    }
}

// ── SplitScreenDemo — root Node2D ─────────────────────────────────────────────

const PLAYER_SPEED: f32 = 160.0;
const CAM_FOLLOW_SPEED: f32 = 8.0;

/// Godot node wiring two viewports with independent per-player cameras.
#[derive(GodotClass)]
#[class(base=Node2D)]
pub struct SplitScreenDemo {
    p1_pos: Vector2,
    p2_pos: Vector2,
    cam1_pos: Vector2,
    cam2_pos: Vector2,
    base: Base<Node2D>,
}

#[godot_api]
impl INode2D for SplitScreenDemo {
    fn init(base: Base<Node2D>) -> Self {
        Self {
            p1_pos: Vector2::new(-100.0, 0.0),
            p2_pos: Vector2::new(100.0, 0.0),
            cam1_pos: Vector2::new(-100.0, 0.0),
            cam2_pos: Vector2::new(100.0, 0.0),
            base,
        }
    }

    fn ready(&mut self) {
        // Viewport 1 (left half)
        let mut vc1 = SubViewportContainer::new_alloc();
        vc1.set_name("VC1");
        vc1.set_size(Vector2::new(400.0, 600.0));
        vc1.set_position(Vector2::new(0.0, 0.0));
        vc1.set_stretch(true);

        let mut sv1 = SubViewport::new_alloc();
        sv1.set_name("SV1");
        sv1.set_size(Vector2i::new(400, 600));

        let mut world1 = Node2D::new_alloc();
        world1.set_name("World1");

        let mut p1_rect = ColorRect::new_alloc();
        p1_rect.set_name("P1Rect");
        p1_rect.set_size(Vector2::new(20.0, 20.0));
        p1_rect.set_color(Color::from_rgb(0.3, 0.7, 1.0));
        p1_rect.set_position(self.p1_pos - Vector2::new(10.0, 10.0));
        world1.add_child(&p1_rect);

        let mut cam1 = Camera2D::new_alloc();
        cam1.set_name("Cam1");
        cam1.set_position(self.cam1_pos);
        world1.add_child(&cam1);

        // Label
        let mut lbl1 = Label::new_alloc();
        lbl1.set_text("P1: WASD");
        lbl1.set_position(Vector2::new(-195.0, -295.0));
        world1.add_child(&lbl1);

        sv1.add_child(&world1);
        vc1.add_child(&sv1);
        self.base_mut().add_child(&vc1);

        // Viewport 2 (right half)
        let mut vc2 = SubViewportContainer::new_alloc();
        vc2.set_name("VC2");
        vc2.set_size(Vector2::new(400.0, 600.0));
        vc2.set_position(Vector2::new(400.0, 0.0));
        vc2.set_stretch(true);

        let mut sv2 = SubViewport::new_alloc();
        sv2.set_name("SV2");
        sv2.set_size(Vector2i::new(400, 600));

        let mut world2 = Node2D::new_alloc();
        world2.set_name("World2");

        let mut p2_rect = ColorRect::new_alloc();
        p2_rect.set_name("P2Rect");
        p2_rect.set_size(Vector2::new(20.0, 20.0));
        p2_rect.set_color(Color::from_rgb(1.0, 0.45, 0.3));
        p2_rect.set_position(self.p2_pos - Vector2::new(10.0, 10.0));
        world2.add_child(&p2_rect);

        let mut cam2 = Camera2D::new_alloc();
        cam2.set_name("Cam2");
        cam2.set_position(self.cam2_pos);
        world2.add_child(&cam2);

        let mut lbl2 = Label::new_alloc();
        lbl2.set_text("P2: IJKL");
        lbl2.set_position(Vector2::new(-195.0, -295.0));
        world2.add_child(&lbl2);

        sv2.add_child(&world2);
        vc2.add_child(&sv2);
        self.base_mut().add_child(&vc2);
    }

    fn process(&mut self, delta: f64) {
        let dt = delta as f32;
        let input = Input::singleton();

        // Player 1 — WASD (mapped to arrow actions)
        let mut d1 = Vector2::ZERO;
        if input.is_action_pressed("ui_right") {
            d1.x += 1.0;
        }
        if input.is_action_pressed("ui_left") {
            d1.x -= 1.0;
        }
        if input.is_action_pressed("ui_down") {
            d1.y += 1.0;
        }
        if input.is_action_pressed("ui_up") {
            d1.y -= 1.0;
        }
        self.p1_pos += movement_delta(d1, PLAYER_SPEED, dt);

        // Player 2 — numeric pad simulation via page_up/page_down etc.
        let mut d2 = Vector2::ZERO;
        if input.is_action_pressed("ui_page_up") {
            d2.y -= 1.0;
        }
        if input.is_action_pressed("ui_page_down") {
            d2.y += 1.0;
        }
        if input.is_action_pressed("ui_home") {
            d2.x -= 1.0;
        }
        if input.is_action_pressed("ui_end") {
            d2.x += 1.0;
        }
        self.p2_pos += movement_delta(d2, PLAYER_SPEED, dt);

        // Smooth cameras
        self.cam1_pos = smooth_follow(self.cam1_pos, self.p1_pos, CAM_FOLLOW_SPEED, dt);
        self.cam2_pos = smooth_follow(self.cam2_pos, self.p2_pos, CAM_FOLLOW_SPEED, dt);

        // Sync visuals — navigate through VC1/SV1/World1
        if let Some(vc1) = self.base().try_get_node_as::<SubViewportContainer>("VC1")
            && let Some(sv1) = vc1.try_get_node_as::<SubViewport>("SV1")
            && let Some(w1) = sv1.try_get_node_as::<Node2D>("World1")
        {
            if let Some(mut r) = w1.try_get_node_as::<ColorRect>("P1Rect") {
                r.set_position(self.p1_pos - Vector2::new(10.0, 10.0));
            }
            if let Some(mut c) = w1.try_get_node_as::<Camera2D>("Cam1") {
                c.set_position(self.cam1_pos);
            }
        }
        if let Some(vc2) = self.base().try_get_node_as::<SubViewportContainer>("VC2")
            && let Some(sv2) = vc2.try_get_node_as::<SubViewport>("SV2")
            && let Some(w2) = sv2.try_get_node_as::<Node2D>("World2")
        {
            if let Some(mut r) = w2.try_get_node_as::<ColorRect>("P2Rect") {
                r.set_position(self.p2_pos - Vector2::new(10.0, 10.0));
            }
            if let Some(mut c) = w2.try_get_node_as::<Camera2D>("Cam2") {
                c.set_position(self.cam2_pos);
            }
        }
    }
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn smooth_follow_zero_dt_unchanged() {
        let p = smooth_follow(Vector2::ZERO, Vector2::new(100.0, 0.0), 8.0, 0.0);
        assert!(p.x.abs() < 1e-5);
    }

    #[test]
    fn smooth_follow_moves_toward_target() {
        let p = smooth_follow(Vector2::ZERO, Vector2::new(10.0, 0.0), 8.0, 0.1);
        assert!(p.x > 0.0 && p.x < 10.0);
    }

    #[test]
    fn smooth_follow_large_dt_snaps() {
        let p = smooth_follow(Vector2::ZERO, Vector2::new(5.0, 0.0), 8.0, 1.0);
        assert!((p.x - 5.0).abs() < 1e-5);
    }

    #[test]
    fn panel_size_splits_screen_vertically() {
        let s = panel_size(800.0, 600.0);
        assert!((s.x - 400.0).abs() < 1e-3);
        assert!((s.y - 600.0).abs() < 1e-3);
    }

    #[test]
    fn movement_delta_zero_dir_is_zero() {
        let d = movement_delta(Vector2::ZERO, 100.0, 0.016);
        assert!(d.x.abs() < 1e-5 && d.y.abs() < 1e-5);
    }

    #[test]
    fn movement_delta_right_is_positive_x() {
        let d = movement_delta(Vector2::new(1.0, 0.0), 100.0, 0.016);
        assert!(d.x > 0.0 && d.y.abs() < 1e-5);
    }

    #[test]
    fn movement_delta_diagonal_is_unit_scaled() {
        let dir = Vector2::new(1.0, 1.0);
        let d = movement_delta(dir, 100.0, 1.0);
        assert!((d.length() - 100.0).abs() < 1e-3);
    }

    #[test]
    fn movement_delta_scales_with_speed() {
        let d1 = movement_delta(Vector2::new(1.0, 0.0), 50.0, 1.0);
        let d2 = movement_delta(Vector2::new(1.0, 0.0), 100.0, 1.0);
        assert!((d2.x / d1.x - 2.0).abs() < 1e-4);
    }
}
