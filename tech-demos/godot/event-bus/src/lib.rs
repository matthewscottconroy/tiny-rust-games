//! Event Bus demo — Godot 4.3 + gdext 0.5
//!
//! `EventBus` is an Autoload Node that declares custom signals and provides
//! thin `#[func]` wrappers that emit them.  Other nodes connect to the bus in
//! `ready()` rather than directly to each other, keeping them fully decoupled.
//!
//! `EventListener` connects to all three signals in `ready()` and prints to
//! the Godot console when they fire.

use godot::classes::{INode, Node};
use godot::prelude::*;

// ---------------------------------------------------------------------------
// Extension entry-point
// ---------------------------------------------------------------------------

struct EventBusExtension;
#[gdextension]
unsafe impl ExtensionLibrary for EventBusExtension {}

// ---------------------------------------------------------------------------
// Pure helper functions
// ---------------------------------------------------------------------------

/// Build a human-readable event string.
pub fn format_event(event_type: u8, value: i32) -> String {
    let name = match event_type {
        0 => "player_died",
        1 => "item_collected",
        2 => "level_completed",
        _ => "unknown_event",
    };
    format!("[{}] value={}", name, value)
}

/// Higher number = higher priority.
pub fn event_priority(event_type: u8) -> u8 {
    match event_type {
        0 => 3, // player_died — critical
        1 => 1, // item_collected — low
        2 => 2, // level_completed — medium
        _ => 0,
    }
}

// ---------------------------------------------------------------------------
// EventBus
// ---------------------------------------------------------------------------

#[derive(GodotClass)]
#[class(base=Node)]
pub struct EventBus {
    base: Base<Node>,
}

#[godot_api]
impl INode for EventBus {
    fn init(base: Base<Node>) -> Self {
        Self { base }
    }
}

#[godot_api]
impl EventBus {
    #[signal]
    fn player_died(score: i32);

    #[signal]
    fn item_collected(item_id: i32);

    #[signal]
    fn level_completed(level: i32);

    #[func]
    pub fn emit_player_died(&mut self, score: i32) {
        let score_copy = score;
        self.base_mut()
            .emit_signal("player_died", &[score_copy.to_variant()]);
    }

    #[func]
    pub fn emit_item_collected(&mut self, item_id: i32) {
        let id_copy = item_id;
        self.base_mut()
            .emit_signal("item_collected", &[id_copy.to_variant()]);
    }

    #[func]
    pub fn emit_level_completed(&mut self, level: i32) {
        let level_copy = level;
        self.base_mut()
            .emit_signal("level_completed", &[level_copy.to_variant()]);
    }
}

// ---------------------------------------------------------------------------
// EventListener
// ---------------------------------------------------------------------------

#[derive(GodotClass)]
#[class(base=Node)]
pub struct EventListener {
    base: Base<Node>,
}

#[godot_api]
impl INode for EventListener {
    fn init(base: Base<Node>) -> Self {
        Self { base }
    }

    fn ready(&mut self) {
        // Connect to the EventBus signals if the bus is present in the tree.
        if let Some(bus_node) = self.base().get_node_or_null("/root/EventBus") {
            if let Ok(mut bus) = bus_node.try_cast::<EventBus>() {
                let on_died = self.base().callable("on_player_died");
                let on_item = self.base().callable("on_item_collected");
                let on_level = self.base().callable("on_level_completed");
                bus.connect("player_died", &on_died);
                bus.connect("item_collected", &on_item);
                bus.connect("level_completed", &on_level);
            }
        }
    }
}

#[godot_api]
impl EventListener {
    #[func]
    pub fn on_player_died(&self, score: i32) {
        godot_print!("{}", format_event(0, score));
    }

    #[func]
    pub fn on_item_collected(&self, item_id: i32) {
        godot_print!("{}", format_event(1, item_id));
    }

    #[func]
    pub fn on_level_completed(&self, level: i32) {
        godot_print!("{}", format_event(2, level));
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn format_event_player_died() {
        assert_eq!(format_event(0, 42), "[player_died] value=42");
    }

    #[test]
    fn format_event_item_collected() {
        assert_eq!(format_event(1, 7), "[item_collected] value=7");
    }

    #[test]
    fn format_event_level_completed() {
        assert_eq!(format_event(2, 3), "[level_completed] value=3");
    }

    #[test]
    fn format_event_unknown() {
        assert!(format_event(99, 0).contains("unknown_event"));
    }

    #[test]
    fn priority_ordering() {
        // player_died > level_completed > item_collected
        assert!(event_priority(0) > event_priority(2));
        assert!(event_priority(2) > event_priority(1));
    }

    #[test]
    fn priority_unknown_is_lowest() {
        assert_eq!(event_priority(99), 0);
    }
}
