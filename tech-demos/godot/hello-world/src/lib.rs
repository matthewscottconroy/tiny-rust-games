//! Hello World GDExtension demo — the minimum viable Rust extension for Godot 4.
//!
//! Teaches: extension library declaration, a single Node2D subclass, an
//! `#[export]` field visible in the Godot inspector, the `ready()` lifecycle
//! hook, and how to reach a child node from Rust.

use godot::classes::Label;
use godot::prelude::*;

// ─── Extension entry point ───────────────────────────────────────────────────

/// Declares this crate as a GDExtension library.
struct HelloWorldExt;

#[gdextension]
unsafe impl ExtensionLibrary for HelloWorldExt {}

// ─── HelloWorld node ─────────────────────────────────────────────────────────

/// A Node2D that prints a configurable greeting and sets a child Label's text.
///
/// Add this as the root node of a scene, place a `Label` child named `"Label"`,
/// then run the scene to see the greeting in the Godot output panel and in the
/// Label on screen.
#[derive(GodotClass)]
#[class(base=Node2D)]
pub struct HelloWorld {
    /// The greeting shown in the Godot console and on the Label child.
    #[export]
    greeting: GString,
    base: Base<Node2D>,
}

#[godot_api]
impl INode2D for HelloWorld {
    fn init(base: Base<Node2D>) -> Self {
        Self {
            greeting: GString::from("Hello from Rust!"),
            base,
        }
    }

    fn ready(&mut self) {
        let greeting = self.greeting.clone();
        godot_print!("{}", greeting);

        let mut label = self.base().get_node_as::<Label>("Label");
        label.set_text(&greeting);
    }

    fn process(&mut self, _delta: f64) {
        // Nothing to do each frame; hook shown for reference.
    }
}

#[godot_api]
impl HelloWorld {
    /// Returns the current greeting string (callable from GDScript as `greeting_text()`).
    #[func]
    pub fn greeting_text(&self) -> GString {
        self.greeting.clone()
    }
}

// ─── Pure functions ───────────────────────────────────────────────────────────

/// Formats a personalised greeting: `"Hello, {name}! (from Rust)"`.
pub fn format_greeting(name: &str) -> String {
    format!("Hello, {name}! (from Rust)")
}

/// Returns `true` if `s` is non-empty after trimming ASCII whitespace.
pub fn is_valid_greeting(s: &str) -> bool {
    !s.trim().is_empty()
}

// ─── Tests ────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn format_greeting_includes_name() {
        assert!(format_greeting("Alice").contains("Alice"));
    }

    #[test]
    fn format_greeting_includes_from_rust() {
        assert!(format_greeting("Bob").contains("from Rust"));
    }

    #[test]
    fn format_greeting_structure() {
        assert_eq!(format_greeting("World"), "Hello, World! (from Rust)");
    }

    #[test]
    fn is_valid_greeting_rejects_empty() {
        assert!(!is_valid_greeting(""));
    }

    #[test]
    fn is_valid_greeting_rejects_whitespace_only() {
        assert!(!is_valid_greeting("   "));
        assert!(!is_valid_greeting("\t\n"));
    }

    #[test]
    fn is_valid_greeting_accepts_normal_text() {
        assert!(is_valid_greeting("Hello!"));
        assert!(is_valid_greeting("  hi  "));
    }

    #[test]
    fn format_greeting_empty_name() {
        let result = format_greeting("");
        assert!(result.contains("Hello,"));
        assert!(result.contains("(from Rust)"));
    }
}
