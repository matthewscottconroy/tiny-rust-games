//! Editor Plugin GDExtension demo — a dockable editor panel written entirely
//! in Rust using gdext's `experimental-godot-api` feature.
//!
//! Teaches:
//!
//! - Subclassing `EditorPlugin` and registering it with `#[class(tool)]`.
//! - Using `enter_tree` / `exit_tree` to add/remove an editor dock panel.
//! - Building a simple `VBoxContainer` panel with a `Label` and `Button` child.
//! - Connecting a button's `pressed` signal to a Rust `#[func]` handler.
//! - Pure helper functions tested without needing the Godot runtime.
//!
//! # Runtime verification
//!
//! After running `cargo build` and opening this project in Godot 4.3+ with the
//! plugin enabled (see `addons/rust-panel/plugin.cfg`), the "Rust Panel" dock
//! should appear on the right side of the editor showing a click counter.

use godot::classes::{
    Button, Control, EditorPlugin, IEditorPlugin, Label, VBoxContainer,
};
use godot::prelude::*;

// ─── Extension entry point ────────────────────────────────────────────────────

struct EditorPluginExtension;

#[gdextension]
unsafe impl ExtensionLibrary for EditorPluginExtension {}

// ─── Pure functions ───────────────────────────────────────────────────────────

/// Returns the label text shown in the dock for a given click count.
///
/// # Examples
/// ```
/// assert_eq!(editor_plugin::click_label(0), "Clicks: 0");
/// assert_eq!(editor_plugin::click_label(42), "Clicks: 42");
/// ```
pub fn click_label(count: i32) -> String {
    format!("Clicks: {}", count)
}

/// Returns the static version string for this plugin.
///
/// # Examples
/// ```
/// assert_eq!(editor_plugin::plugin_version(), "0.1.0");
/// ```
pub fn plugin_version() -> &'static str {
    "0.1.0"
}

/// Returns a human-readable name for an internal dock slot index.
///
/// Slot indices mirror `EditorPlugin::DockSlot` ordinals (0-based).
///
/// # Examples
/// ```
/// assert_eq!(editor_plugin::dock_slot_name(0), "LEFT_UL");
/// assert_eq!(editor_plugin::dock_slot_name(255), "UNKNOWN");
/// ```
pub fn dock_slot_name(slot: u8) -> &'static str {
    match slot {
        0 => "LEFT_UL",
        1 => "LEFT_BL",
        2 => "LEFT_UR",
        3 => "LEFT_BR",
        4 => "RIGHT_UL",
        5 => "RIGHT_BL",
        6 => "RIGHT_UR",
        7 => "RIGHT_BR",
        _ => "UNKNOWN",
    }
}

// ─── RustPanel EditorPlugin ───────────────────────────────────────────────────

/// An `EditorPlugin` that adds a dockable panel to the Godot editor.
///
/// The panel contains:
/// - A title `Label` ("Rust Editor Plugin")
/// - A `Button` ("Click Me")
/// - A count `Label` updated on each click
///
/// The dock panel is created in `enter_tree` and removed in `exit_tree`.
#[derive(GodotClass)]
#[class(base=EditorPlugin, tool)]
pub struct RustPanel {
    /// Running count of button presses.
    click_count: i32,

    /// Reference to the dock panel so we can remove it in `exit_tree`.
    panel: Option<Gd<Control>>,

    base: Base<EditorPlugin>,
}

#[godot_api]
impl IEditorPlugin for RustPanel {
    fn init(base: Base<EditorPlugin>) -> Self {
        Self {
            click_count: 0,
            panel: None,
            base,
        }
    }

    fn enter_tree(&mut self) {
        // Build the dock panel.
        let mut vbox = VBoxContainer::new_alloc();

        let mut title = Label::new_alloc();
        title.set_text("Rust Editor Plugin");
        let title_node = title.upcast::<godot::classes::Node>();
        vbox.add_child(&title_node);

        let mut button = Button::new_alloc();
        button.set_text("Click Me");

        // Connect button pressed signal to our handler.
        let callable = self.base().callable("on_button_pressed");
        button.connect("pressed", &callable);

        let button_node = button.upcast::<godot::classes::Node>();
        vbox.add_child(&button_node);

        let mut count_label = Label::new_alloc();
        count_label.set_name("CountLabel");
        count_label.set_text(click_label(0).as_str());
        let count_node = count_label.upcast::<godot::classes::Node>();
        vbox.add_child(&count_node);

        // Cast to Control for add_control_to_dock.
        let panel: Gd<Control> = vbox.upcast::<Control>();

        self.base_mut().add_control_to_dock(
            godot::classes::editor_plugin::DockSlot::RIGHT_UL,
            &panel,
        );

        self.panel = Some(panel);

        godot_print!(
            "[RustPanel] Plugin v{} entered tree — dock added.",
            plugin_version()
        );
    }

    fn exit_tree(&mut self) {
        if let Some(panel) = self.panel.take() {
            self.base_mut().remove_control_from_docks(&panel);
            godot_print!("[RustPanel] Dock removed.");
        }
    }
}

#[godot_api]
impl RustPanel {
    /// Increments the click counter and updates the label in the dock.
    #[func]
    pub fn on_button_pressed(&mut self) {
        self.click_count += 1;
        let count = self.click_count;
        let label_text = click_label(count);

        if let Some(panel) = &self.panel {
            if let Some(mut label) =
                panel.clone().try_cast::<VBoxContainer>().ok().and_then(|vb| {
                    vb.try_get_node_as::<Label>("CountLabel")
                })
            {
                label.set_text(label_text.as_str());
            }
        }

        godot_print!("[RustPanel] Click #{}", count);
    }
}

// ─── Tests ────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn click_label_zero() {
        assert_eq!(click_label(0), "Clicks: 0");
    }

    #[test]
    fn click_label_positive() {
        assert_eq!(click_label(42), "Clicks: 42");
    }

    #[test]
    fn plugin_version_is_semver() {
        let v = plugin_version();
        assert!(v.contains('.'), "expected semver: {v}");
    }

    #[test]
    fn dock_slot_name_known() {
        assert_eq!(dock_slot_name(4), "RIGHT_UL");
        assert_eq!(dock_slot_name(0), "LEFT_UL");
    }

    #[test]
    fn dock_slot_name_unknown() {
        assert_eq!(dock_slot_name(255), "UNKNOWN");
    }
}
