//! Ability Cooldowns GDExtension demo — a cooldown system for game abilities from Rust.
//!
//! Demonstrates:
//!
//! - Defining a plain Rust struct (`Ability`) to hold per-ability state.
//! - Ticking cooldowns every frame in `process()`.
//! - Exposing `use_ability(index)` and `is_ready(index)` as `#[func]` callables.
//! - Rendering a live HUD string on a `Label` child.
//! - Pure cooldown helpers (`tick_cooldown`, `is_ready`, `cooldown_fraction`,
//!   `format_cooldown`) fully covered by unit tests.

use godot::classes::{INode, Label};
use godot::prelude::*;

// ─── Extension entry point ───────────────────────────────────────────────────

struct AbilityCooldownsExt;

#[gdextension]
unsafe impl ExtensionLibrary for AbilityCooldownsExt {}

// ─── Data types ───────────────────────────────────────────────────────────────

/// Data for a single player ability.
pub struct Ability {
    /// Display name shown in the HUD.
    pub name: String,
    /// Total cooldown duration in seconds.
    pub cooldown: f64,
    /// Remaining cooldown in seconds; 0.0 means the ability is ready.
    pub remaining: f64,
    /// Whether the ability is currently active (used but not yet expired).
    pub active: bool,
}

/// The three abilities the demo registers on startup.
///
/// Hand-aligned into columns so the abilities read as a table; `rustfmt` is told
/// to leave it alone.
#[rustfmt::skip]
pub fn default_abilities() -> Vec<Ability> {
    vec![
        Ability { name: "Fireball".into(), cooldown: 3.0, remaining: 0.0, active: false },
        Ability { name: "Shield".into(),   cooldown: 5.0, remaining: 0.0, active: false },
        Ability { name: "Dash".into(),     cooldown: 1.0, remaining: 0.0, active: false },
    ]
}

// ─── Pure functions ───────────────────────────────────────────────────────────

/// Advances the cooldown timer by `delta`, clamping at 0.
///
/// # Examples
/// ```
/// assert!((ability_cooldowns::tick_cooldown(2.0, 0.5) - 1.5).abs() < 1e-9);
/// assert!((ability_cooldowns::tick_cooldown(0.3, 0.5) - 0.0).abs() < 1e-9);
/// assert!((ability_cooldowns::tick_cooldown(0.0, 1.0) - 0.0).abs() < 1e-9);
/// ```
pub fn tick_cooldown(remaining: f64, delta: f64) -> f64 {
    (remaining - delta).max(0.0)
}

/// Returns `true` when `remaining` is exactly 0.0 (ability is ready).
///
/// # Examples
/// ```
/// assert!(ability_cooldowns::is_ready(0.0));
/// assert!(!ability_cooldowns::is_ready(0.001));
/// ```
pub fn is_ready(remaining: f64) -> bool {
    remaining <= 0.0
}

/// Returns the cooldown fraction consumed: `remaining / total`, clamped 0..1.
///
/// Returns 0.0 when `total` is zero to avoid division by zero.
///
/// # Examples
/// ```
/// assert!((ability_cooldowns::cooldown_fraction(1.5, 3.0) - 0.5).abs() < 1e-9);
/// assert!((ability_cooldowns::cooldown_fraction(0.0, 3.0) - 0.0).abs() < 1e-9);
/// assert!((ability_cooldowns::cooldown_fraction(3.0, 3.0) - 1.0).abs() < 1e-9);
/// assert_eq!(ability_cooldowns::cooldown_fraction(1.0, 0.0), 0.0);
/// ```
pub fn cooldown_fraction(remaining: f64, total: f64) -> f64 {
    if total <= 0.0 {
        return 0.0;
    }
    (remaining / total).clamp(0.0, 1.0)
}

/// Formats a remaining cooldown as a short string.
///
/// Shows `"Ready"` when 0, otherwise `"X.Xs"`.
///
/// # Examples
/// ```
/// assert_eq!(ability_cooldowns::format_cooldown(0.0), "Ready");
/// assert_eq!(ability_cooldowns::format_cooldown(1.5), "1.5s");
/// assert_eq!(ability_cooldowns::format_cooldown(3.0), "3.0s");
/// ```
pub fn format_cooldown(remaining: f64) -> String {
    if remaining <= 0.0 {
        "Ready".to_string()
    } else {
        format!("{:.1}s", remaining)
    }
}

// ─── AbilityManager node ─────────────────────────────────────────────────────

/// A `Node` that tracks three player abilities and their cooldowns.
///
/// Abilities:
/// 1. Fireball — 3 s cooldown
/// 2. Shield   — 5 s cooldown
/// 3. Dash     — 1 s cooldown
///
/// The HUD `Label` child (named `"HudLabel"`) is updated every frame.
#[derive(GodotClass)]
#[class(base=Node)]
pub struct AbilityManager {
    /// All tracked abilities.
    abilities: Vec<Ability>,
    base: Base<Node>,
}

#[godot_api]
impl INode for AbilityManager {
    fn init(base: Base<Node>) -> Self {
        Self {
            abilities: default_abilities(),
            base,
        }
    }

    fn ready(&mut self) {
        godot_print!(
            "[AbilityManager] Ready — {} abilities registered.",
            self.abilities.len()
        );
    }

    fn process(&mut self, delta: f64) {
        for ability in &mut self.abilities {
            ability.remaining = tick_cooldown(ability.remaining, delta);
            if ability.remaining <= 0.0 {
                ability.active = false;
            }
        }
        self.update_hud();
    }
}

#[godot_api]
impl AbilityManager {
    /// Attempts to activate ability at `index`.  Returns `true` on success.
    ///
    /// Fails (returns `false`) when the index is out of range or the ability
    /// is still on cooldown.
    #[func]
    pub fn use_ability(&mut self, index: i32) -> bool {
        let idx = index as usize;
        if idx >= self.abilities.len() {
            return false;
        }
        if !is_ready(self.abilities[idx].remaining) {
            return false;
        }
        self.abilities[idx].remaining = self.abilities[idx].cooldown;
        self.abilities[idx].active = true;
        let name = self.abilities[idx].name.clone();
        godot_print!("[AbilityManager] Used '{}'.", name);
        true
    }

    /// Returns `true` when the ability at `index` is off cooldown and usable.
    #[func]
    pub fn is_ready_func(&self, index: i32) -> bool {
        let idx = index as usize;
        if idx >= self.abilities.len() {
            return false;
        }
        is_ready(self.abilities[idx].remaining)
    }

    /// Rebuilds and pushes the HUD string to the `Label` child.
    fn update_hud(&mut self) {
        let lines: Vec<String> = self
            .abilities
            .iter()
            .map(|a| format!("{}: {}", a.name, format_cooldown(a.remaining)))
            .collect();
        let text = lines.join("\n");
        if let Some(mut label) = self.base().try_get_node_as::<Label>("HudLabel") {
            label.set_text(text.as_str());
        }
    }
}

// ─── Tests ────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    // tick_cooldown

    #[test]
    fn tick_cooldown_reduces_remaining() {
        assert!((tick_cooldown(2.0, 0.5) - 1.5).abs() < 1e-9);
    }

    #[test]
    fn tick_cooldown_clamps_at_zero() {
        assert!((tick_cooldown(0.3, 0.5)).abs() < 1e-9);
    }

    #[test]
    fn tick_cooldown_already_zero() {
        assert!((tick_cooldown(0.0, 1.0)).abs() < 1e-9);
    }

    // is_ready

    #[test]
    fn is_ready_when_zero() {
        assert!(is_ready(0.0));
    }

    #[test]
    fn not_ready_when_positive() {
        assert!(!is_ready(0.001));
    }

    // cooldown_fraction

    #[test]
    fn cooldown_fraction_half() {
        assert!((cooldown_fraction(1.5, 3.0) - 0.5).abs() < 1e-9);
    }

    #[test]
    fn cooldown_fraction_zero_remaining() {
        assert!((cooldown_fraction(0.0, 3.0)).abs() < 1e-9);
    }

    #[test]
    fn cooldown_fraction_full() {
        assert!((cooldown_fraction(3.0, 3.0) - 1.0).abs() < 1e-9);
    }

    #[test]
    fn cooldown_fraction_zero_total() {
        assert_eq!(cooldown_fraction(1.0, 0.0), 0.0);
    }

    // format_cooldown

    #[test]
    fn format_cooldown_zero_is_ready() {
        assert_eq!(format_cooldown(0.0), "Ready");
    }

    #[test]
    fn format_cooldown_nonzero() {
        assert_eq!(format_cooldown(1.5), "1.5s");
    }

    #[test]
    fn format_cooldown_whole_number() {
        assert_eq!(format_cooldown(3.0), "3.0s");
    }
}
