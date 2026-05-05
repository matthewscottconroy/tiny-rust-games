//! area-detection: Demonstrates connecting Area2D body_entered / body_exited
//! signals to Rust methods. Tracks overlap count and displays it on a Label.

use godot::classes::{Area2D, IArea2D, Label, Node};
use godot::prelude::*;

struct AreaDetectionExtension;
#[gdextension]
unsafe impl ExtensionLibrary for AreaDetectionExtension {}

#[derive(GodotClass)]
#[class(base=Area2D)]
struct AreaDetection {
    overlap_count: i32,
    base: Base<Area2D>,
}

#[godot_api]
impl IArea2D for AreaDetection {
    fn init(base: Base<Area2D>) -> Self {
        Self {
            overlap_count: 0,
            base,
        }
    }

    fn ready(&mut self) {
        let entered = self.base().get_path();
        godot_print!("AreaDetection ready at {:?}", entered);

        // Connect signals. We connect to self so the methods are called on this node.
        let on_entered = Callable::from_object_method(&self.base(), "on_body_entered");
        let on_exited = Callable::from_object_method(&self.base(), "on_body_exited");
        self.base_mut().connect("body_entered", &on_entered);
        self.base_mut().connect("body_exited", &on_exited);
    }

    fn process(&mut self, _delta: f64) {
        let count = self.overlap_count;
        let text = overlap_message(count);
        if let Some(mut label) = self.base_mut().try_get_node_as::<Label>("Label") {
            label.set_text(text.as_str());
        }
    }
}

#[godot_api]
impl AreaDetection {
    /// Signal handler: called when a body enters the area.
    #[func]
    pub fn on_body_entered(&mut self, _body: Gd<Node>) {
        self.overlap_count += 1;
        godot_print!("Body entered — overlap count: {}", self.overlap_count);
    }

    /// Signal handler: called when a body exits the area.
    #[func]
    pub fn on_body_exited(&mut self, _body: Gd<Node>) {
        self.overlap_count = (self.overlap_count - 1).max(0);
        godot_print!("Body exited — overlap count: {}", self.overlap_count);
    }

    /// Exposes the current overlap count to GDScript and the editor.
    #[func]
    pub fn get_overlap_count(&self) -> i32 {
        self.overlap_count
    }
}

// ---------------------------------------------------------------------------
// Pure helpers
// ---------------------------------------------------------------------------

/// Returns a human-readable message for the given overlap count.
pub fn overlap_message(count: i32) -> String {
    match count {
        0 => "No overlaps".to_string(),
        1 => "1 body overlapping".to_string(),
        n => format!("{} bodies overlapping", n),
    }
}

/// Returns true when at least one body overlaps.
pub fn is_overlapping(count: i32) -> bool {
    count > 0
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn no_overlaps_message() {
        assert_eq!(overlap_message(0), "No overlaps");
    }

    #[test]
    fn one_overlap_message() {
        assert_eq!(overlap_message(1), "1 body overlapping");
    }

    #[test]
    fn many_overlaps_message() {
        assert_eq!(overlap_message(5), "5 bodies overlapping");
    }

    #[test]
    fn is_overlapping_zero() {
        assert!(!is_overlapping(0));
    }

    #[test]
    fn is_overlapping_positive() {
        assert!(is_overlapping(3));
    }

    #[test]
    fn is_overlapping_negative_treated_as_none() {
        // Negative counts should never occur in practice but the function is pure.
        assert!(!is_overlapping(-1));
    }
}
