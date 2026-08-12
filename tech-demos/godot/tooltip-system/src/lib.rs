//! Tooltip System demo — shows a PanelContainer with tooltip text on mouse
//! enter and hides it on mouse exit.
//!
//! Teaches: showing and hiding a `PanelContainer` tooltip from hover signals wired up
//! in Rust.

use godot::classes::{Control, IControl, Label, PanelContainer};
use godot::prelude::*;

struct TooltipSystemExtension;
#[gdextension]
unsafe impl ExtensionLibrary for TooltipSystemExtension {}

#[derive(GodotClass)]
#[class(base=Control)]
struct TooltipSystem {
    #[export]
    tooltip_text: GString,

    base: Base<Control>,
}

#[godot_api]
impl IControl for TooltipSystem {
    fn init(base: Base<Control>) -> Self {
        Self {
            tooltip_text: GString::from("Tooltip text"),
            base,
        }
    }

    fn ready(&mut self) {
        // Hide the panel by default
        let mut panel = self.base().get_node_as::<PanelContainer>("PanelContainer");
        panel.hide();

        // Connect mouse signals
        let on_enter = self.base().callable("on_mouse_entered");
        let on_exit = self.base().callable("on_mouse_exited");
        self.base_mut().connect("mouse_entered", &on_enter);
        self.base_mut().connect("mouse_exited", &on_exit);
    }
}

#[godot_api]
impl TooltipSystem {
    #[func]
    pub fn on_mouse_entered(&mut self) {
        let text = self.tooltip_text.clone();
        if should_show(text.to_string().as_str()) {
            let mut panel = self.base().get_node_as::<PanelContainer>("PanelContainer");
            panel.show();

            let mut label = panel.get_node_as::<Label>("Label");
            label.set_text(&text);
        }
    }

    #[func]
    pub fn on_mouse_exited(&mut self) {
        let mut panel = self.base().get_node_as::<PanelContainer>("PanelContainer");
        panel.hide();
    }

    #[func]
    pub fn set_tooltip(&mut self, text: GString) {
        self.tooltip_text = text;
    }
}

// ---------------------------------------------------------------------------
// Pure helper functions
// ---------------------------------------------------------------------------

/// Truncate tooltip text to `max_len` characters, appending "…" if needed.
pub fn truncate_tooltip(text: &str, max_len: usize) -> String {
    if text.len() <= max_len {
        text.to_string()
    } else {
        let mut s = text[..max_len].to_string();
        s.push('…');
        s
    }
}

/// Returns true if the tooltip text is non-empty (worth showing).
pub fn should_show(text: &str) -> bool {
    !text.is_empty()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn should_show_non_empty() {
        assert!(should_show("Hello"));
    }

    #[test]
    fn should_show_empty_returns_false() {
        assert!(!should_show(""));
    }

    #[test]
    fn truncate_within_limit() {
        assert_eq!(truncate_tooltip("Hello", 10), "Hello");
    }

    #[test]
    fn truncate_exactly_at_limit() {
        assert_eq!(truncate_tooltip("Hello", 5), "Hello");
    }

    #[test]
    fn truncate_over_limit_appends_ellipsis() {
        let result = truncate_tooltip("Hello World", 5);
        assert_eq!(result, "Hello…");
    }

    #[test]
    fn truncate_empty_string() {
        assert_eq!(truncate_tooltip("", 10), "");
    }

    #[test]
    fn truncate_max_len_zero() {
        let result = truncate_tooltip("Hello", 0);
        assert_eq!(result, "…");
    }

    #[test]
    fn should_show_whitespace_only() {
        assert!(should_show("   "));
    }
}
