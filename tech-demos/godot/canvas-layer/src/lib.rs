//! Canvas Layer demo — a CanvasLayer keeps HUD elements fixed on screen
//! while a Camera2D scrolls through the world. Demonstrates that UI pinned
//! to a CanvasLayer is unaffected by camera movement.

use godot::classes::{Camera2D, CanvasLayer, INode2D, Label, Node2D};
use godot::prelude::*;

struct CanvasLayerExtension;
#[gdextension]
unsafe impl ExtensionLibrary for CanvasLayerExtension {}

/// Root Node2D that acts as the game world. Spawns a moving Camera2D and a
/// CanvasLayer-pinned HUD so the player can see that the HUD never moves.
#[derive(GodotClass)]
#[class(base=Node2D)]
struct CanvasLayerWorld {
    /// Pixels per second the camera scrolls.
    #[export]
    scroll_speed: f32,

    /// Accumulated horizontal camera offset (increases each frame).
    camera_offset: f32,

    base: Base<Node2D>,
}

#[godot_api]
impl INode2D for CanvasLayerWorld {
    fn init(base: Base<Node2D>) -> Self {
        Self {
            scroll_speed: 100.0,
            camera_offset: 0.0,
            base,
        }
    }

    fn ready(&mut self) {
        // --- Camera2D child ---
        let mut camera = Camera2D::new_alloc();
        camera.set_position(Vector2::new(0.0, 0.0));
        let camera_node = camera.upcast::<Node>();
        // Store as local so base_mut borrow is not held across add_child
        let camera_ref = camera_node.clone();
        self.base_mut().add_child(&camera_ref);

        // --- CanvasLayer child (layer 10 keeps it above world geometry) ---
        let mut canvas = CanvasLayer::new_alloc();
        canvas.set_layer(10);

        // Static HUD label
        let mut hud_label = Label::new_alloc();
        hud_label.set_text("HUD — always fixed on screen");
        hud_label.set_position(Vector2::new(8.0, 8.0));
        let hud_ref = hud_label.upcast::<Node>();
        canvas.add_child(&hud_ref);

        // Dynamic camera-position label (updated in process)
        let mut pos_label = Label::new_alloc();
        pos_label.set_name("PosLabel");
        pos_label.set_text(hud_position_text(0.0).as_str());
        pos_label.set_position(Vector2::new(8.0, 40.0));
        let pos_ref = pos_label.upcast::<Node>();
        canvas.add_child(&pos_ref);

        let canvas_ref = canvas.upcast::<Node>();
        self.base_mut().add_child(&canvas_ref);
    }

    fn process(&mut self, delta: f64) {
        let speed = self.scroll_speed;
        let new_offset = scroll_offset(self.camera_offset, speed, delta as f32);
        self.camera_offset = new_offset;

        // Move the camera to simulate a scrolling world
        if let Some(mut cam) = self.base().try_get_node_as::<Camera2D>("Camera2D") {
            cam.set_position(Vector2::new(new_offset, 0.0));
        }

        // Update the HUD label to show current offset
        let text = hud_position_text(new_offset);
        // Navigate: Main → CanvasLayer → PosLabel
        if let Some(canvas) = self.base().try_get_node_as::<CanvasLayer>("CanvasLayer")
            && let Some(mut label) = canvas.try_get_node_as::<Label>("PosLabel")
        {
            label.set_text(text.as_str());
        }
    }
}

#[godot_api]
impl CanvasLayerWorld {
    /// Resets the camera scroll back to the origin.
    #[func]
    pub fn reset_camera(&mut self) {
        self.camera_offset = 0.0;
        if let Some(mut cam) = self.base().try_get_node_as::<Camera2D>("Camera2D") {
            cam.set_position(Vector2::new(0.0, 0.0));
        }
    }
}

// ---------------------------------------------------------------------------
// Pure helper functions
// ---------------------------------------------------------------------------

/// Build the HUD string showing the current camera offset.
pub fn hud_position_text(offset: f32) -> String {
    format!("Camera X: {:.1}", offset)
}

/// Advance the camera offset by one frame.
pub fn scroll_offset(current: f32, speed: f32, delta: f32) -> f32 {
    current + speed * delta
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn scroll_offset_advances_by_speed_times_delta() {
        let result = scroll_offset(0.0, 100.0, 0.016);
        assert!((result - 1.6).abs() < 1e-4, "got {result}");
    }

    #[test]
    fn scroll_offset_accumulates() {
        let after_first = scroll_offset(0.0, 100.0, 0.1);
        let after_second = scroll_offset(after_first, 100.0, 0.1);
        assert!((after_second - 20.0).abs() < 1e-4, "got {after_second}");
    }

    #[test]
    fn scroll_offset_zero_delta_no_change() {
        let result = scroll_offset(42.5, 100.0, 0.0);
        assert!((result - 42.5).abs() < 1e-6);
    }

    #[test]
    fn hud_position_text_formats_one_decimal() {
        let text = hud_position_text(123.456);
        assert_eq!(text, "Camera X: 123.5");
    }

    #[test]
    fn hud_position_text_zero() {
        let text = hud_position_text(0.0);
        assert_eq!(text, "Camera X: 0.0");
    }

    #[test]
    fn hud_position_text_negative() {
        let text = hud_position_text(-50.0);
        assert_eq!(text, "Camera X: -50.0");
    }

    #[test]
    fn scroll_offset_large_speed() {
        let result = scroll_offset(0.0, 500.0, 1.0);
        assert!((result - 500.0).abs() < 1e-4);
    }
}
