//! collision-layers: Demonstrates programmatic control of Godot collision layers
//! and masks from Rust. Spawns StaticBody2D children and configures their
//! layer / mask bits at runtime.

use godot::classes::{INode2D, Label, Node2D, StaticBody2D};
use godot::prelude::*;

struct CollisionLayersExtension;
#[gdextension]
unsafe impl ExtensionLibrary for CollisionLayersExtension {}

/// Number of StaticBody2D bodies spawned for demonstration.
const BODY_COUNT: usize = 4;

#[derive(GodotClass)]
#[class(base=Node2D)]
struct CollisionLayers {
    /// Collision layer bitmasks for each spawned body (plain u32, no Godot types).
    layer_masks: [u32; BODY_COUNT],
    base: Base<Node2D>,
}

#[godot_api]
impl INode2D for CollisionLayers {
    fn init(base: Base<Node2D>) -> Self {
        Self {
            // Default: body i is on layer (i+1)
            layer_masks: [1, 2, 4, 8],
            base,
        }
    }

    fn ready(&mut self) {
        // Spawn StaticBody2D children and apply layer / mask
        for i in 0..BODY_COUNT {
            let mut body = StaticBody2D::new_alloc();
            body.set_collision_layer(self.layer_masks[i]);
            // Each body detects everything on layer 1
            body.set_collision_mask(1);
            body.set_position(Vector2::new(i as f32 * 80.0, 0.0));
            let b = body.clone();
            self.base_mut().add_child(&b);
        }
        godot_print!("CollisionLayers ready — {} bodies spawned", BODY_COUNT);
        self.update_label();
    }

    fn process(&mut self, _delta: f64) {
        // Nothing to update every frame; label is refreshed when layers change.
    }
}

#[godot_api]
impl CollisionLayers {
    /// Toggle a single collision layer bit on a spawned body.
    #[func]
    pub fn set_layer_bit(&mut self, body_index: i32, bit: i32, enabled: bool) {
        if body_index < 0 || body_index as usize >= BODY_COUNT {
            return;
        }
        let idx = body_index as usize;
        self.layer_masks[idx] = set_bit(self.layer_masks[idx], bit as u32, enabled);

        // Copy the new mask to a local before borrowing base_mut().
        let new_mask = self.layer_masks[idx];
        let child_index = body_index; // children were added in order
        if let Some(mut child) = self
            .base_mut()
            .try_get_node_as::<StaticBody2D>(&format!("@StaticBody2D@{}", child_index))
        {
            child.set_collision_layer(new_mask);
        }
        self.update_label();
    }

    fn update_label(&mut self) {
        let text: String = self
            .layer_masks
            .iter()
            .enumerate()
            .map(|(i, &m)| format!("Body {}: {}", i, mask_to_binary_string(m)))
            .collect::<Vec<_>>()
            .join("\n");

        if let Some(mut label) = self.base_mut().try_get_node_as::<Label>("Label") {
            label.set_text(text.as_str());
        }
    }
}

// ---------------------------------------------------------------------------
// Pure helpers
// ---------------------------------------------------------------------------

/// Set or clear bit `bit` (0-indexed) in `mask`.
pub fn set_bit(mask: u32, bit: u32, enabled: bool) -> u32 {
    if bit >= 32 {
        return mask;
    }
    if enabled {
        mask | (1 << bit)
    } else {
        mask & !(1 << bit)
    }
}

/// Format a u32 bitmask as a 32-character binary string, MSB first.
pub fn mask_to_binary_string(mask: u32) -> String {
    format!("{:032b}", mask)
}

/// Returns true if two layers share at least one bit (i.e. they can interact).
pub fn layers_overlap(a: u32, b: u32) -> bool {
    (a & b) != 0
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn set_bit_enables() {
        assert_eq!(set_bit(0, 0, true), 1);
        assert_eq!(set_bit(0, 1, true), 2);
        assert_eq!(set_bit(0, 3, true), 8);
    }

    #[test]
    fn set_bit_disables() {
        assert_eq!(set_bit(0b111, 1, false), 0b101);
    }

    #[test]
    fn set_bit_idempotent_enable() {
        assert_eq!(set_bit(1, 0, true), 1);
    }

    #[test]
    fn set_bit_out_of_range_is_noop() {
        assert_eq!(set_bit(0xFF, 32, true), 0xFF);
    }

    #[test]
    fn mask_to_binary_string_zero() {
        assert_eq!(mask_to_binary_string(0), "00000000000000000000000000000000");
    }

    #[test]
    fn mask_to_binary_string_one() {
        assert!(mask_to_binary_string(1).ends_with('1'));
        assert_eq!(mask_to_binary_string(1).len(), 32);
    }

    #[test]
    fn layers_overlap_shared_bit() {
        assert!(layers_overlap(0b1010, 0b0010));
    }

    #[test]
    fn layers_no_overlap() {
        assert!(!layers_overlap(0b1010, 0b0101));
    }

    #[test]
    fn layers_overlap_zero() {
        assert!(!layers_overlap(0, 0));
    }
}
