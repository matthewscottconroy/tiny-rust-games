//! Camera zoom demo — Godot 4.3 + gdext 0.5.
//!
//! Teaches:
//! - Driving `Camera2D::set_zoom` from Rust.
//! - `clamp_zoom` keeps zoom within designer-specified limits.
//! - `lerp_zoom` smoothly interpolates the camera toward the target zoom each frame.
//! - Reading mouse-wheel input via `InputEventMouseButton` and `WHEEL_UP/DOWN`.
//! - Panning the world with WASD while the zoom is applied.
//!
//! Zoom is stored as a single f32 scalar; `Camera2D.zoom` is `Vector2::ONE * zoom`.
//!
//! **Controls:** scroll wheel — zoom;  WASD — pan.

use godot::classes::{
    Camera2D, ColorRect, INode2D, Input, InputEvent, InputEventMouseButton, Label, Node2D,
};
use godot::global::MouseButton;
use godot::prelude::*;

// ── Extension entry point ─────────────────────────────────────────────────────

struct CameraZoomExt;
#[gdextension]
unsafe impl ExtensionLibrary for CameraZoomExt {}

// ── Pure zoom helpers ─────────────────────────────────────────────────────────

/// Clamps `zoom` to `[min, max]`.
pub fn clamp_zoom(zoom: f32, min: f32, max: f32) -> f32 {
    zoom.clamp(min, max)
}

/// Exponentially smooth `current` toward `target`.
/// `speed` is units per second; higher = snappier.
pub fn lerp_zoom(current: f32, target: f32, speed: f32, dt: f32) -> f32 {
    let t = (speed * dt).clamp(0.0, 1.0);
    current + (target - current) * t
}

/// Converts a zoom scalar to the `Vector2` Godot uses for `Camera2D.zoom`.
pub fn zoom_vector(zoom: f32) -> Vector2 {
    Vector2::new(zoom, zoom)
}

const ZOOM_MIN: f32 = 0.25;
const ZOOM_MAX: f32 = 4.0;
const ZOOM_STEP: f32 = 0.15;
const ZOOM_SPEED: f32 = 10.0;
const PAN_SPEED: f32 = 220.0;

// ── ZoomDemo — root Node2D ────────────────────────────────────────────────────

#[derive(GodotClass)]
#[class(base=Node2D)]
pub struct ZoomDemo {
    target_zoom: f32,
    current_zoom: f32,
    base: Base<Node2D>,
}

#[godot_api]
impl INode2D for ZoomDemo {
    fn init(base: Base<Node2D>) -> Self {
        Self {
            target_zoom: 1.0,
            current_zoom: 1.0,
            base,
        }
    }

    fn ready(&mut self) {
        // Camera
        let mut cam = Camera2D::new_alloc();
        cam.set_name("Camera");
        cam.set_zoom(zoom_vector(1.0));
        self.base_mut().add_child(&cam);

        // Scatter some coloured rects as landmarks
        let landmarks = [
            ((-200.0, -100.0), Color::from_rgb(0.8, 0.3, 0.3)),
            ((150.0, 80.0), Color::from_rgb(0.3, 0.7, 0.4)),
            ((-100.0, 200.0), Color::from_rgb(0.4, 0.5, 0.9)),
            ((250.0, -180.0), Color::from_rgb(0.9, 0.75, 0.2)),
            ((0.0, 0.0), Color::from_rgb(0.7, 0.7, 0.7)),
        ];
        for ((x, y), color) in landmarks {
            let mut rect = ColorRect::new_alloc();
            rect.set_size(Vector2::new(50.0, 50.0));
            rect.set_position(Vector2::new(x - 25.0, y - 25.0));
            rect.set_color(color);
            self.base_mut().add_child(&rect);
        }

        // HUD label (lives on a fixed screen-space layer via relative label position)
        let mut hint = Label::new_alloc();
        hint.set_name("Hint");
        hint.set_text("scroll: zoom   WASD: pan");
        hint.set_position(Vector2::new(-390.0, -290.0));
        self.base_mut().add_child(&hint);

        let mut zoom_lbl = Label::new_alloc();
        zoom_lbl.set_name("ZoomLabel");
        zoom_lbl.set_position(Vector2::new(-390.0, -265.0));
        self.base_mut().add_child(&zoom_lbl);
    }

    fn input(&mut self, event: Gd<InputEvent>) {
        if let Ok(mb) = event.try_cast::<InputEventMouseButton>()
            && mb.is_pressed()
        {
            let step = match mb.get_button_index() {
                MouseButton::WHEEL_UP => -ZOOM_STEP,
                MouseButton::WHEEL_DOWN => ZOOM_STEP,
                _ => return,
            };
            // Zoom in = larger scalar (objects bigger), scroll up = zoom in
            self.target_zoom = clamp_zoom(self.target_zoom - step, ZOOM_MIN, ZOOM_MAX);
        }
    }

    fn process(&mut self, delta: f64) {
        let dt = delta as f32;
        let input = Input::singleton();

        // Pan
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
            let zoom = self.current_zoom;
            self.base_mut()
                .set_position(pos + dir.normalized() * PAN_SPEED * dt / zoom);
        }

        // Smooth zoom
        self.current_zoom = lerp_zoom(self.current_zoom, self.target_zoom, ZOOM_SPEED, dt);
        if let Some(mut cam) = self.base().try_get_node_as::<Camera2D>("Camera") {
            cam.set_zoom(zoom_vector(self.current_zoom));
        }

        // Update HUD
        let zoom_text = format!("Zoom: {:.2}×", self.current_zoom);
        if let Some(mut lbl) = self.base().try_get_node_as::<Label>("ZoomLabel") {
            lbl.set_text(&GString::from(zoom_text.as_str()));
        }
    }
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn clamp_zoom_within_range_unchanged() {
        assert!((clamp_zoom(1.5, 0.25, 4.0) - 1.5).abs() < 1e-5);
    }

    #[test]
    fn clamp_zoom_below_min_becomes_min() {
        assert!((clamp_zoom(0.1, 0.25, 4.0) - 0.25).abs() < 1e-5);
    }

    #[test]
    fn clamp_zoom_above_max_becomes_max() {
        assert!((clamp_zoom(5.0, 0.25, 4.0) - 4.0).abs() < 1e-5);
    }

    #[test]
    fn lerp_zoom_at_zero_dt_unchanged() {
        let z = lerp_zoom(1.0, 3.0, 10.0, 0.0);
        assert!((z - 1.0).abs() < 1e-5);
    }

    #[test]
    fn lerp_zoom_converges_toward_target() {
        // speed=10, dt=0.04 → t=0.4, partial step
        let z = lerp_zoom(1.0, 2.0, 10.0, 0.04);
        assert!(z > 1.0 && z < 2.0);
    }

    #[test]
    fn lerp_zoom_large_dt_snaps_to_target() {
        let z = lerp_zoom(1.0, 3.0, 10.0, 1.0);
        assert!((z - 3.0).abs() < 1e-5);
    }

    #[test]
    fn zoom_vector_makes_uniform_scale() {
        let v = zoom_vector(2.0);
        assert!((v.x - 2.0).abs() < 1e-5 && (v.y - 2.0).abs() < 1e-5);
    }

    #[test]
    fn zoom_vector_one_is_identity() {
        let v = zoom_vector(1.0);
        assert!((v.x - 1.0).abs() < 1e-5);
    }
}
