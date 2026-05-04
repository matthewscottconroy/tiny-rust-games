//! Scene Tree GDExtension demo — navigating the Godot scene tree from Rust.
//!
//! Teaches: getting a child by name with `get_node_as`, iterating all children
//! with `get_children`, reading child names via `Gd<T>` smart pointers, and
//! checking child count with `get_child_count`.  `TreeExplorer` logs every
//! child's name on `ready` and exposes helpers that GDScript can call at
//! runtime to inspect the tree programmatically.

use godot::classes::Label;
use godot::prelude::*;

// ─── Extension entry point ────────────────────────────────────────────────────

/// Declares this crate as a GDExtension library.
struct SceneTreeExt;

#[gdextension]
unsafe impl ExtensionLibrary for SceneTreeExt {}

// ─── TreeExplorer node ────────────────────────────────────────────────────────

/// A Node2D that demonstrates scene-tree navigation from Rust.
///
/// In the editor, place `TreeExplorer` as the root node of a scene and add at
/// least one child named `"Label"`.  On `ready`, the node:
/// 1. Finds the `Label` child by name and sets its text.
/// 2. Prints the total number of children.
/// 3. Iterates all children and prints each one's name.
#[derive(GodotClass)]
#[class(base=Node2D)]
pub struct TreeExplorer {
    base: Base<Node2D>,
}

#[godot_api]
impl INode2D for TreeExplorer {
    fn init(base: Base<Node2D>) -> Self {
        Self { base }
    }

    fn ready(&mut self) {
        // 1. Get the Label child by name and write to it.
        let mut label = self.base().get_node_as::<Label>("Label");
        label.set_text("Rust was here");

        // 2. Print total child count.
        let count = self.base().get_child_count();
        godot_print!("TreeExplorer: {} child(ren)", count);

        // 3. Print each child's name.
        let children = self.base().get_children();
        for child in children.iter_shared() {
            godot_print!("  child: {}", child.get_name());
        }
    }
}

#[godot_api]
impl TreeExplorer {
    /// Returns the number of direct children of this node.
    #[func]
    pub fn count_children(&self) -> i64 {
        self.base().get_child_count() as i64
    }

    /// Returns the name of the child at `index`, or an empty string if the
    /// index is out of range.
    #[func]
    pub fn get_child_name(&self, index: i64) -> GString {
        let count = self.base().get_child_count() as i64;
        if !is_valid_index(index, count) {
            return GString::new();
        }
        self.base()
            .get_child(index as i32)
            .map(|n| GString::from(n.get_name().to_string().as_str()))
            .unwrap_or_default()
    }

    /// Returns `true` if any direct child has a name exactly equal to `name`.
    #[func]
    pub fn has_child_named(&self, name: GString) -> bool {
        let target = name.to_string();
        let children = self.base().get_children();
        for child in children.iter_shared() {
            if child.get_name().to_string() == target {
                return true;
            }
        }
        false
    }
}

// ─── Pure functions ───────────────────────────────────────────────────────────

/// Returns the index of the first element in `names` equal to `target`, or
/// `None` if no such element exists.
pub fn find_by_name<'a>(names: &[&'a str], target: &str) -> Option<usize> {
    names.iter().position(|&n| n == target)
}

/// Returns `true` if `index` is a valid index into a collection of `count`
/// items (i.e. `0 <= index < count`).
pub fn is_valid_index(index: i64, count: i64) -> bool {
    index >= 0 && index < count
}

/// Returns all names from `names` that start with `prefix`.
pub fn filter_by_prefix<'a>(names: &[&'a str], prefix: &str) -> Vec<&'a str> {
    names.iter().copied().filter(|n| n.starts_with(prefix)).collect()
}

// ─── Tests ────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    // ── find_by_name ──────────────────────────────────────────────────────────

    #[test]
    fn find_by_name_returns_correct_index() {
        let names = ["Label", "Sprite2D", "AudioStreamPlayer"];
        assert_eq!(find_by_name(&names, "Sprite2D"), Some(1));
    }

    #[test]
    fn find_by_name_returns_first_match() {
        let names = ["A", "B", "A"];
        assert_eq!(find_by_name(&names, "A"), Some(0));
    }

    #[test]
    fn find_by_name_returns_none_when_absent() {
        let names = ["Label", "Sprite2D"];
        assert_eq!(find_by_name(&names, "Camera2D"), None);
    }

    #[test]
    fn find_by_name_empty_slice() {
        assert_eq!(find_by_name(&[], "Label"), None);
    }

    // ── is_valid_index ────────────────────────────────────────────────────────

    #[test]
    fn is_valid_index_zero_is_valid() {
        assert!(is_valid_index(0, 3));
    }

    #[test]
    fn is_valid_index_last_is_valid() {
        assert!(is_valid_index(2, 3));
    }

    #[test]
    fn is_valid_index_equal_to_count_is_invalid() {
        assert!(!is_valid_index(3, 3));
    }

    #[test]
    fn is_valid_index_negative_is_invalid() {
        assert!(!is_valid_index(-1, 3));
    }

    #[test]
    fn is_valid_index_empty_collection() {
        assert!(!is_valid_index(0, 0));
    }

    // ── filter_by_prefix ──────────────────────────────────────────────────────

    #[test]
    fn filter_by_prefix_matches_multiple() {
        let names = ["Sprite2D", "Sprite3D", "Label", "SpritePart"];
        let result = filter_by_prefix(&names, "Sprite");
        assert_eq!(result, vec!["Sprite2D", "Sprite3D", "SpritePart"]);
    }

    #[test]
    fn filter_by_prefix_no_match_returns_empty() {
        let names = ["Label", "AudioStreamPlayer"];
        assert!(filter_by_prefix(&names, "Camera").is_empty());
    }

    #[test]
    fn filter_by_prefix_empty_prefix_returns_all() {
        let names = ["A", "B", "C"];
        assert_eq!(filter_by_prefix(&names, ""), vec!["A", "B", "C"]);
    }

    #[test]
    fn filter_by_prefix_exact_match() {
        let names = ["Label", "LabelExtra"];
        let result = filter_by_prefix(&names, "Label");
        assert_eq!(result, vec!["Label", "LabelExtra"]);
    }

    #[test]
    fn filter_by_prefix_empty_slice() {
        assert!(filter_by_prefix(&[], "Sprite").is_empty());
    }
}
