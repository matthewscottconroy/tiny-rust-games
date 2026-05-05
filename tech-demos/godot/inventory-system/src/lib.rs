//! Inventory System demo — Godot 4.3 + gdext 0.5
//!
//! A slot-limited inventory backed by a `Vec<Item>`.  Item lookup, quantity
//! totalling, and capacity checks live in pure functions so they are easily
//! unit-tested without the Godot runtime.

use godot::classes::{INode, Label, Node};
use godot::prelude::*;

// ---------------------------------------------------------------------------
// Extension entry-point
// ---------------------------------------------------------------------------

struct InventorySystemExtension;
#[gdextension]
unsafe impl ExtensionLibrary for InventorySystemExtension {}

// ---------------------------------------------------------------------------
// Pure-Rust types
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, PartialEq)]
pub struct Item {
    pub id: u32,
    pub name: String,
    pub quantity: u32,
}

// ---------------------------------------------------------------------------
// Pure helper functions
// ---------------------------------------------------------------------------

/// Find an item by id in the inventory slice.
pub fn find_item<'a>(inv: &'a [Item], id: u32) -> Option<&'a Item> {
    inv.iter().find(|i| i.id == id)
}

/// Sum all item quantities.
pub fn total_quantity(inv: &[Item]) -> u32 {
    inv.iter().map(|i| i.quantity).sum()
}

/// Return true when the inventory is at or above the maximum slot count.
pub fn is_full(inv: &[Item], max: usize) -> bool {
    inv.len() >= max
}

// ---------------------------------------------------------------------------
// GodotClass
// ---------------------------------------------------------------------------

#[derive(GodotClass)]
#[class(base=Node)]
pub struct InventoryManager {
    inventory: Vec<Item>,
    #[export]
    max_slots: i32,
    base: Base<Node>,
}

#[godot_api]
impl INode for InventoryManager {
    fn init(base: Base<Node>) -> Self {
        Self {
            inventory: Vec::new(),
            max_slots: 10,
            base,
        }
    }

    fn ready(&mut self) {
        self.refresh_label();
    }
}

#[godot_api]
impl InventoryManager {
    /// Add an item; returns false if inventory is full or an item with that id
    /// already exists.
    #[func]
    pub fn add_item(&mut self, id: i32, name: GString, qty: i32) -> bool {
        let uid = id as u32;
        let max = self.max_slots as usize;
        if is_full(&self.inventory, max) {
            return false;
        }
        if find_item(&self.inventory, uid).is_some() {
            return false;
        }
        self.inventory.push(Item {
            id: uid,
            name: name.to_string(),
            quantity: qty.max(0) as u32,
        });
        self.refresh_label();
        true
    }

    /// Remove an item by id; returns false if not found.
    #[func]
    pub fn remove_item(&mut self, id: i32) -> bool {
        let uid = id as u32;
        let before = self.inventory.len();
        self.inventory.retain(|i| i.id != uid);
        let removed = self.inventory.len() < before;
        if removed {
            self.refresh_label();
        }
        removed
    }

    #[func]
    pub fn get_item_count(&self) -> i32 {
        self.inventory.len() as i32
    }

    #[func]
    pub fn has_item(&self, id: i32) -> bool {
        find_item(&self.inventory, id as u32).is_some()
    }

    #[func]
    pub fn get_total_quantity(&self) -> i32 {
        total_quantity(&self.inventory) as i32
    }

    fn refresh_label(&mut self) {
        let lines: Vec<String> = self
            .inventory
            .iter()
            .map(|i| format!("[{}] {} x{}", i.id, i.name, i.quantity))
            .collect();
        let text = if lines.is_empty() {
            "Inventory empty".to_string()
        } else {
            lines.join("\n")
        };
        if let Some(node) = self.base().get_node_or_null("Label") {
            if let Ok(mut label) = node.try_cast::<Label>() {
                label.set_text(text.as_str());
            }
        }
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    fn item(id: u32, name: &str, qty: u32) -> Item {
        Item { id, name: name.to_string(), quantity: qty }
    }

    #[test]
    fn find_item_present() {
        let inv = vec![item(1, "Sword", 1), item(2, "Potion", 5)];
        let found = find_item(&inv, 2).expect("should find");
        assert_eq!(found.name, "Potion");
    }

    #[test]
    fn find_item_absent() {
        let inv = vec![item(1, "Sword", 1)];
        assert!(find_item(&inv, 99).is_none());
    }

    #[test]
    fn total_quantity_empty() {
        assert_eq!(total_quantity(&[]), 0);
    }

    #[test]
    fn total_quantity_multiple() {
        let inv = vec![item(1, "A", 3), item(2, "B", 7)];
        assert_eq!(total_quantity(&inv), 10);
    }

    #[test]
    fn is_full_at_capacity() {
        let inv = vec![item(1, "A", 1), item(2, "B", 1)];
        assert!(is_full(&inv, 2));
    }

    #[test]
    fn is_full_below_capacity() {
        let inv = vec![item(1, "A", 1)];
        assert!(!is_full(&inv, 10));
    }

    #[test]
    fn is_full_empty_max_zero() {
        assert!(is_full(&[], 0));
    }

    #[test]
    fn find_item_first_match() {
        let inv = vec![item(5, "Shield", 2)];
        let found = find_item(&inv, 5).expect("should find");
        assert_eq!(found.quantity, 2);
    }
}
