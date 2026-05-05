//! Tweening demo — animates a node's position and modulate color using Godot Tweens.

use godot::classes::{INode2D, Node2D, Tween};
use godot::prelude::*;

struct TweeningExtension;
#[gdextension]
unsafe impl ExtensionLibrary for TweeningExtension {}

#[derive(GodotClass)]
#[class(base=Node2D)]
struct TweeningDemo {
    #[export]
    duration: f32,
    #[export]
    distance: f32,

    tween: Option<Gd<Tween>>,
    base: Base<Node2D>,
}

#[godot_api]
impl INode2D for TweeningDemo {
    fn init(base: Base<Node2D>) -> Self {
        Self {
            duration: 1.5,
            distance: 300.0,
            tween: None,
            base,
        }
    }

    fn ready(&mut self) {
        self.start_tween();
    }
}

#[godot_api]
impl TweeningDemo {
    #[func]
    pub fn restart_tween(&mut self) {
        if let Some(mut t) = self.tween.take() {
            t.kill();
        }
        self.start_tween();
    }
}

impl TweeningDemo {
    fn start_tween(&mut self) {
        let duration = self.duration as f64;
        let distance = self.distance;

        let start_pos = Vector2::new(100.0, 300.0);
        let end_pos = Vector2::new(100.0 + distance, 300.0);

        // Reset position and colour before starting
        self.base_mut().set_position(start_pos);
        self.base_mut().set_modulate(Color::WHITE);

        let mut tween = self.base_mut().create_tween();

        // Tween position from left to right
        let obj = self.base().clone().upcast::<Object>();
        let pos_path = NodePath::from("position");
        tween.tween_property(&obj, &pos_path, &Variant::from(end_pos), duration);

        // Tween colour from white to red after the position tween finishes
        let color_path = NodePath::from("modulate");
        tween.tween_property(
            &obj,
            &color_path,
            &Variant::from(Color::from_rgb(1.0, 0.0, 0.0)),
            duration,
        );

        self.tween = Some(tween);
    }
}

// ---------------------------------------------------------------------------
// Pure helper functions
// ---------------------------------------------------------------------------

/// Smoothstep ease-in-out for t in [0, 1].
pub fn ease_in_out(t: f32) -> f32 {
    let t = t.clamp(0.0, 1.0);
    t * t * (3.0 - 2.0 * t)
}

/// Linear interpolation between a and b by t.
pub fn lerp_f32(a: f32, b: f32, t: f32) -> f32 {
    a + (b - a) * t
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ease_in_out_at_zero_is_zero() {
        assert!((ease_in_out(0.0) - 0.0).abs() < 1e-6);
    }

    #[test]
    fn ease_in_out_at_one_is_one() {
        assert!((ease_in_out(1.0) - 1.0).abs() < 1e-6);
    }

    #[test]
    fn ease_in_out_at_half_is_half() {
        assert!((ease_in_out(0.5) - 0.5).abs() < 1e-6);
    }

    #[test]
    fn ease_in_out_clamps_below_zero() {
        assert!((ease_in_out(-1.0) - 0.0).abs() < 1e-6);
    }

    #[test]
    fn ease_in_out_clamps_above_one() {
        assert!((ease_in_out(2.0) - 1.0).abs() < 1e-6);
    }

    #[test]
    fn lerp_f32_at_zero() {
        assert!((lerp_f32(10.0, 20.0, 0.0) - 10.0).abs() < 1e-6);
    }

    #[test]
    fn lerp_f32_at_one() {
        assert!((lerp_f32(10.0, 20.0, 1.0) - 20.0).abs() < 1e-6);
    }

    #[test]
    fn lerp_f32_at_half() {
        assert!((lerp_f32(0.0, 100.0, 0.5) - 50.0).abs() < 1e-6);
    }

    #[test]
    fn ease_in_out_monotone() {
        let mut prev = ease_in_out(0.0);
        for i in 1..=10 {
            let t = i as f32 / 10.0;
            let cur = ease_in_out(t);
            assert!(cur >= prev, "ease_in_out should be monotone");
            prev = cur;
        }
    }

    #[test]
    fn lerp_f32_negative_range() {
        assert!((lerp_f32(-10.0, 10.0, 0.5) - 0.0).abs() < 1e-6);
    }
}
