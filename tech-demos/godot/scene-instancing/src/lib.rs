//! Scene Instancing GDExtension demo — load a `.tscn` at runtime and instantiate
//! it from Rust using `ResourceLoader` and `PackedScene`.
//!
//! Demonstrates:
//!
//! - Using `ResourceLoader::singleton()` to load a resource by path.
//! - Casting `Gd<Resource>` to `Gd<PackedScene>` with `try_cast`.
//! - Calling `PackedScene::instantiate()` to get a live node.
//! - Adding the instance as a child of the scene root.
//! - Clearing all instances with `queue_free`.
//! - Enforcing a maximum instance count.

use godot::classes::{INode2D, Label, Node2D, PackedScene, ResourceLoader};
use godot::prelude::*;

// ─── Extension entry point ───────────────────────────────────────────────────

struct SceneInstancingExt;

#[gdextension]
unsafe impl ExtensionLibrary for SceneInstancingExt {}

// ─── Pure functions ───────────────────────────────────────────────────────────

/// Returns `true` if another instance may be spawned.
///
/// # Examples
/// ```
/// assert!(scene_instancing::can_spawn(0, 5));
/// assert!(scene_instancing::can_spawn(4, 5));
/// assert!(!scene_instancing::can_spawn(5, 5));
/// assert!(!scene_instancing::can_spawn(10, 5));
/// ```
pub fn can_spawn(current: i32, max: i32) -> bool {
    current < max
}

/// Returns a label string showing `current / max` instances.
///
/// # Examples
/// ```
/// assert_eq!(scene_instancing::instance_label(0, 5), "Instances: 0 / 5");
/// assert_eq!(scene_instancing::instance_label(3, 5), "Instances: 3 / 5");
/// ```
pub fn instance_label(count: i32, max: i32) -> String {
    format!("Instances: {} / {}", count, max)
}

// ─── SceneInstancer node ──────────────────────────────────────────────────────

/// Scene root that loads and instantiates a `.tscn` file at runtime.
///
/// Call `spawn_scene()` to add one instance of `scene_path` up to `max_instances`
/// times, and `clear_instances()` to remove them all.
///
/// Add a `Label` child named `"Label"` in the Godot editor to see the count.
#[derive(GodotClass)]
#[class(base=Node2D)]
pub struct SceneInstancer {
    /// Path to the scene resource to instantiate.
    #[export]
    scene_path: GString,
    /// Maximum number of simultaneous instances allowed.
    #[export]
    max_instances: i32,
    /// Current number of spawned instances.
    instance_count: i32,
    base: Base<Node2D>,
}

#[godot_api]
impl INode2D for SceneInstancer {
    fn init(base: Base<Node2D>) -> Self {
        Self {
            scene_path: GString::from("res://scenes/spawnable.tscn"),
            max_instances: 5,
            instance_count: 0,
            base,
        }
    }

    fn ready(&mut self) {
        let path = self.scene_path.clone();
        godot_print!("[SceneInstancer] Ready — scene path: {}", path);
        self.update_label();
    }
}

#[godot_api]
impl SceneInstancer {
    /// Loads the scene at `scene_path` and instantiates it as a child, up to
    /// `max_instances` times.
    #[func]
    pub fn spawn_scene(&mut self) {
        let max = self.max_instances;
        if !can_spawn(self.instance_count, max) {
            godot_print!("[SceneInstancer] At max instances ({}).", max);
            return;
        }

        let path = self.scene_path.clone();
        let mut loader = ResourceLoader::singleton();

        let instance_opt = loader
            .load(&path)
            .and_then(|r| r.try_cast::<PackedScene>().ok())
            .and_then(|ps| ps.instantiate());

        if let Some(node) = instance_opt {
            self.base_mut().add_child(&node);
            self.instance_count += 1;
            godot_print!(
                "[SceneInstancer] Spawned instance #{}.",
                self.instance_count
            );
        } else {
            godot_print!(
                "[SceneInstancer] Failed to load or instantiate scene: {}",
                path
            );
        }

        self.update_label();
    }

    /// Removes all child nodes that were added as instances and resets the counter.
    #[func]
    pub fn clear_instances(&mut self) {
        let child_count = self.base().get_child_count();
        let children: Vec<Gd<Node>> = (0..child_count)
            .filter_map(|i| self.base().get_child(i))
            .collect();

        // Only free nodes that are NOT the Label (keep the UI intact).
        for child in children {
            if child.get_class() != "Label" {
                let mut c = child;
                c.queue_free();
            }
        }

        self.instance_count = 0;
        godot_print!("[SceneInstancer] Cleared all instances.");
        self.update_label();
    }

    /// Returns the current number of spawned instances.
    #[func]
    pub fn get_instance_count(&self) -> i32 {
        self.instance_count
    }

    /// Updates the Label child text with the current instance count.
    fn update_label(&mut self) {
        let count = self.instance_count;
        let max = self.max_instances;
        let text = instance_label(count, max);
        if let Some(mut label) = self.base_mut().try_get_node_as::<Label>("Label") {
            label.set_text(text.as_str());
        }
    }
}

// ─── Tests ────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    // can_spawn ───────────────────────────────────────────────────────────────

    #[test]
    fn can_spawn_zero_of_five() {
        assert!(can_spawn(0, 5));
    }

    #[test]
    fn can_spawn_one_below_max() {
        assert!(can_spawn(4, 5));
    }

    #[test]
    fn can_spawn_at_max_is_false() {
        assert!(!can_spawn(5, 5));
    }

    #[test]
    fn can_spawn_over_max_is_false() {
        assert!(!can_spawn(10, 5));
    }

    // instance_label ──────────────────────────────────────────────────────────

    #[test]
    fn instance_label_zero() {
        assert_eq!(instance_label(0, 5), "Instances: 0 / 5");
    }

    #[test]
    fn instance_label_partial() {
        assert_eq!(instance_label(3, 5), "Instances: 3 / 5");
    }

    #[test]
    fn instance_label_at_max() {
        assert_eq!(instance_label(5, 5), "Instances: 5 / 5");
    }

    #[test]
    fn instance_label_different_max() {
        assert_eq!(instance_label(1, 10), "Instances: 1 / 10");
    }

    #[test]
    fn can_spawn_zero_max_is_false() {
        assert!(!can_spawn(0, 0));
    }
}
