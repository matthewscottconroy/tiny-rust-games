//! Camera shake and color flash screen-effects demo — Godot 4.3 + gdext 0.5.
//!
//! Teaches:
//! - Driving camera shake from Rust using trauma → shake magnitude.
//! - Computing a per-frame shake offset from a trauma value and deterministic angle.
//! - Lerping a color flash overlay back to transparent from Rust.
//! - Wiring input to trigger both effects without GDScript.
//!
//! `trauma` decays every frame; `shake_offset` converts it to a random-looking
//! displacement using the LCG-derived angle so behaviour is testable without RNG.
//!
//! **Controls:** SPACE — camera shake;  F — color flash;  ESC — quit.

use godot::classes::{Camera2D, ColorRect, INode2D, Input, Label, Node2D};
use godot::prelude::*;

// ─── Extension entry point ────────────────────────────────────────────────────

struct ScreenEffectsExt;
#[gdextension]
unsafe impl ExtensionLibrary for ScreenEffectsExt {}

// ─── Pure effect helpers ──────────────────────────────────────────────────────

/// Maps `trauma ∈ [0,1]` to shake magnitude (trauma²) and displaces by `angle`.
pub fn shake_offset(trauma: f32, angle: f32) -> Vector2 {
    let magnitude = trauma * trauma * 32.0;
    Vector2::new(angle.cos() * magnitude, angle.sin() * magnitude)
}

/// Decay `trauma` by `rate * dt`, clamped to `[0, 1]`.
pub fn decay_trauma(trauma: f32, rate: f32, dt: f32) -> f32 {
    (trauma - rate * dt).clamp(0.0, 1.0)
}

/// Lerp `a` toward `b` by factor `t ∈ [0,1]`.
pub fn lerp_f32(a: f32, b: f32, t: f32) -> f32 {
    a + (b - a) * t.clamp(0.0, 1.0)
}

/// Lerp each channel of `from` toward `to` by `t`.
pub fn lerp_color(from: Color, to: Color, t: f32) -> Color {
    Color::from_rgba(
        lerp_f32(from.r, to.r, t),
        lerp_f32(from.g, to.g, t),
        lerp_f32(from.b, to.b, t),
        lerp_f32(from.a, to.a, t),
    )
}

/// A minimal LCG step — produces a pseudo-random angle in `[0, 2π)`.
pub fn lcg_angle(state: u64) -> (f32, u64) {
    let next = state
        .wrapping_mul(6364136223846793005)
        .wrapping_add(1442695040888963407);
    let angle = (next >> 33) as f32 / (u32::MAX as f32) * std::f32::consts::TAU;
    (angle, next)
}

// ─── EffectsArena — root Node2D ───────────────────────────────────────────────

/// Godot node driving camera shake and a colour-flash overlay.
#[derive(GodotClass)]
#[class(base=Node2D)]
pub struct EffectsArena {
    trauma: f32,
    rng_state: u64,
    flash_color: Color,
    base: Base<Node2D>,
}

#[godot_api]
impl INode2D for EffectsArena {
    fn init(base: Base<Node2D>) -> Self {
        Self {
            trauma: 0.0,
            rng_state: 12345,
            flash_color: Color::from_rgba(1.0, 0.2, 0.2, 0.0),
            base,
        }
    }

    fn ready(&mut self) {
        // HUD
        let mut hint = Label::new_alloc();
        hint.set_text("SPACE — shake   F — flash");
        hint.set_position(Vector2::new(10.0, 8.0));
        self.base_mut().add_child(&hint);

        // Full-screen color flash overlay (starts transparent)
        let mut overlay = ColorRect::new_alloc();
        overlay.set_name("FlashOverlay");
        overlay.set_size(Vector2::new(800.0, 600.0));
        overlay.set_position(Vector2::new(-400.0, -300.0));
        overlay.set_color(Color::from_rgba(1.0, 0.2, 0.2, 0.0));
        self.base_mut().add_child(&overlay);

        // Camera
        let mut cam = Camera2D::new_alloc();
        cam.set_name("Camera");
        self.base_mut().add_child(&cam);

        // Target sprite (so there's something to look at)
        let mut body = godot::classes::ColorRect::new_alloc();
        body.set_size(Vector2::new(60.0, 60.0));
        body.set_position(Vector2::new(-30.0, -30.0));
        body.set_color(Color::from_rgb(0.3, 0.7, 0.3));
        self.base_mut().add_child(&body);
    }

    fn process(&mut self, delta: f64) {
        let dt = delta as f32;
        let input = Input::singleton();

        if input.is_action_just_pressed("ui_accept") {
            self.trauma = (self.trauma + 0.7).min(1.0);
        }
        if input.is_action_just_pressed("ui_select") {
            self.flash_color.a = 0.6;
        }

        // Apply shake to camera
        if self.trauma > 0.0 {
            let (angle, next_state) = lcg_angle(self.rng_state);
            self.rng_state = next_state;
            let offset = shake_offset(self.trauma, angle);
            if let Some(mut cam) = self.base().try_get_node_as::<Camera2D>("Camera") {
                cam.set_offset(offset);
            }
            self.trauma = decay_trauma(self.trauma, 1.4, dt);
        } else if let Some(mut cam) = self.base().try_get_node_as::<Camera2D>("Camera") {
            cam.set_offset(Vector2::ZERO);
        }

        // Fade out flash overlay
        if self.flash_color.a > 0.0 {
            self.flash_color = lerp_color(
                self.flash_color,
                Color::from_rgba(1.0, 0.2, 0.2, 0.0),
                dt * 4.0,
            );
            if let Some(mut overlay) = self.base().try_get_node_as::<ColorRect>("FlashOverlay") {
                overlay.set_color(self.flash_color);
            }
        }
    }
}

// ─── Tests ────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn shake_offset_zero_trauma_is_zero() {
        let offset = shake_offset(0.0, 0.0);
        assert!(offset.x.abs() < 1e-4 && offset.y.abs() < 1e-4);
    }

    #[test]
    fn shake_offset_full_trauma_angle_zero_is_rightward() {
        let offset = shake_offset(1.0, 0.0);
        assert!((offset.x - 32.0).abs() < 1e-3, "x={}", offset.x);
        assert!(offset.y.abs() < 1e-3);
    }

    #[test]
    fn shake_offset_magnitude_scales_as_trauma_squared() {
        let half = shake_offset(0.5, 0.0);
        let full = shake_offset(1.0, 0.0);
        // half trauma → ¼ displacement
        assert!((half.x / full.x - 0.25).abs() < 1e-3);
    }

    #[test]
    fn decay_trauma_reduces_over_time() {
        let t = decay_trauma(1.0, 1.4, 0.1);
        assert!(t < 1.0);
        assert!(t > 0.0);
    }

    #[test]
    fn decay_trauma_clamped_at_zero() {
        let t = decay_trauma(0.1, 5.0, 1.0);
        assert_eq!(t, 0.0);
    }

    #[test]
    fn decay_trauma_never_exceeds_one() {
        let t = decay_trauma(0.5, -10.0, 1.0);
        assert_eq!(t, 1.0);
    }

    #[test]
    fn lerp_f32_zero_t_returns_a() {
        assert!((lerp_f32(2.0, 8.0, 0.0) - 2.0).abs() < 1e-5);
    }

    #[test]
    fn lerp_f32_one_t_returns_b() {
        assert!((lerp_f32(2.0, 8.0, 1.0) - 8.0).abs() < 1e-5);
    }

    #[test]
    fn lerp_f32_half_t_midpoint() {
        assert!((lerp_f32(0.0, 10.0, 0.5) - 5.0).abs() < 1e-5);
    }

    #[test]
    fn lerp_color_alpha_fades_to_zero() {
        let from = Color::from_rgba(1.0, 0.0, 0.0, 0.8);
        let to = Color::from_rgba(1.0, 0.0, 0.0, 0.0);
        let mid = lerp_color(from, to, 0.5);
        assert!((mid.a - 0.4).abs() < 1e-4);
    }

    #[test]
    fn lcg_angle_stays_in_tau_range() {
        let (angle, _) = lcg_angle(99999);
        assert!((0.0..std::f32::consts::TAU).contains(&angle));
    }

    #[test]
    fn lcg_angle_different_states_produce_different_angles() {
        let (a1, _) = lcg_angle(1);
        let (a2, _) = lcg_angle(2);
        assert!((a1 - a2).abs() > 1e-4);
    }
}
