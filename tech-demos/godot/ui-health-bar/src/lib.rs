//! UI Health Bar demo — drives a ProgressBar and Label from Rust, exposing
//! damage() and heal() functions that clamp health to [0, max_health].

use godot::classes::{INode, Label, Node, ProgressBar};
use godot::prelude::*;

struct UiHealthBarExtension;
#[gdextension]
unsafe impl ExtensionLibrary for UiHealthBarExtension {}

#[derive(GodotClass)]
#[class(base=Node)]
struct UiHealthBar {
    #[export]
    max_health: f32,

    current_health: f32,
    base: Base<Node>,
}

#[godot_api]
impl INode for UiHealthBar {
    fn init(base: Base<Node>) -> Self {
        Self {
            max_health: 100.0,
            current_health: 100.0,
            base,
        }
    }

    fn ready(&mut self) {
        let max = self.max_health;
        let current = self.current_health;

        let mut bar = self.base().get_node_as::<ProgressBar>("ProgressBar");
        bar.set_max(max as f64);
        bar.set_value(current as f64);

        let text = health_text(current, max);
        let mut label = self.base().get_node_as::<Label>("Label");
        label.set_text(text.as_str());
    }
}

#[godot_api]
impl UiHealthBar {
    #[func]
    pub fn damage(&mut self, amount: f32) {
        let max = self.max_health;
        self.current_health = clamp_health(self.current_health - amount, max);
        self.update_ui();
    }

    #[func]
    pub fn heal(&mut self, amount: f32) {
        let max = self.max_health;
        self.current_health = clamp_health(self.current_health + amount, max);
        self.update_ui();
    }
}

impl UiHealthBar {
    fn update_ui(&mut self) {
        let current = self.current_health;
        let max = self.max_health;

        let mut bar = self.base().get_node_as::<ProgressBar>("ProgressBar");
        bar.set_value(current as f64);

        let text = health_text(current, max);
        let mut label = self.base().get_node_as::<Label>("Label");
        label.set_text(text.as_str());
    }
}

// ---------------------------------------------------------------------------
// Pure helper functions
// ---------------------------------------------------------------------------

/// Returns health as a fraction in [0.0, 1.0]. Returns 0 if max <= 0.
pub fn health_fraction(current: f32, max: f32) -> f32 {
    if max <= 0.0 {
        return 0.0;
    }
    (current / max).clamp(0.0, 1.0)
}

/// Formats health as "current / max" rounded to nearest integer.
pub fn health_text(current: f32, max: f32) -> String {
    format!("{} / {}", current.round() as i32, max.round() as i32)
}

/// Clamp health to [0, max].
pub fn clamp_health(h: f32, max: f32) -> f32 {
    h.clamp(0.0, max)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn health_fraction_full() {
        assert!((health_fraction(100.0, 100.0) - 1.0).abs() < 1e-6);
    }

    #[test]
    fn health_fraction_empty() {
        assert!((health_fraction(0.0, 100.0) - 0.0).abs() < 1e-6);
    }

    #[test]
    fn health_fraction_half() {
        assert!((health_fraction(50.0, 100.0) - 0.5).abs() < 1e-6);
    }

    #[test]
    fn health_fraction_zero_max_returns_zero() {
        assert!((health_fraction(50.0, 0.0) - 0.0).abs() < 1e-6);
    }

    #[test]
    fn health_fraction_clamps_over_max() {
        assert!((health_fraction(150.0, 100.0) - 1.0).abs() < 1e-6);
    }

    #[test]
    fn health_text_format() {
        assert_eq!(health_text(75.0, 100.0), "75 / 100");
    }

    #[test]
    fn health_text_rounds() {
        assert_eq!(health_text(33.6, 100.0), "34 / 100");
    }

    #[test]
    fn clamp_health_below_zero() {
        assert!((clamp_health(-10.0, 100.0) - 0.0).abs() < 1e-6);
    }

    #[test]
    fn clamp_health_above_max() {
        assert!((clamp_health(150.0, 100.0) - 100.0).abs() < 1e-6);
    }

    #[test]
    fn clamp_health_within_range() {
        assert!((clamp_health(42.0, 100.0) - 42.0).abs() < 1e-6);
    }
}
