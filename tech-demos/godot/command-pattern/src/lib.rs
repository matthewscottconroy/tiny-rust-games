//! Command Pattern demo — Godot 4.3 + gdext 0.5
//!
//! Implements a classic undo/redo stack using a pure-Rust `Command` trait.
//! The Godot class exposes `execute_move`, `undo_last`, and `redo_last` as
//! `#[func]` methods callable from GDScript.

use godot::classes::{INode, Label, Node};
use godot::prelude::*;

// ---------------------------------------------------------------------------
// Extension entry-point
// ---------------------------------------------------------------------------

struct CommandPatternExtension;
#[gdextension]
unsafe impl ExtensionLibrary for CommandPatternExtension {}

// ---------------------------------------------------------------------------
// Pure-Rust types
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, PartialEq)]
pub struct GameState {
    pub x: f32,
    pub y: f32,
    pub score: i32,
}

impl GameState {
    pub fn new() -> Self {
        Self {
            x: 0.0,
            y: 0.0,
            score: 0,
        }
    }
}

impl Default for GameState {
    fn default() -> Self {
        Self::new()
    }
}

// ---------------------------------------------------------------------------
// Command trait and implementations
// ---------------------------------------------------------------------------

pub trait Command: Send {
    fn execute(&self, state: &mut GameState) -> GameState;
    fn undo(&self, state: &mut GameState) -> GameState;
}

pub struct MoveCommand {
    pub dx: f32,
    pub dy: f32,
}

impl Command for MoveCommand {
    fn execute(&self, state: &mut GameState) -> GameState {
        apply_move(state, self.dx, self.dy)
    }

    fn undo(&self, state: &mut GameState) -> GameState {
        apply_move(state, -self.dx, -self.dy)
    }
}

pub struct ScoreCommand {
    pub delta: i32,
}

impl Command for ScoreCommand {
    fn execute(&self, state: &mut GameState) -> GameState {
        apply_score(state, self.delta)
    }

    fn undo(&self, state: &mut GameState) -> GameState {
        apply_score(state, -self.delta)
    }
}

// ---------------------------------------------------------------------------
// Pure helper functions
// ---------------------------------------------------------------------------

/// Apply a movement offset to a game state, returning the new state.
pub fn apply_move(state: &GameState, dx: f32, dy: f32) -> GameState {
    GameState {
        x: state.x + dx,
        y: state.y + dy,
        score: state.score,
    }
}

/// Apply a score delta to a game state, returning the new state.
pub fn apply_score(state: &GameState, delta: i32) -> GameState {
    GameState {
        x: state.x,
        y: state.y,
        score: state.score + delta,
    }
}

// ---------------------------------------------------------------------------
// GodotClass
// ---------------------------------------------------------------------------

#[derive(GodotClass)]
#[class(base=Node)]
pub struct CommandManager {
    state: GameState,
    history: Vec<Box<dyn Command>>,
    redo_stack: Vec<Box<dyn Command>>,
    base: Base<Node>,
}

#[godot_api]
impl INode for CommandManager {
    fn init(base: Base<Node>) -> Self {
        Self {
            state: GameState::new(),
            history: Vec::new(),
            redo_stack: Vec::new(),
            base,
        }
    }
}

#[godot_api]
impl CommandManager {
    #[func]
    pub fn execute_move(&mut self, dx: f32, dy: f32) {
        let cmd = MoveCommand { dx, dy };
        self.state = cmd.execute(&mut self.state);
        self.history.push(Box::new(cmd));
        self.redo_stack.clear();
        self.refresh_label();
    }

    #[func]
    pub fn execute_score(&mut self, delta: i32) {
        let cmd = ScoreCommand { delta };
        self.state = cmd.execute(&mut self.state);
        self.history.push(Box::new(cmd));
        self.redo_stack.clear();
        self.refresh_label();
    }

    #[func]
    pub fn undo_last(&mut self) {
        if let Some(cmd) = self.history.pop() {
            self.state = cmd.undo(&mut self.state);
            self.redo_stack.push(cmd);
            self.refresh_label();
        }
    }

    #[func]
    pub fn redo_last(&mut self) {
        if let Some(cmd) = self.redo_stack.pop() {
            self.state = cmd.execute(&mut self.state);
            self.history.push(cmd);
            self.refresh_label();
        }
    }

    #[func]
    pub fn get_x(&self) -> f32 {
        self.state.x
    }

    #[func]
    pub fn get_y(&self) -> f32 {
        self.state.y
    }

    #[func]
    pub fn get_score(&self) -> i32 {
        self.state.score
    }

    fn refresh_label(&mut self) {
        let text = format!(
            "x={:.1}  y={:.1}  score={}",
            self.state.x, self.state.y, self.state.score
        );
        if let Some(node) = self.base().get_node_or_null("Label")
            && let Ok(mut label) = node.try_cast::<Label>()
        {
            label.set_text(text.as_str());
        }
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn apply_move_adds_offsets() {
        let s = GameState {
            x: 1.0,
            y: 2.0,
            score: 0,
        };
        let next = apply_move(&s, 3.0, 4.0);
        assert!((next.x - 4.0).abs() < f32::EPSILON);
        assert!((next.y - 6.0).abs() < f32::EPSILON);
    }

    #[test]
    fn apply_move_preserves_score() {
        let s = GameState {
            x: 0.0,
            y: 0.0,
            score: 42,
        };
        let next = apply_move(&s, 1.0, 0.0);
        assert_eq!(next.score, 42);
    }

    #[test]
    fn apply_score_adds_delta() {
        let s = GameState {
            x: 0.0,
            y: 0.0,
            score: 10,
        };
        let next = apply_score(&s, 5);
        assert_eq!(next.score, 15);
    }

    #[test]
    fn apply_score_preserves_position() {
        let s = GameState {
            x: 3.0,
            y: 7.0,
            score: 0,
        };
        let next = apply_score(&s, 100);
        assert!((next.x - 3.0).abs() < f32::EPSILON);
        assert!((next.y - 7.0).abs() < f32::EPSILON);
    }

    #[test]
    fn move_command_execute_undo_roundtrip() {
        let mut s = GameState::default();
        let cmd = MoveCommand { dx: 5.0, dy: -3.0 };
        let after_exec = cmd.execute(&mut s);
        let after_undo = cmd.undo(&mut after_exec.clone());
        assert!((after_undo.x - s.x).abs() < f32::EPSILON);
        assert!((after_undo.y - s.y).abs() < f32::EPSILON);
    }

    #[test]
    fn score_command_execute_undo_roundtrip() {
        let mut s = GameState {
            x: 0.0,
            y: 0.0,
            score: 50,
        };
        let cmd = ScoreCommand { delta: 20 };
        let after_exec = cmd.execute(&mut s);
        assert_eq!(after_exec.score, 70);
        let after_undo = cmd.undo(&mut after_exec.clone());
        assert_eq!(after_undo.score, 50);
    }

    #[test]
    fn game_state_default() {
        let s = GameState::default();
        assert_eq!(s.score, 0);
        assert!((s.x).abs() < f32::EPSILON);
        assert!((s.y).abs() < f32::EPSILON);
    }

    #[test]
    fn negative_score_delta() {
        let mut s = GameState {
            x: 0.0,
            y: 0.0,
            score: 10,
        };
        let cmd = ScoreCommand { delta: -15 };
        let next = cmd.execute(&mut s);
        assert_eq!(next.score, -5);
    }
}
