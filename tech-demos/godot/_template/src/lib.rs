//! Template demo — the canonical shape every Godot demo in this directory follows.
//!
//! Teaches: nothing on its own. Copy this crate to start a new demo; it is a
//! minimal, working instance of the conventions in
//! [`DEMO_ANATOMY.md`](../DEMO_ANATOMY.md):
//!
//! 1. exactly one `ExtensionLibrary` per crate;
//! 2. `init` sets defaults only — child nodes do not exist yet;
//! 3. `ready` is the first place the scene tree can be touched;
//! 4. `base` is the last field and is always named `base`;
//! 5. logic lives in free `pub fn`s that name no Godot type, so `cargo test`
//!    can exercise it without starting an engine;
//! 6. `#[export]` for what a designer tunes, `#[func]` for what GDScript calls.
//!
//! **Controls:** left/right arrows — move the sprite.

use godot::classes::{INode2D, Input, Node2D};
use godot::prelude::*;

// ─── Extension entry point ────────────────────────────────────────────────────

/// Declares this crate as a GDExtension library. Exactly one per crate.
struct TemplateExt;

#[gdextension]
unsafe impl ExtensionLibrary for TemplateExt {}

// ─── Pure functions ───────────────────────────────────────────────────────────

/// Advances a position by `direction * speed * delta`, clamped to `±limit`.
///
/// This is the part of the demo worth testing, so it takes and returns plain
/// numbers and mentions no Godot type at all.
pub fn advance(position: f32, direction: f32, speed: f32, delta: f64, limit: f32) -> f32 {
    (position + direction * speed * delta as f32).clamp(-limit, limit)
}

/// Maps held left/right inputs to a direction in `-1.0..=1.0`.
///
/// Both or neither held means no movement.
pub fn input_direction(left: bool, right: bool) -> f32 {
    f32::from(right) - f32::from(left)
}

// ─── Template node ────────────────────────────────────────────────────────────

/// A `Node2D` that slides left and right under arrow-key control.
///
/// Attach it as the root node of a scene and run. `speed` and `limit` are
/// editable in the inspector.
#[derive(GodotClass)]
#[class(base=Node2D)]
pub struct TemplateNode {
    /// Movement speed in pixels per second.
    #[export]
    speed: f32,
    /// Maximum distance from the origin, in pixels.
    #[export]
    limit: f32,
    base: Base<Node2D>,
}

#[godot_api]
impl INode2D for TemplateNode {
    fn init(base: Base<Node2D>) -> Self {
        // Defaults only — the scene tree does not exist yet.
        Self {
            speed: 240.0,
            limit: 320.0,
            base,
        }
    }

    fn ready(&mut self) {
        godot_print!("[TemplateNode] Ready — speed {}", self.speed);
    }

    fn process(&mut self, delta: f64) {
        let input = Input::singleton();
        let direction = input_direction(
            input.is_action_pressed("ui_left"),
            input.is_action_pressed("ui_right"),
        );
        if direction == 0.0 {
            return;
        }

        let (speed, limit) = (self.speed, self.limit);
        let mut position = self.base().get_position();
        position.x = advance(position.x, direction, speed, delta, limit);
        self.base_mut().set_position(position);
    }
}

#[godot_api]
impl TemplateNode {
    /// Current horizontal offset from the origin (callable from GDScript).
    #[func]
    pub fn offset(&self) -> f32 {
        self.base().get_position().x
    }
}

// ─── Tests ────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn input_direction_maps_each_combination() {
        assert_eq!(input_direction(false, false), 0.0);
        assert_eq!(input_direction(true, false), -1.0);
        assert_eq!(input_direction(false, true), 1.0);
        // Holding both cancels out rather than favouring one side.
        assert_eq!(input_direction(true, true), 0.0);
    }

    #[test]
    fn advance_moves_by_speed_times_delta() {
        assert_eq!(advance(0.0, 1.0, 100.0, 0.5, 1000.0), 50.0);
        assert_eq!(advance(0.0, -1.0, 100.0, 0.5, 1000.0), -50.0);
    }

    #[test]
    fn advance_does_not_move_without_a_direction() {
        assert_eq!(advance(12.0, 0.0, 100.0, 1.0, 1000.0), 12.0);
    }

    #[test]
    fn advance_clamps_to_the_limit_in_both_directions() {
        assert_eq!(advance(0.0, 1.0, 10_000.0, 1.0, 320.0), 320.0);
        assert_eq!(advance(0.0, -1.0, 10_000.0, 1.0, 320.0), -320.0);
    }
}
