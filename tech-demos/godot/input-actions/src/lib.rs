//! input-actions: Demonstrates polling Godot's Input singleton for action state
//! from Rust. Shows held vs. just-pressed distinction for any configurable action.

use godot::classes::{INode2D, Input, Label, Node2D};
use godot::prelude::*;

struct InputActionsExtension;
#[gdextension]
unsafe impl ExtensionLibrary for InputActionsExtension {}

#[derive(GodotClass)]
#[class(base=Node2D)]
struct InputActions {
    /// The Godot input action to monitor (e.g. "ui_accept", "ui_left").
    #[export]
    action_name: GString,
    base: Base<Node2D>,
}

#[godot_api]
impl INode2D for InputActions {
    fn init(base: Base<Node2D>) -> Self {
        Self {
            action_name: GString::from("ui_accept"),
            base,
        }
    }

    fn ready(&mut self) {
        godot_print!(
            "InputActions ready — monitoring action: {}",
            self.action_name
        );
    }

    fn process(&mut self, _delta: f64) {
        let input = Input::singleton();
        // Convert GString -> StringName for the is_action_* calls
        let action_sn = StringName::from(self.action_name.to_string().as_str());
        let held = input.is_action_pressed(&action_sn);
        let just_pressed = input.is_action_just_pressed(&action_sn);

        let text = describe_state(held, just_pressed);

        // Update child Label if present
        if let Some(mut label) = self.base_mut().try_get_node_as::<Label>("Label") {
            label.set_text(text);
        }
    }
}

#[godot_api]
impl InputActions {
    /// Returns whether the monitored action name is currently valid (non-empty).
    #[func]
    pub fn check_action_valid(&self) -> bool {
        is_valid_action_name(self.action_name.to_string().as_str())
    }
}

// ---------------------------------------------------------------------------
// Pure helpers (no Godot types in signatures)
// ---------------------------------------------------------------------------

/// Human-readable description of the held / just-pressed combination.
pub fn describe_state(held: bool, just_pressed: bool) -> &'static str {
    match (held, just_pressed) {
        (true, true) => "Just Pressed (held)",
        (true, false) => "Held",
        (false, true) => "Just Pressed (released same frame)",
        (false, false) => "Released",
    }
}

/// An action name is valid if it is non-empty and contains only alphanumeric
/// characters, underscores, or hyphens.
pub fn is_valid_action_name(s: &str) -> bool {
    if s.is_empty() {
        return false;
    }
    s.chars()
        .all(|c| c.is_alphanumeric() || c == '_' || c == '-')
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn released_state_when_both_false() {
        assert_eq!(describe_state(false, false), "Released");
    }

    #[test]
    fn held_state_when_held_only() {
        assert_eq!(describe_state(true, false), "Held");
    }

    #[test]
    fn just_pressed_held_when_both_true() {
        assert_eq!(describe_state(true, true), "Just Pressed (held)");
    }

    #[test]
    fn just_pressed_released_same_frame() {
        assert_eq!(
            describe_state(false, true),
            "Just Pressed (released same frame)"
        );
    }

    #[test]
    fn valid_action_name_simple() {
        assert!(is_valid_action_name("ui_accept"));
    }

    #[test]
    fn valid_action_name_with_hyphen() {
        assert!(is_valid_action_name("move-left"));
    }

    #[test]
    fn invalid_action_name_empty() {
        assert!(!is_valid_action_name(""));
    }

    #[test]
    fn invalid_action_name_spaces() {
        assert!(!is_valid_action_name("ui accept"));
    }

    #[test]
    fn invalid_action_name_special_chars() {
        assert!(!is_valid_action_name("ui@accept"));
    }
}
