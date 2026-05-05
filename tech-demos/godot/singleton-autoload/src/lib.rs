//! Singleton Autoload demo — Godot 4.3 + gdext 0.5
//!
//! `GameManager` is intended to be registered as an Autoload singleton in
//! Godot's project settings.  It stores a score and level, exposing #[func]
//! methods that GDScript (or other Rust nodes) can call.
//!
//! `ScoreDisplay` is a Label subclass that polls the singleton every frame
//! and updates its own text.

use godot::classes::{Engine, ILabel, INode, Label, Node};
use godot::prelude::*;

// ---------------------------------------------------------------------------
// Extension entry-point
// ---------------------------------------------------------------------------

struct SingletonAutoloadExtension;
#[gdextension]
unsafe impl ExtensionLibrary for SingletonAutoloadExtension {}

// ---------------------------------------------------------------------------
// Pure helper functions
// ---------------------------------------------------------------------------

/// Compute a per-level score threshold.
pub fn score_for_level(base: i32, level: i32) -> i32 {
    base * (1 + level.max(0))
}

/// Format a score/level string for display.
pub fn format_score(score: i32, level: i32) -> String {
    format!("Level {} — Score: {}", level, score)
}

// ---------------------------------------------------------------------------
// GameManager singleton
// ---------------------------------------------------------------------------

#[derive(GodotClass)]
#[class(base=Node)]
pub struct GameManager {
    score: i32,
    level: i32,
    base: Base<Node>,
}

#[godot_api]
impl INode for GameManager {
    fn init(base: Base<Node>) -> Self {
        Self {
            score: 0,
            level: 1,
            base,
        }
    }
}

#[godot_api]
impl GameManager {
    #[func]
    pub fn add_score(&mut self, points: i32) {
        self.score += points;
    }

    #[func]
    pub fn next_level(&mut self) {
        self.level += 1;
    }

    #[func]
    pub fn reset(&mut self) {
        self.score = 0;
        self.level = 1;
    }

    #[func]
    pub fn get_score(&self) -> i32 {
        self.score
    }

    #[func]
    pub fn get_level(&self) -> i32 {
        self.level
    }
}

// ---------------------------------------------------------------------------
// ScoreDisplay — polls the singleton every frame
// ---------------------------------------------------------------------------

#[derive(GodotClass)]
#[class(base=Label)]
pub struct ScoreDisplay {
    base: Base<Label>,
}

#[godot_api]
impl ILabel for ScoreDisplay {
    fn init(base: Base<Label>) -> Self {
        Self { base }
    }

    fn process(&mut self, _delta: f64) {
        let engine = Engine::singleton();
        if let Some(gd) = engine.get_singleton("GameManager") {
            if let Ok(manager) = gd.try_cast::<GameManager>() {
                let score = manager.bind().get_score();
                let level = manager.bind().get_level();
                let text = format_score(score, level);
                self.base_mut().set_text(text.as_str());
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

    #[test]
    fn score_for_level_base_case() {
        assert_eq!(score_for_level(100, 1), 200);
    }

    #[test]
    fn score_for_level_zero() {
        assert_eq!(score_for_level(100, 0), 100);
    }

    #[test]
    fn score_for_level_negative_clamped() {
        // negative level is treated as 0
        assert_eq!(score_for_level(100, -5), 100);
    }

    #[test]
    fn format_score_basic() {
        assert_eq!(format_score(500, 2), "Level 2 — Score: 500");
    }

    #[test]
    fn format_score_zero() {
        assert_eq!(format_score(0, 1), "Level 1 — Score: 0");
    }

    #[test]
    fn format_score_high_values() {
        let s = format_score(999_999, 99);
        assert!(s.contains("999999"));
        assert!(s.contains("99"));
    }
}
