//! Achievement system demo — Godot 4.3 + gdext 0.5.
//!
//! Teaches:
//! - Storing achievement state in a plain Rust struct (no Godot Resource needed).
//! - Pure functions for progress fraction, unlock detection, and display text.
//! - Refreshing a Label-based HUD from Rust each time state changes.
//! - Showing a brief unlock banner when an achievement is newly completed.
//!
//! Achievements track: score, steps taken, and enemies defeated.
//! Press keys to simulate game events; the HUD reflects progress in real time.
//!
//! **Controls:** S — add score;  W — step;  K — kill enemy;  ESC — reset.

use godot::classes::{INode2D, Input, Label, Node2D};
use godot::prelude::*;

// ─── Extension entry point ────────────────────────────────────────────────────

struct AchievementSystemExt;
#[gdextension]
unsafe impl ExtensionLibrary for AchievementSystemExt {}

// ─── Pure achievement helpers ─────────────────────────────────────────────────

/// Fraction of goal reached, clamped to `[0.0, 1.0]`.
pub fn achievement_fraction(progress: i32, goal: i32) -> f32 {
    if goal <= 0 {
        return 1.0;
    }
    (progress as f32 / goal as f32).clamp(0.0, 1.0)
}

/// `true` when `progress >= goal`.
pub fn is_unlocked(progress: i32, goal: i32) -> bool {
    progress >= goal
}

/// Human-readable progress string: `"Name  3 / 10"` or `"Name  ✓"`.
pub fn progress_text(name: &str, progress: i32, goal: i32) -> String {
    if is_unlocked(progress, goal) {
        format!("[✓] {}", name)
    } else {
        format!("[ ] {}  {}/{}", name, progress, goal)
    }
}

// ─── Achievement data ─────────────────────────────────────────────────────────

#[derive(Clone)]
pub struct Achievement {
    pub name: &'static str,
    pub goal: i32,
    pub progress: i32,
    pub unlocked: bool,
}

impl Achievement {
    pub fn new(name: &'static str, goal: i32) -> Self {
        Self {
            name,
            goal,
            progress: 0,
            unlocked: false,
        }
    }

    /// Returns `true` the first time the achievement tips into unlocked.
    pub fn add_progress(&mut self, amount: i32) -> bool {
        if self.unlocked {
            return false;
        }
        self.progress = (self.progress + amount).min(self.goal);
        if is_unlocked(self.progress, self.goal) {
            self.unlocked = true;
            return true;
        }
        false
    }
}

// ─── AchievementArena — root Node2D ───────────────────────────────────────────

#[derive(GodotClass)]
#[class(base=Node2D)]
pub struct AchievementArena {
    achievements: Vec<Achievement>,
    banner_timer: f32,
    banner_text: String,
    dirty: bool,
    base: Base<Node2D>,
}

pub fn default_achievements() -> Vec<Achievement> {
    vec![
        Achievement::new("First Blood", 1),
        Achievement::new("Score 50", 50),
        Achievement::new("Score 200", 200),
        Achievement::new("Take 10 Steps", 10),
        Achievement::new("Take 50 Steps", 50),
        Achievement::new("Kill 5 Enemies", 5),
        Achievement::new("Kill 20 Enemies", 20),
    ]
}

#[godot_api]
impl INode2D for AchievementArena {
    fn init(base: Base<Node2D>) -> Self {
        Self {
            achievements: default_achievements(),
            banner_timer: 0.0,
            banner_text: String::new(),
            dirty: true,
            base,
        }
    }

    fn ready(&mut self) {
        let mut hint = Label::new_alloc();
        hint.set_text("S — score   W — step   K — kill   ESC — reset");
        hint.set_position(Vector2::new(10.0, 8.0));
        self.base_mut().add_child(&hint);

        // Achievement list labels
        for i in 0..7usize {
            let mut lbl = Label::new_alloc();
            let lname = format!("Ach{}", i);
            lbl.set_name(lname.as_str());
            lbl.set_position(Vector2::new(20.0, 40.0 + i as f32 * 26.0));
            self.base_mut().add_child(&lbl);
        }

        // Banner label (unlock announcement)
        let mut banner = Label::new_alloc();
        banner.set_name("Banner");
        banner.set_position(Vector2::new(20.0, 240.0));
        banner.set_text("");
        self.base_mut().add_child(&banner);
    }

    fn process(&mut self, delta: f64) {
        let dt = delta as f32;
        let input = Input::singleton();

        if input.is_action_just_pressed("ui_cancel") {
            self.achievements = default_achievements();
            self.dirty = true;
        }

        // Score (S key — mapped to ui_select in Godot default actions)
        if input.is_action_just_pressed("ui_select") {
            let newly = self.add_progress_by_category("Score", 10);
            self.show_banner_if(newly);
        }

        // Step (W key — mapped via ui_up for demo convenience)
        if input.is_action_just_pressed("ui_up") {
            let newly = self.add_progress_by_category("Step", 1);
            self.show_banner_if(newly);
        }

        // Kill (K key — mapped via ui_accept)
        if input.is_action_just_pressed("ui_accept") {
            let newly = self.add_progress_by_category("Kill", 1);
            // "First Blood" also unlocks on first kill
            let fb = self.achievements[0].add_progress(1);
            let mut all = newly;
            if fb {
                all.push("First Blood");
            }
            self.show_banner_if(all);
        }

        // Tick banner
        if self.banner_timer > 0.0 {
            self.banner_timer -= dt;
            if self.banner_timer <= 0.0 {
                self.banner_timer = 0.0;
                if let Some(mut b) = self.base().try_get_node_as::<Label>("Banner") {
                    b.set_text("");
                }
            }
        }

        if self.dirty {
            self.refresh_hud();
            self.dirty = false;
        }
    }
}

#[godot_api]
impl AchievementArena {
    fn add_progress_by_category(&mut self, keyword: &str, amount: i32) -> Vec<&'static str> {
        let mut newly_unlocked = Vec::new();
        for ach in &mut self.achievements {
            if ach.name.contains(keyword) && ach.add_progress(amount) {
                newly_unlocked.push(ach.name);
            }
        }
        self.dirty = true;
        newly_unlocked
    }

    fn show_banner_if(&mut self, names: Vec<&'static str>) {
        if names.is_empty() {
            return;
        }
        self.banner_text = format!("Unlocked: {}", names.join(", "));
        self.banner_timer = 2.5;
        let text = self.banner_text.clone();
        if let Some(mut b) = self.base().try_get_node_as::<Label>("Banner") {
            b.set_text(&GString::from(text.as_str()));
        }
    }

    fn refresh_hud(&mut self) {
        for (i, ach) in self.achievements.iter().enumerate() {
            let text = progress_text(ach.name, ach.progress, ach.goal);
            let node_name = format!("Ach{}", i);
            if let Some(mut lbl) = self.base().try_get_node_as::<Label>(node_name.as_str()) {
                lbl.set_text(&GString::from(text.as_str()));
            }
        }
    }

    pub fn achievement_count(&self) -> usize {
        self.achievements.len()
    }

    pub fn unlocked_count(&self) -> usize {
        self.achievements.iter().filter(|a| a.unlocked).count()
    }
}

// ─── Tests ────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn achievement_fraction_zero_progress() {
        assert!((achievement_fraction(0, 10) - 0.0).abs() < 1e-5);
    }

    #[test]
    fn achievement_fraction_full_progress() {
        assert!((achievement_fraction(10, 10) - 1.0).abs() < 1e-5);
    }

    #[test]
    fn achievement_fraction_half_progress() {
        assert!((achievement_fraction(5, 10) - 0.5).abs() < 1e-5);
    }

    #[test]
    fn achievement_fraction_clamped_above_one() {
        assert!((achievement_fraction(15, 10) - 1.0).abs() < 1e-5);
    }

    #[test]
    fn achievement_fraction_zero_goal_is_one() {
        assert!((achievement_fraction(0, 0) - 1.0).abs() < 1e-5);
    }

    #[test]
    fn is_unlocked_at_goal() {
        assert!(is_unlocked(10, 10));
    }

    #[test]
    fn is_unlocked_above_goal() {
        assert!(is_unlocked(15, 10));
    }

    #[test]
    fn is_unlocked_below_goal_is_false() {
        assert!(!is_unlocked(9, 10));
    }

    #[test]
    fn progress_text_incomplete() {
        assert_eq!(progress_text("Score 50", 20, 50), "[ ] Score 50  20/50");
    }

    #[test]
    fn progress_text_complete_shows_check() {
        let t = progress_text("Score 50", 50, 50);
        assert!(t.contains("[✓]"));
    }

    #[test]
    fn add_progress_returns_true_on_unlock() {
        let mut a = Achievement::new("Test", 3);
        assert!(!a.add_progress(2));
        assert!(a.add_progress(1));
    }

    #[test]
    fn add_progress_no_double_unlock() {
        let mut a = Achievement::new("Test", 1);
        assert!(a.add_progress(1));
        assert!(!a.add_progress(1));
    }

    #[test]
    fn add_progress_does_not_exceed_goal() {
        let mut a = Achievement::new("Test", 5);
        a.add_progress(10);
        assert_eq!(a.progress, 5);
    }
}
