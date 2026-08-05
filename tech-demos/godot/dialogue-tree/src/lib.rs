//! Branching dialogue tree demo — Godot 4.3 + gdext 0.5.
//!
//! Teaches:
//! - Representing a dialogue tree as a flat Vec of nodes with index-based edges.
//! - Pure functions for tree traversal decoupled from Godot types.
//! - Rendering dialogue and numbered choices via Label nodes created from Rust.
//! - Keyboard navigation (1–4 to pick choices, ESC to restart).
//!
//! **Controls:** 1–4 — select choice;  ESC — restart dialogue.

use godot::classes::{INode2D, Input, Label, Node2D};
use godot::prelude::*;

// ─── Extension entry point ────────────────────────────────────────────────────

struct DialogueTreeExt;
#[gdextension]
unsafe impl ExtensionLibrary for DialogueTreeExt {}

// ─── Data model ───────────────────────────────────────────────────────────────

/// A single node in the dialogue tree.
#[derive(Clone)]
pub struct DialogueNode {
    /// The line of text spoken at this node.
    pub text: String,
    /// `(choice_label, target_node_index)` pairs.  Empty = terminal node.
    pub choices: Vec<(String, usize)>,
}

impl DialogueNode {
    pub fn new(text: impl Into<String>, choices: Vec<(&'static str, usize)>) -> Self {
        Self {
            text: text.into(),
            choices: choices
                .into_iter()
                .map(|(s, i)| (s.to_string(), i))
                .collect(),
        }
    }
}

// ─── Pure tree helpers ────────────────────────────────────────────────────────

/// Returns the text at `index` in `tree`, or `""` when out of bounds.
pub fn node_text<'a>(tree: &'a [DialogueNode], index: usize) -> &'a str {
    tree.get(index).map(|n| n.text.as_str()).unwrap_or("")
}

/// How many choices the node at `index` has.
pub fn choice_count(tree: &[DialogueNode], index: usize) -> usize {
    tree.get(index).map(|n| n.choices.len()).unwrap_or(0)
}

/// `true` when the node has at least one choice (i.e., is not a terminal node).
pub fn can_advance(tree: &[DialogueNode], index: usize) -> bool {
    choice_count(tree, index) > 0
}

/// Returns the target node index for choice `choice` at node `index`,
/// or `None` when the choice or node is out of range.
pub fn next_node_index(
    tree: &[DialogueNode],
    index: usize,
    choice: usize,
) -> Option<usize> {
    tree.get(index)
        .and_then(|n| n.choices.get(choice))
        .map(|(_, target)| *target)
}

// ─── Hardcoded sample tree ────────────────────────────────────────────────────

fn build_tree() -> Vec<DialogueNode> {
    vec![
        // 0 — opening
        DialogueNode::new(
            "A hooded stranger blocks your path.\n\"State your business.\"",
            vec![("\"I seek the lost relic.\"", 1), ("\"Just passing through.\"", 2)],
        ),
        // 1 — relic path
        DialogueNode::new(
            "The stranger's eyes narrow.\n\"Many have tried. Most didn't return.\"\n\"Are you certain?\"",
            vec![("\"Yes, absolutely.\"", 3), ("\"Perhaps not...\"", 4)],
        ),
        // 2 — passing through
        DialogueNode::new(
            "\"Then be on your way.\"\nThe stranger steps aside.",
            vec![("Continue walking.", 5)],
        ),
        // 3 — accept quest
        DialogueNode::new(
            "\"Then take this map.\"\nYou receive the Ancient Map.\n[Quest started!]",
            vec![],
        ),
        // 4 — back down
        DialogueNode::new(
            "\"Wise choice.\"\nThe stranger moves on.",
            vec![],
        ),
        // 5 — end of passing through
        DialogueNode::new(
            "You walk past without incident.\nThe road stretches ahead.",
            vec![],
        ),
    ]
}

// ─── DialogueManager — root Node2D ────────────────────────────────────────────

#[derive(GodotClass)]
#[class(base=Node2D)]
pub struct DialogueManager {
    tree: Vec<DialogueNode>,
    current: usize,
    base: Base<Node2D>,
}

#[godot_api]
impl INode2D for DialogueManager {
    fn init(base: Base<Node2D>) -> Self {
        Self {
            tree: build_tree(),
            current: 0,
            base,
        }
    }

    fn ready(&mut self) {
        // Instructions label
        let mut hint = Label::new_alloc();
        hint.set_name("Hint");
        hint.set_text("1–4: choose   ESC: restart");
        hint.set_position(Vector2::new(10.0, 8.0));
        self.base_mut().add_child(&hint);

        // Dialogue text label
        let mut text_label = Label::new_alloc();
        text_label.set_name("DialogueText");
        text_label.set_position(Vector2::new(40.0, 60.0));
        self.base_mut().add_child(&text_label);

        // Choice labels (up to 4)
        for i in 0..4usize {
            let mut cl = Label::new_alloc();
            let name = format!("Choice{}", i);
            cl.set_name(name.as_str());
            cl.set_position(Vector2::new(60.0, 160.0 + i as f32 * 30.0));
            self.base_mut().add_child(&cl);
        }

        self.refresh_ui();
    }

    fn process(&mut self, _delta: f64) {
        let input = Input::singleton();

        if input.is_action_just_pressed("ui_cancel") {
            self.current = 0;
            self.refresh_ui();
            return;
        }

        let n_choices = choice_count(&self.tree, self.current);
        let action_names = ["ui_text_indent", "ui_page_up", "ui_page_down", "ui_home"];
        // Map 1–4 keys via their action names; fall back to numeric check.
        for (i, &action) in action_names.iter().enumerate().take(n_choices) {
            if input.is_action_just_pressed(action) {
                if let Some(target) = next_node_index(&self.tree, self.current, i) {
                    self.current = target;
                    self.refresh_ui();
                    return;
                }
            }
        }
    }
}

#[godot_api]
impl DialogueManager {
    fn refresh_ui(&mut self) {
        let text = node_text(&self.tree, self.current).to_string();
        if let Some(mut lbl) = self.base().try_get_node_as::<Label>("DialogueText") {
            lbl.set_text(&GString::from(text.as_str()));
        }

        let choices: Vec<String> = self
            .tree
            .get(self.current)
            .map(|n| {
                n.choices
                    .iter()
                    .enumerate()
                    .map(|(i, (label, _))| format!("[{}] {}", i + 1, label))
                    .collect()
            })
            .unwrap_or_default();

        for i in 0..4usize {
            let node_name = format!("Choice{}", i);
            if let Some(mut cl) =
                self.base().try_get_node_as::<Label>(node_name.as_str())
            {
                if i < choices.len() {
                    cl.set_text(&GString::from(choices[i].as_str()));
                } else {
                    cl.set_text(&GString::from(""));
                }
            }
        }
    }

    pub fn get_current_index(&self) -> usize {
        self.current
    }
}

// ─── Tests ────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_tree() -> Vec<DialogueNode> {
        vec![
            DialogueNode::new("Hello", vec![("Yes", 1), ("No", 2)]),
            DialogueNode::new("You said yes!", vec![]),
            DialogueNode::new("You said no.", vec![("Go back", 0)]),
        ]
    }

    #[test]
    fn node_text_returns_correct_text() {
        let tree = sample_tree();
        assert_eq!(node_text(&tree, 0), "Hello");
        assert_eq!(node_text(&tree, 1), "You said yes!");
    }

    #[test]
    fn node_text_out_of_bounds_returns_empty() {
        let tree = sample_tree();
        assert_eq!(node_text(&tree, 99), "");
    }

    #[test]
    fn choice_count_correct() {
        let tree = sample_tree();
        assert_eq!(choice_count(&tree, 0), 2);
        assert_eq!(choice_count(&tree, 1), 0);
        assert_eq!(choice_count(&tree, 2), 1);
    }

    #[test]
    fn choice_count_out_of_bounds_is_zero() {
        let tree = sample_tree();
        assert_eq!(choice_count(&tree, 50), 0);
    }

    #[test]
    fn can_advance_true_when_choices_exist() {
        let tree = sample_tree();
        assert!(can_advance(&tree, 0));
    }

    #[test]
    fn can_advance_false_on_terminal_node() {
        let tree = sample_tree();
        assert!(!can_advance(&tree, 1));
    }

    #[test]
    fn next_node_index_returns_target() {
        let tree = sample_tree();
        assert_eq!(next_node_index(&tree, 0, 0), Some(1));
        assert_eq!(next_node_index(&tree, 0, 1), Some(2));
    }

    #[test]
    fn next_node_index_out_of_range_choice_is_none() {
        let tree = sample_tree();
        assert_eq!(next_node_index(&tree, 0, 5), None);
    }

    #[test]
    fn next_node_index_terminal_is_none() {
        let tree = sample_tree();
        assert_eq!(next_node_index(&tree, 1, 0), None);
    }

    #[test]
    fn next_node_index_loopback_works() {
        let tree = sample_tree();
        assert_eq!(next_node_index(&tree, 2, 0), Some(0));
    }

    #[test]
    fn build_tree_starts_with_choices() {
        let tree = build_tree();
        assert!(can_advance(&tree, 0), "root node must have choices");
    }
}
