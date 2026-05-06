//! Node Lifecycle demo — `_enter_tree`, `_ready`, `_exit_tree` ordering and
//! `NodeNotification` constants observed from Rust.
//!
//! Two classes are defined:
//! - `LifecycleRoot` — the scene root; creates a `LifecycleChild` dynamically
//!   in `ready()` and exposes an event log.
//! - `LifecycleChild` — a plain `Node` subclass that logs its own lifecycle
//!   events.
//!
//! Key ordering rule (demonstrated here):
//!   1. `enter_tree` fires top-down (parent first, then children).
//!   2. `ready` fires bottom-up (children first, then parent).
//!   3. `exit_tree` fires when the node leaves the tree (child before parent
//!      if the parent frees the child).
//!
//! Both classes implement `on_notification` to intercept `ENTER_TREE`,
//! `READY`, and `EXIT_TREE` events.

use godot::classes::notify::NodeNotification;
use godot::classes::{INode, Node};
use godot::prelude::*;

// ---------------------------------------------------------------------------
// Extension entry-point
// ---------------------------------------------------------------------------

struct NodeLifecycleExtension;
#[gdextension]
unsafe impl ExtensionLibrary for NodeLifecycleExtension {}

// ---------------------------------------------------------------------------
// LifecycleChild
// ---------------------------------------------------------------------------

/// A simple child node that records its own lifecycle notifications.
///
/// It does not hold a shared log — it prints to stdout and lets
/// `LifecycleRoot` observe the ordering through its own log.
#[derive(GodotClass)]
#[class(base=Node)]
pub struct LifecycleChild {
    /// Name tag shown in log messages.  Exported so it can be set from the
    /// Godot inspector or from Rust before the node enters the tree.
    #[export]
    child_name: GString,

    base: Base<Node>,
}

#[godot_api]
impl INode for LifecycleChild {
    fn init(base: Base<Node>) -> Self {
        Self {
            child_name: GString::from("LifecycleChild"),
            base,
        }
    }

    fn ready(&mut self) {
        let name = self.child_name.to_string();
        // By the time ready() fires on the child, the parent's enter_tree has
        // already run — but the parent's ready() has NOT yet fired.
        godot_print!(
            "[{}] ready() — parent enter_tree already done; parent ready() NOT yet",
            name
        );
    }

    fn on_notification(&mut self, what: NodeNotification) {
        let name = self.child_name.to_string();
        match what {
            NodeNotification::ENTER_TREE => {
                godot_print!("[{}] ENTER_TREE notification", name);
            }
            NodeNotification::READY => {
                godot_print!("[{}] READY notification", name);
            }
            NodeNotification::EXIT_TREE => {
                godot_print!("[{}] EXIT_TREE notification", name);
            }
            _ => {}
        }
    }
}

#[godot_api]
impl LifecycleChild {}

// ---------------------------------------------------------------------------
// LifecycleRoot
// ---------------------------------------------------------------------------

/// Scene root that creates a `LifecycleChild` dynamically and records all
/// lifecycle events in an ordered log.
///
/// Place this as the scene root with a child `Label` named `"EventLog"`.
#[derive(GodotClass)]
#[class(base=Node)]
pub struct LifecycleRoot {
    /// Chronological log of lifecycle events observed by this node.
    event_log: Vec<String>,

    base: Base<Node>,
}

#[godot_api]
impl INode for LifecycleRoot {
    fn init(base: Base<Node>) -> Self {
        Self {
            event_log: Vec::new(),
            base,
        }
    }

    fn ready(&mut self) {
        self.event_log.push("root: ready()".into());
        godot_print!("[LifecycleRoot] ready() — child ready() has already fired");

        // Dynamically spawn a LifecycleChild to demonstrate the ordering.
        let mut child = Gd::<LifecycleChild>::from_init_fn(|base| LifecycleChild {
            child_name: GString::from("DynamicChild"),
            base,
        });
        child.set_name("DynamicChild");

        let child_node = child.upcast::<Node>();
        self.base_mut().add_child(&child_node);

        self.event_log.push("root: add_child(DynamicChild)".into());
        self.refresh_label();
    }

    fn on_notification(&mut self, what: NodeNotification) {
        match what {
            NodeNotification::ENTER_TREE => {
                self.event_log.push("root: ENTER_TREE".into());
                godot_print!("[LifecycleRoot] ENTER_TREE notification");
            }
            NodeNotification::READY => {
                // READY notification arrives just after ready() — both fire.
                godot_print!("[LifecycleRoot] READY notification");
            }
            NodeNotification::EXIT_TREE => {
                self.event_log.push("root: EXIT_TREE".into());
                godot_print!("[LifecycleRoot] EXIT_TREE notification");
            }
            _ => {}
        }
    }
}

#[godot_api]
impl LifecycleRoot {
    /// Detaches and frees the `DynamicChild` node to trigger its `exit_tree`.
    #[func]
    pub fn remove_child_node(&mut self) {
        if let Some(mut child) = self.base().try_get_node_as::<Node>("DynamicChild") {
            self.base_mut().remove_child(&child);
            child.queue_free();
            self.event_log.push("root: removed DynamicChild".into());
            godot_print!("[LifecycleRoot] DynamicChild removed and queued free");
            self.refresh_label();
        }
    }

    /// Returns the chronological event log as a `GString` array.
    #[func]
    pub fn get_event_log(&self) -> Array<GString> {
        let mut arr = Array::new();
        for entry in &self.event_log {
            let gs = GString::from(entry.as_str());
            arr.push(&gs);
        }
        arr
    }

    // Internal: write the log to the EventLog label.
    fn refresh_label(&mut self) {
        if let Some(mut label) = self.base().try_get_node_as::<godot::classes::Label>("EventLog") {
            let text = format_event_log(&self.event_log);
            label.set_text(text.as_str());
        }
    }
}

// ---------------------------------------------------------------------------
// Pure helper functions
// ---------------------------------------------------------------------------

/// Returns `true` when the event sequence is valid: every `enter_tree` event
/// must appear before every `ready` event in the slice.
///
/// A valid sequence has all `enter_tree` events finishing before `ready` events
/// start (ignoring interleaved events from child/parent).
///
/// # Examples
/// ```
/// let events = vec!["enter_tree", "enter_tree", "ready", "ready"];
/// assert!(node_lifecycle::lifecycle_order_is_valid(&events));
/// let bad = vec!["ready", "enter_tree"];
/// assert!(!node_lifecycle::lifecycle_order_is_valid(&bad));
/// ```
pub fn lifecycle_order_is_valid(events: &[&str]) -> bool {
    let mut seen_ready = false;
    for &e in events {
        if e.contains("ready") {
            seen_ready = true;
        }
        if e.contains("enter_tree") && seen_ready {
            return false;
        }
    }
    true
}

/// Formats the event log into a newline-delimited string with line numbers.
///
/// # Examples
/// ```
/// let events = vec!["root: ENTER_TREE".to_string(), "root: ready()".to_string()];
/// let s = node_lifecycle::format_event_log(&events);
/// assert!(s.contains("1: root: ENTER_TREE"));
/// assert!(s.contains("2: root: ready()"));
/// ```
pub fn format_event_log(events: &[String]) -> String {
    if events.is_empty() {
        return "No events yet.".into();
    }
    events
        .iter()
        .enumerate()
        .map(|(i, e)| format!("{}: {}", i + 1, e))
        .collect::<Vec<_>>()
        .join("\n")
}

/// Returns a short label string for a numeric event type code.
///
/// - `0` → `"enter_tree"`
/// - `1` → `"ready"`
/// - `2` → `"exit_tree"`
/// - anything else → `"unknown"`
///
/// # Examples
/// ```
/// assert_eq!(node_lifecycle::event_label(0), "enter_tree");
/// assert_eq!(node_lifecycle::event_label(1), "ready");
/// assert_eq!(node_lifecycle::event_label(2), "exit_tree");
/// assert_eq!(node_lifecycle::event_label(99), "unknown");
/// ```
pub fn event_label(event_type: u8) -> &'static str {
    match event_type {
        0 => "enter_tree",
        1 => "ready",
        2 => "exit_tree",
        _ => "unknown",
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    // lifecycle_order_is_valid ------------------------------------------------

    #[test]
    fn lifecycle_order_valid_enter_before_ready() {
        let events = vec!["enter_tree", "enter_tree", "ready", "ready"];
        assert!(lifecycle_order_is_valid(&events));
    }

    #[test]
    fn lifecycle_order_invalid_ready_before_enter() {
        let events = vec!["ready", "enter_tree"];
        assert!(!lifecycle_order_is_valid(&events));
    }

    #[test]
    fn lifecycle_order_valid_empty_sequence() {
        assert!(lifecycle_order_is_valid(&[]));
    }

    #[test]
    fn lifecycle_order_valid_only_enter_tree() {
        assert!(lifecycle_order_is_valid(&["enter_tree", "enter_tree"]));
    }

    #[test]
    fn lifecycle_order_valid_only_ready() {
        // No enter_tree at all — trivially valid (nothing to violate the rule).
        assert!(lifecycle_order_is_valid(&["ready", "ready"]));
    }

    // format_event_log --------------------------------------------------------

    #[test]
    fn format_event_log_empty() {
        assert_eq!(format_event_log(&[]), "No events yet.");
    }

    #[test]
    fn format_event_log_single_event() {
        let events = vec!["root: ENTER_TREE".to_string()];
        let s = format_event_log(&events);
        assert!(s.contains("1: root: ENTER_TREE"));
    }

    #[test]
    fn format_event_log_multiple_events_numbered() {
        let events = vec!["root: ENTER_TREE".to_string(), "root: ready()".to_string()];
        let s = format_event_log(&events);
        assert!(s.contains("1: root: ENTER_TREE"));
        assert!(s.contains("2: root: ready()"));
    }

    #[test]
    fn format_event_log_newline_separated() {
        let events = vec!["a".to_string(), "b".to_string()];
        let s = format_event_log(&events);
        assert!(s.contains('\n'));
    }

    // event_label -------------------------------------------------------------

    #[test]
    fn event_label_enter_tree() {
        assert_eq!(event_label(0), "enter_tree");
    }

    #[test]
    fn event_label_ready() {
        assert_eq!(event_label(1), "ready");
    }

    #[test]
    fn event_label_exit_tree() {
        assert_eq!(event_label(2), "exit_tree");
    }

    #[test]
    fn event_label_unknown() {
        assert_eq!(event_label(99), "unknown");
    }
}
