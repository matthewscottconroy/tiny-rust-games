//! Scene Manager demo — Godot 4.3 + gdext 0.5
//!
//! `SceneManager` is an Autoload Node that wraps `SceneTree` scene-switching
//! with a breadcrumb history so `go_back()` can navigate to the previous
//! scene.  All path validation lives in pure functions that are fully testable
//! without the Godot runtime.
//!
//! Teaches: an autoload that swaps scenes with `change_scene_to_file` while keeping a
//! breadcrumb of where the player has been.

use godot::classes::{INode, Node};
use godot::prelude::*;

// ---------------------------------------------------------------------------
// Extension entry-point
// ---------------------------------------------------------------------------

struct SceneManagerExtension;
#[gdextension]
unsafe impl ExtensionLibrary for SceneManagerExtension {}

// ---------------------------------------------------------------------------
// Pure helper functions
// ---------------------------------------------------------------------------

/// A valid scene path must be non-empty and end with `.tscn`.
pub fn is_valid_scene_path(path: &str) -> bool {
    !path.is_empty() && path.ends_with(".tscn")
}

/// Return the current depth of the history stack.
pub fn history_depth(history: &[String]) -> usize {
    history.len()
}

// ---------------------------------------------------------------------------
// GodotClass
// ---------------------------------------------------------------------------

/// Autoloaded node that changes scenes and remembers the navigation history.
#[derive(GodotClass)]
#[class(base=Node)]
pub struct SceneManager {
    scene_history: Vec<String>,
    base: Base<Node>,
}

#[godot_api]
impl INode for SceneManager {
    fn init(base: Base<Node>) -> Self {
        Self {
            scene_history: Vec::new(),
            base,
        }
    }
}

#[godot_api]
impl SceneManager {
    /// Navigate to a scene by path, pushing the current scene to history.
    #[func]
    pub fn go_to_scene(&mut self, path: GString) {
        let path_str = path.to_string();
        if !is_valid_scene_path(&path_str) {
            godot_warn!("SceneManager: invalid scene path '{}'", path_str);
            return;
        }
        // Record the current scene file path before switching
        let current = self
            .base()
            .get_tree()
            .get_current_scene()
            .map(|n| n.get_scene_file_path().to_string())
            .unwrap_or_default();
        if !current.is_empty() {
            self.scene_history.push(current);
        }
        self.base_mut().get_tree().change_scene_to_file(&path);
    }

    /// Reload the current scene without modifying history.
    #[func]
    pub fn reload_current(&mut self) {
        self.base_mut().get_tree().reload_current_scene();
    }

    /// Navigate back to the previous scene in history.
    #[func]
    pub fn go_back(&mut self) {
        if let Some(prev) = self.scene_history.pop() {
            let gpath = GString::from(prev.as_str());
            self.base_mut().get_tree().change_scene_to_file(&gpath);
        } else {
            godot_print!("SceneManager: history is empty, cannot go back");
        }
    }

    /// Return the current history depth.
    #[func]
    pub fn get_history_depth(&self) -> i32 {
        self.scene_history.len() as i32
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn valid_tscn_path() {
        assert!(is_valid_scene_path("res://scenes/main.tscn"));
    }

    #[test]
    fn invalid_path_wrong_extension() {
        assert!(!is_valid_scene_path("res://scenes/main.scn"));
    }

    #[test]
    fn invalid_path_empty() {
        assert!(!is_valid_scene_path(""));
    }

    #[test]
    fn invalid_path_no_extension() {
        assert!(!is_valid_scene_path("res://scenes/main"));
    }

    #[test]
    fn history_depth_empty() {
        let h: Vec<String> = vec![];
        assert_eq!(history_depth(&h), 0);
    }

    #[test]
    fn history_depth_multiple() {
        let h = vec!["res://a.tscn".to_string(), "res://b.tscn".to_string()];
        assert_eq!(history_depth(&h), 2);
    }

    #[test]
    fn valid_path_just_filename() {
        assert!(is_valid_scene_path("game.tscn"));
    }
}
