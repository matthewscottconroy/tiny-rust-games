//! Lighting 2D GDExtension demo — drives PointLight2D energy, color, and
//! texture_scale from Rust with a pulsing sine-wave effect each frame.
//!
//! Teaches:
//!
//! - Fetching a `PointLight2D` child via `get_node_as` in `ready()`.
//! - Animating light energy each frame using a sine-wave accumulator.
//! - Lerping colors between warm orange and cool blue based on elapsed time.
//! - Exposing toggle / color controls via `#[func]` for GDScript callers.
//! - Pure math helpers (`pulse_energy`, `lerp_color`, `energy_to_label`) that
//!   are covered by unit tests without needing the Godot runtime.
//!
//! Counterpart: tech-demos/bevy/lighting-2d — the same concept in Bevy.

use godot::classes::{INode2D, Node2D, PointLight2D};
use godot::prelude::*;

// ─── Extension entry point ────────────────────────────────────────────────────

struct Lighting2DExt;

#[gdextension]
unsafe impl ExtensionLibrary for Lighting2DExt {}

// ─── Pure functions ───────────────────────────────────────────────────────────

/// Computes a pulsed energy value using a sine wave.
///
/// Returns `base + amplitude * sin(elapsed * speed)`, which oscillates between
/// `base - amplitude` and `base + amplitude`.
///
/// # Examples
/// ```
/// // At elapsed=0 the sine is 0, so result equals base.
/// let e = lighting_2d::pulse_energy(1.0, 0.3, 0.0, 2.0);
/// assert!((e - 1.0).abs() < 1e-5);
/// ```
pub fn pulse_energy(base: f32, amp: f32, elapsed: f64, speed: f32) -> f32 {
    base + amp * ((elapsed as f32) * speed).sin()
}

/// Linearly interpolates between two RGB triples.
///
/// `t = 0.0` returns `a`; `t = 1.0` returns `b`; values outside `[0, 1]` are
/// not clamped by this function.
///
/// # Examples
/// ```
/// let c = lighting_2d::lerp_color((1.0, 0.0, 0.0), (0.0, 0.0, 1.0), 0.5);
/// assert!((c.0 - 0.5).abs() < 1e-5);
/// assert!(c.1.abs() < 1e-5);
/// assert!((c.2 - 0.5).abs() < 1e-5);
/// ```
pub fn lerp_color(a: (f32, f32, f32), b: (f32, f32, f32), t: f32) -> (f32, f32, f32) {
    (
        a.0 + (b.0 - a.0) * t,
        a.1 + (b.1 - a.1) * t,
        a.2 + (b.2 - a.2) * t,
    )
}

/// Returns a short human-readable label for an energy value.
///
/// ```
/// assert_eq!(lighting_2d::energy_to_label(0.0), "Off");
/// assert_eq!(lighting_2d::energy_to_label(0.5), "Dim");
/// assert_eq!(lighting_2d::energy_to_label(1.2), "Bright");
/// ```
pub fn energy_to_label(energy: f32) -> String {
    if energy <= 0.05 {
        "Off".to_string()
    } else if energy < 1.0 {
        "Dim".to_string()
    } else {
        "Bright".to_string()
    }
}

// ─── LightingDemo node ────────────────────────────────────────────────────────

/// A `Node2D` that drives a `PointLight2D` child with a sine-wave pulse.
///
/// Scene layout expected:
/// ```text
/// LightingDemo (this class)
/// └── PointLight2D
/// ```
#[derive(GodotClass)]
#[class(base=Node2D)]
pub struct LightingDemo {
    /// Oscillation speed in radians per second.
    #[export]
    pulse_speed: f32,

    /// Half-amplitude of the energy oscillation.
    #[export]
    pulse_amplitude: f32,

    /// Centre energy value around which the pulse oscillates.
    #[export]
    base_energy: f32,

    /// Accumulated time in seconds used as the sine argument.
    elapsed: f64,

    base: Base<Node2D>,
}

#[godot_api]
impl INode2D for LightingDemo {
    fn init(base: Base<Node2D>) -> Self {
        Self {
            pulse_speed: 2.0,
            pulse_amplitude: 0.3,
            base_energy: 1.0,
            elapsed: 0.0,
            base,
        }
    }

    fn ready(&mut self) {
        let base_energy = self.base_energy;

        let mut light = self.base().get_node_as::<PointLight2D>("PointLight2D");
        light.set_energy(base_energy);
        light.set_color(Color::from_rgb(1.0, 0.7, 0.3)); // warm orange
        light.set_texture_scale(2.0);

        godot_print!(
            "[LightingDemo] Ready — base_energy={}, pulse_speed={}, pulse_amplitude={}",
            base_energy,
            self.pulse_speed,
            self.pulse_amplitude
        );
    }

    fn process(&mut self, delta: f64) {
        self.elapsed += delta;

        let elapsed = self.elapsed;
        let speed = self.pulse_speed;
        let amp = self.pulse_amplitude;
        let base_e = self.base_energy;

        let pulsed = pulse_energy(base_e, amp, elapsed, speed);

        // Lerp color: warm orange (1, 0.7, 0.3) → cool blue (0.3, 0.5, 1.0)
        // t oscillates in [0, 1] based on sine.
        let t = ((elapsed as f32 * speed * 0.5).sin() + 1.0) * 0.5;
        let (r, g, b) = lerp_color((1.0, 0.7, 0.3), (0.3, 0.5, 1.0), t);

        let mut light = self.base().get_node_as::<PointLight2D>("PointLight2D");
        light.set_energy(pulsed);
        light.set_color(Color::from_rgb(r, g, b));
    }
}

#[godot_api]
impl LightingDemo {
    /// Sets the light color directly (callable from GDScript).
    #[func]
    pub fn set_light_color(&mut self, r: f32, g: f32, b: f32) {
        let mut light = self.base().get_node_as::<PointLight2D>("PointLight2D");
        light.set_color(Color::from_rgb(r, g, b));
    }

    /// Toggles the `PointLight2D` enabled state.
    #[func]
    pub fn toggle_light(&mut self) {
        let mut light = self.base().get_node_as::<PointLight2D>("PointLight2D");
        let currently_visible = light.is_visible();
        light.set_visible(!currently_visible);
    }
}

// ─── Tests ────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    // pulse_energy ─────────────────────────────────────────────────────────────

    #[test]
    fn pulse_energy_at_zero_elapsed_equals_base() {
        let e = pulse_energy(1.0, 0.3, 0.0, 2.0);
        assert!((e - 1.0).abs() < 1e-5, "expected 1.0 got {e}");
    }

    #[test]
    fn pulse_energy_max_at_quarter_period() {
        // sin(pi/2) = 1.0 → energy = base + amp
        use std::f32::consts::PI;
        let elapsed = (PI / 2.0 / 2.0) as f64; // speed=2, quarter period
        let e = pulse_energy(1.0, 0.3, elapsed, 2.0);
        assert!((e - 1.3).abs() < 1e-5, "expected 1.3 got {e}");
    }

    #[test]
    fn pulse_energy_min_at_three_quarter_period() {
        use std::f32::consts::PI;
        let elapsed = (3.0 * PI / 2.0 / 2.0) as f64;
        let e = pulse_energy(1.0, 0.3, elapsed, 2.0);
        assert!((e - 0.7).abs() < 1e-5, "expected 0.7 got {e}");
    }

    #[test]
    fn pulse_energy_amplitude_zero_is_flat() {
        let e = pulse_energy(0.8, 0.0, 999.0, 5.0);
        assert!((e - 0.8).abs() < 1e-5);
    }

    // lerp_color ───────────────────────────────────────────────────────────────

    #[test]
    fn lerp_color_at_zero_returns_a() {
        let c = lerp_color((1.0, 0.0, 0.0), (0.0, 1.0, 1.0), 0.0);
        assert!((c.0 - 1.0).abs() < 1e-6);
        assert!(c.1.abs() < 1e-6);
        assert!(c.2.abs() < 1e-6);
    }

    #[test]
    fn lerp_color_at_one_returns_b() {
        let c = lerp_color((1.0, 0.0, 0.0), (0.0, 1.0, 1.0), 1.0);
        assert!(c.0.abs() < 1e-6);
        assert!((c.1 - 1.0).abs() < 1e-6);
        assert!((c.2 - 1.0).abs() < 1e-6);
    }

    #[test]
    fn lerp_color_midpoint() {
        let c = lerp_color((1.0, 0.0, 0.0), (0.0, 0.0, 1.0), 0.5);
        assert!((c.0 - 0.5).abs() < 1e-5);
        assert!(c.1.abs() < 1e-6);
        assert!((c.2 - 0.5).abs() < 1e-5);
    }

    // energy_to_label ──────────────────────────────────────────────────────────

    #[test]
    fn energy_to_label_zero_is_off() {
        assert_eq!(energy_to_label(0.0), "Off");
    }

    #[test]
    fn energy_to_label_dim() {
        assert_eq!(energy_to_label(0.5), "Dim");
    }

    #[test]
    fn energy_to_label_bright_at_one() {
        assert_eq!(energy_to_label(1.0), "Bright");
    }

    #[test]
    fn energy_to_label_bright_above_one() {
        assert_eq!(energy_to_label(1.5), "Bright");
    }
}
