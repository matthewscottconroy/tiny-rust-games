//! Parallax Background GDExtension demo — multiple ParallaxLayer nodes driven
//! by a Camera2D position updated from Rust, giving a depth-layered scrolling
//! effect at different speeds.
//!
//! Teaches:
//!
//! - Controlling a `Camera2D` position from Rust each frame.
//! - How `ParallaxBackground` auto-scrolls its `ParallaxLayer` children when
//!   the camera moves (no manual offset calculation needed at runtime).
//! - Configuring `motion_scale` on `ParallaxLayer` children in `ready()`.
//! - Exposing `reset_scroll` and `set_scroll_speed` as `#[func]` methods.
//! - Pure helper functions for parallax math covered by unit tests.
//!
//! Counterpart: tech-demos/bevy/parallax-background — the same concept in Bevy.

use godot::classes::{Camera2D, INode2D, Node2D, ParallaxLayer};
use godot::prelude::*;

// ─── Extension entry point ────────────────────────────────────────────────────

struct ParallaxBackgroundExt;

#[gdextension]
unsafe impl ExtensionLibrary for ParallaxBackgroundExt {}

// ─── Pure functions ───────────────────────────────────────────────────────────

/// Returns how far a parallax layer has shifted given the camera position and
/// the layer's motion scale.
///
/// This mirrors how Godot internally offsets each layer. A `scale` of `0.0`
/// means the layer does not move (infinite depth); `1.0` moves with the camera.
///
/// # Examples
/// ```
/// assert!((parallax_background::parallax_offset(100.0, 0.5) - 50.0).abs() < 1e-5);
/// assert_eq!(parallax_background::parallax_offset(100.0, 0.0), 0.0);
/// assert_eq!(parallax_background::parallax_offset(100.0, 1.0), 100.0);
/// ```
pub fn parallax_offset(camera_x: f32, scale: f32) -> f32 {
    camera_x * scale
}

/// Returns the effective horizontal scroll speed for a layer given the base
/// camera scroll speed and the layer's motion scale.
///
/// # Examples
/// ```
/// assert!((parallax_background::layer_speed(80.0, 0.5) - 40.0).abs() < 1e-5);
/// assert_eq!(parallax_background::layer_speed(80.0, 0.0), 0.0);
/// ```
pub fn layer_speed(base_speed: f32, scale: f32) -> f32 {
    base_speed * scale
}

/// Formats a human-readable status string showing camera position and scroll speed.
///
/// # Examples
/// ```
/// let s = parallax_background::format_scroll_info(120.5, 80.0);
/// assert!(s.contains("120.5"));
/// assert!(s.contains("80.0"));
/// ```
pub fn format_scroll_info(camera_x: f32, speed: f32) -> String {
    format!("x: {:.1}  speed: {:.1} px/s", camera_x, speed)
}

// ─── ParallaxDemo node ────────────────────────────────────────────────────────

/// A `Node2D` that scrolls a camera rightward, triggering automatic parallax
/// layer scrolling via Godot's built-in `ParallaxBackground` system.
///
/// Expected scene layout:
/// ```text
/// ParallaxDemo (this class, Node2D)
/// ├── Camera2D
/// ├── ParallaxBackground
/// │   ├── LayerFar   (ParallaxLayer, motion_scale = (0.1, 0))
/// │   ├── LayerMid   (ParallaxLayer, motion_scale = (0.4, 0))
/// │   └── LayerNear  (ParallaxLayer, motion_scale = (0.8, 0))
/// └── Label
/// ```
#[derive(GodotClass)]
#[class(base=Node2D)]
pub struct ParallaxDemo {
    /// Camera scroll speed in pixels per second.
    #[export]
    scroll_speed: f32,

    /// Accumulated camera X position.
    camera_x: f32,

    base: Base<Node2D>,
}

#[godot_api]
impl INode2D for ParallaxDemo {
    fn init(base: Base<Node2D>) -> Self {
        Self {
            scroll_speed: 80.0,
            camera_x: 0.0,
            base,
        }
    }

    fn ready(&mut self) {
        // Configure motion scales on each ParallaxLayer so different layers
        // scroll at different speeds relative to the camera.
        if let Some(mut layer) = self
            .base()
            .try_get_node_as::<ParallaxLayer>("ParallaxBackground/LayerFar")
        {
            layer.set_motion_scale(Vector2::new(0.1, 0.0));
        }
        if let Some(mut layer) = self
            .base()
            .try_get_node_as::<ParallaxLayer>("ParallaxBackground/LayerMid")
        {
            layer.set_motion_scale(Vector2::new(0.4, 0.0));
        }
        if let Some(mut layer) = self
            .base()
            .try_get_node_as::<ParallaxLayer>("ParallaxBackground/LayerNear")
        {
            layer.set_motion_scale(Vector2::new(0.8, 0.0));
        }

        godot_print!("[ParallaxDemo] Ready — scroll_speed={}", self.scroll_speed);
    }

    fn process(&mut self, delta: f64) {
        let speed = self.scroll_speed;
        self.camera_x += speed * delta as f32;

        let camera_x = self.camera_x;

        if let Some(mut camera) = self.base().try_get_node_as::<Camera2D>("Camera2D") {
            camera.set_position(Vector2::new(camera_x, 0.0));
        }

        // Update status label.
        let text = format_scroll_info(camera_x, speed);
        if let Some(mut label) = self
            .base()
            .try_get_node_as::<godot::classes::Label>("Label")
        {
            label.set_text(text.as_str());
        }
    }
}

#[godot_api]
impl ParallaxDemo {
    /// Resets camera_x to 0 and repositions the Camera2D to the origin.
    #[func]
    pub fn reset_scroll(&mut self) {
        self.camera_x = 0.0;
        if let Some(mut camera) = self.base().try_get_node_as::<Camera2D>("Camera2D") {
            camera.set_position(Vector2::new(0.0, 0.0));
        }
        godot_print!("[ParallaxDemo] Scroll reset.");
    }

    /// Updates the scroll speed. Note: `#[export]` already generates
    /// `set_scroll_speed` as a property setter, so this is exposed with a
    /// distinct name for explicit callable use.
    #[func]
    pub fn apply_scroll_speed(&mut self, speed: f32) {
        self.scroll_speed = speed;
        godot_print!("[ParallaxDemo] scroll_speed set to {}", speed);
    }
}

// ─── Tests ────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    // parallax_offset ──────────────────────────────────────────────────────────

    #[test]
    fn parallax_offset_half_scale() {
        assert!((parallax_offset(100.0, 0.5) - 50.0).abs() < 1e-5);
    }

    #[test]
    fn parallax_offset_zero_scale_does_not_move() {
        assert_eq!(parallax_offset(100.0, 0.0), 0.0);
    }

    #[test]
    fn parallax_offset_full_scale_equals_camera() {
        assert_eq!(parallax_offset(100.0, 1.0), 100.0);
    }

    #[test]
    fn parallax_offset_zero_camera() {
        assert_eq!(parallax_offset(0.0, 0.5), 0.0);
    }

    // layer_speed ──────────────────────────────────────────────────────────────

    #[test]
    fn layer_speed_half_scale() {
        assert!((layer_speed(80.0, 0.5) - 40.0).abs() < 1e-5);
    }

    #[test]
    fn layer_speed_zero_scale_is_zero() {
        assert_eq!(layer_speed(80.0, 0.0), 0.0);
    }

    #[test]
    fn layer_speed_full_scale_equals_base() {
        assert_eq!(layer_speed(80.0, 1.0), 80.0);
    }

    // format_scroll_info ───────────────────────────────────────────────────────

    #[test]
    fn format_scroll_info_contains_camera_x() {
        let s = format_scroll_info(120.5, 80.0);
        assert!(s.contains("120.5"), "missing camera_x in: {s}");
    }

    #[test]
    fn format_scroll_info_contains_speed() {
        let s = format_scroll_info(0.0, 80.0);
        assert!(s.contains("80.0"), "missing speed in: {s}");
    }
}
