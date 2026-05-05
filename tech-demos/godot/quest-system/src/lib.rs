//! Quest System demo — Godot 4.3 + gdext 0.5
//!
//! Manages a collection of `Quest` values with status transitions driven by
//! `start_quest`, `advance_quest`, and `fail_quest`.  All pure logic lives
//! in helper functions so unit tests run without the Godot runtime.

use godot::classes::{INode, Label, Node};
use godot::prelude::*;

// ---------------------------------------------------------------------------
// Extension entry-point
// ---------------------------------------------------------------------------

struct QuestSystemExtension;
#[gdextension]
unsafe impl ExtensionLibrary for QuestSystemExtension {}

// ---------------------------------------------------------------------------
// Pure-Rust types
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum QuestStatus {
    Locked,
    Available,
    Active,
    Completed,
    Failed,
}

#[derive(Debug, Clone)]
pub struct Quest {
    pub id: u32,
    pub title: String,
    pub status: QuestStatus,
    pub objective_count: u32,
    pub progress: u32,
}

// ---------------------------------------------------------------------------
// Pure helper functions
// ---------------------------------------------------------------------------

/// Fraction of objectives completed, clamped to [0.0, 1.0].
pub fn progress_fraction(progress: u32, total: u32) -> f32 {
    if total == 0 {
        return 0.0;
    }
    (progress as f32 / total as f32).min(1.0)
}

/// Human-readable status label.
pub fn status_label(status: u8) -> &'static str {
    match status {
        0 => "Locked",
        1 => "Available",
        2 => "Active",
        3 => "Completed",
        4 => "Failed",
        _ => "Unknown",
    }
}

/// Returns true if a quest in this status can be started.
pub fn can_start(status: u8) -> bool {
    status == 1 // Only Available quests can be started
}

/// Convert `QuestStatus` to its u8 discriminant.
pub fn status_to_u8(s: QuestStatus) -> u8 {
    match s {
        QuestStatus::Locked => 0,
        QuestStatus::Available => 1,
        QuestStatus::Active => 2,
        QuestStatus::Completed => 3,
        QuestStatus::Failed => 4,
    }
}

// ---------------------------------------------------------------------------
// GodotClass
// ---------------------------------------------------------------------------

#[derive(GodotClass)]
#[class(base=Node)]
pub struct QuestManager {
    quests: Vec<Quest>,
    base: Base<Node>,
}

#[godot_api]
impl INode for QuestManager {
    fn init(base: Base<Node>) -> Self {
        // Pre-populate a few demo quests
        let quests = vec![
            Quest {
                id: 1,
                title: "First Steps".to_string(),
                status: QuestStatus::Available,
                objective_count: 3,
                progress: 0,
            },
            Quest {
                id: 2,
                title: "Treasure Hunter".to_string(),
                status: QuestStatus::Locked,
                objective_count: 5,
                progress: 0,
            },
            Quest {
                id: 3,
                title: "Dragon Slayer".to_string(),
                status: QuestStatus::Locked,
                objective_count: 1,
                progress: 0,
            },
        ];
        Self {
            quests,
            base,
        }
    }

    fn ready(&mut self) {
        self.refresh_label();
    }
}

#[godot_api]
impl QuestManager {
    /// Start a quest by id; returns false if the quest is not in Available status.
    #[func]
    pub fn start_quest(&mut self, id: i32) -> bool {
        let uid = id as u32;
        if let Some(q) = self.quests.iter_mut().find(|q| q.id == uid) {
            if can_start(status_to_u8(q.status)) {
                q.status = QuestStatus::Active;
                self.refresh_label();
                return true;
            }
        }
        false
    }

    /// Advance a quest's progress; auto-completes if progress reaches the
    /// objective count.
    #[func]
    pub fn advance_quest(&mut self, id: i32, amount: i32) {
        let uid = id as u32;
        if let Some(q) = self.quests.iter_mut().find(|q| q.id == uid) {
            if q.status == QuestStatus::Active {
                q.progress = q.progress.saturating_add(amount.max(0) as u32);
                if q.progress >= q.objective_count {
                    q.progress = q.objective_count;
                    q.status = QuestStatus::Completed;
                }
                self.refresh_label();
            }
        }
    }

    /// Fail a quest, preventing further progress.
    #[func]
    pub fn fail_quest(&mut self, id: i32) {
        let uid = id as u32;
        if let Some(q) = self.quests.iter_mut().find(|q| q.id == uid) {
            if q.status == QuestStatus::Active {
                q.status = QuestStatus::Failed;
                self.refresh_label();
            }
        }
    }

    /// Return the number of currently active quests.
    #[func]
    pub fn get_active_count(&self) -> i32 {
        self.quests
            .iter()
            .filter(|q| q.status == QuestStatus::Active)
            .count() as i32
    }

    /// Unlock a quest (move from Locked to Available).
    #[func]
    pub fn unlock_quest(&mut self, id: i32) {
        let uid = id as u32;
        if let Some(q) = self.quests.iter_mut().find(|q| q.id == uid) {
            if q.status == QuestStatus::Locked {
                q.status = QuestStatus::Available;
                self.refresh_label();
            }
        }
    }

    fn refresh_label(&mut self) {
        let lines: Vec<String> = self
            .quests
            .iter()
            .filter(|q| q.status == QuestStatus::Active)
            .map(|q| {
                let pct = (progress_fraction(q.progress, q.objective_count) * 100.0) as u32;
                format!("{} — {}%", q.title, pct)
            })
            .collect();
        let text = if lines.is_empty() {
            "No active quests".to_string()
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

    #[test]
    fn progress_fraction_zero_total() {
        assert!((progress_fraction(5, 0) - 0.0).abs() < f32::EPSILON);
    }

    #[test]
    fn progress_fraction_partial() {
        let f = progress_fraction(1, 4);
        assert!((f - 0.25).abs() < 1e-6);
    }

    #[test]
    fn progress_fraction_complete() {
        assert!((progress_fraction(4, 4) - 1.0).abs() < f32::EPSILON);
    }

    #[test]
    fn progress_fraction_overflow_clamped() {
        assert!((progress_fraction(10, 4) - 1.0).abs() < f32::EPSILON);
    }

    #[test]
    fn status_label_all_variants() {
        assert_eq!(status_label(0), "Locked");
        assert_eq!(status_label(1), "Available");
        assert_eq!(status_label(2), "Active");
        assert_eq!(status_label(3), "Completed");
        assert_eq!(status_label(4), "Failed");
        assert_eq!(status_label(99), "Unknown");
    }

    #[test]
    fn can_start_only_available() {
        assert!(!can_start(0)); // Locked
        assert!(can_start(1));  // Available
        assert!(!can_start(2)); // Active
        assert!(!can_start(3)); // Completed
        assert!(!can_start(4)); // Failed
    }

    #[test]
    fn status_to_u8_roundtrips() {
        let statuses = [
            QuestStatus::Locked,
            QuestStatus::Available,
            QuestStatus::Active,
            QuestStatus::Completed,
            QuestStatus::Failed,
        ];
        for s in statuses {
            let v = status_to_u8(s);
            assert_eq!(status_label(v), format!("{:?}", s).as_str().split("::").last().unwrap());
        }
    }

    #[test]
    fn progress_fraction_half() {
        let f = progress_fraction(2, 4);
        assert!((f - 0.5).abs() < 1e-6);
    }
}
