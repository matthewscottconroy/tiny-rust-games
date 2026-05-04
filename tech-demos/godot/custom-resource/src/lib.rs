//! Custom Resource GDExtension demo — Rust structs as first-class Godot Resources.
//!
//! Teaches: defining a `Resource` subclass with `#[export]` fields, saving it
//! as a `.tres` file, setting it in the Godot inspector, and reading it from a
//! Node.  `WeaponData` is the Godot equivalent of a data class or
//! `ScriptableObject` — it holds weapon stats and exposes computed properties
//! via `#[func]` methods.

use godot::prelude::*;

// ─── Extension entry point ────────────────────────────────────────────────────

/// Declares this crate as a GDExtension library.
struct CustomResourceExt;

#[gdextension]
unsafe impl ExtensionLibrary for CustomResourceExt {}

// ─── WeaponData resource ──────────────────────────────────────────────────────

/// A `Resource` subclass that holds weapon statistics.
///
/// Create a `.tres` file in the Godot FileSystem dock, set its type to
/// `WeaponData`, fill in the fields in the inspector, then assign it to a
/// `WeaponDisplay` node via the inspector's `weapon` slot.
#[derive(GodotClass)]
#[class(base=Resource)]
pub struct WeaponData {
    /// Human-readable weapon name shown to the player.
    #[export]
    pub name: GString,
    /// Base damage dealt per hit.
    #[export]
    pub damage: f64,
    /// Number of attacks per second.
    #[export]
    pub attack_speed: f64,
    /// Maximum reach of the weapon in pixels.
    #[export]
    pub range: f64,
    /// Whether the weapon deals magic damage (ignores part of armour).
    #[export]
    pub is_magic: bool,
    base: Base<Resource>,
}

#[godot_api]
impl IResource for WeaponData {
    fn init(base: Base<Resource>) -> Self {
        Self {
            name: "Sword".into(),
            damage: 25.0,
            attack_speed: 1.5,
            range: 80.0,
            is_magic: false,
            base,
        }
    }
}

#[godot_api]
impl WeaponData {
    /// Returns damage per second (damage × attack_speed).
    #[func]
    pub fn dps(&self) -> f64 {
        compute_dps(self.damage, self.attack_speed)
    }

    /// Returns a formatted description of the weapon's stats.
    #[func]
    pub fn describe(&self) -> GString {
        GString::from(
            describe_weapon(
                self.name.to_string().as_str(),
                self.damage,
                self.attack_speed,
                self.is_magic,
            )
            .as_str(),
        )
    }
}

// ─── WeaponDisplay node ───────────────────────────────────────────────────────

/// A Node2D that reads a `WeaponData` resource and displays its stats.
///
/// Set the `weapon` property in the Godot inspector to a `WeaponData` resource,
/// then run the scene.  On `ready`, the weapon description is printed to the
/// Godot output panel.
#[derive(GodotClass)]
#[class(base=Node2D)]
pub struct WeaponDisplay {
    /// The weapon resource to display.  Assign in the Godot inspector.
    #[export]
    weapon: Option<Gd<WeaponData>>,
    base: Base<Node2D>,
}

#[godot_api]
impl INode2D for WeaponDisplay {
    fn init(base: Base<Node2D>) -> Self {
        Self { weapon: None, base }
    }

    fn ready(&mut self) {
        if let Some(ref w) = self.weapon {
            let desc = w.bind().describe();
            godot_print!("{}", desc);
        } else {
            godot_print!("WeaponDisplay: no weapon assigned.");
        }
    }
}

#[godot_api]
impl WeaponDisplay {
    /// Returns the weapon's DPS, or `0.0` if no weapon is assigned.
    #[func]
    pub fn get_dps(&self) -> f64 {
        self.weapon
            .as_ref()
            .map(|w| w.bind().dps())
            .unwrap_or(0.0)
    }
}

// ─── Pure functions ───────────────────────────────────────────────────────────

/// Returns damage per second given `damage` and `attack_speed`.
pub fn compute_dps(damage: f64, attack_speed: f64) -> f64 {
    damage * attack_speed
}

/// Returns a weapon description string suitable for display.
pub fn describe_weapon(name: &str, damage: f64, attack_speed: f64, is_magic: bool) -> String {
    let kind = if is_magic { "Magic" } else { "Physical" };
    format!(
        "{name} [{kind}] — {damage:.1} dmg @ {attack_speed:.2}/s ({dps:.1} DPS)",
        dps = compute_dps(damage, attack_speed),
    )
}

/// Returns the effective damage dealt against an armoured target.
///
/// Physical damage is reduced by `armour`; magic damage ignores 50 % of that
/// reduction (i.e. only `armour / 2` applies).  Effective damage is clamped
/// to a minimum of `0.0`.
pub fn effective_damage(damage: f64, armour: f64, is_magic: bool) -> f64 {
    let reduction = if is_magic { armour / 2.0 } else { armour };
    (damage - reduction).max(0.0)
}

// ─── Tests ────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn compute_dps_basic() {
        assert_eq!(compute_dps(25.0, 1.5), 37.5);
    }

    #[test]
    fn compute_dps_zero_attack_speed() {
        assert_eq!(compute_dps(100.0, 0.0), 0.0);
    }

    #[test]
    fn compute_dps_zero_damage() {
        assert_eq!(compute_dps(0.0, 3.0), 0.0);
    }

    #[test]
    fn describe_weapon_physical() {
        let desc = describe_weapon("Sword", 25.0, 1.5, false);
        assert!(desc.contains("Sword"));
        assert!(desc.contains("Physical"));
        assert!(desc.contains("25.0 dmg"));
        assert!(desc.contains("37.5 DPS"));
    }

    #[test]
    fn describe_weapon_magic() {
        let desc = describe_weapon("Staff", 40.0, 1.0, true);
        assert!(desc.contains("Staff"));
        assert!(desc.contains("Magic"));
        assert!(desc.contains("40.0 dmg"));
    }

    #[test]
    fn effective_damage_physical_reduced_by_full_armour() {
        assert_eq!(effective_damage(30.0, 10.0, false), 20.0);
    }

    #[test]
    fn effective_damage_magic_reduced_by_half_armour() {
        assert_eq!(effective_damage(30.0, 10.0, true), 25.0);
    }

    #[test]
    fn effective_damage_clamps_to_zero() {
        assert_eq!(effective_damage(5.0, 100.0, false), 0.0);
        assert_eq!(effective_damage(5.0, 100.0, true), 0.0);
    }

    #[test]
    fn effective_damage_no_armour() {
        assert_eq!(effective_damage(50.0, 0.0, false), 50.0);
        assert_eq!(effective_damage(50.0, 0.0, true), 50.0);
    }

    #[test]
    fn describe_weapon_dps_matches_compute_dps() {
        let damage = 20.0_f64;
        let speed = 2.0_f64;
        let desc = describe_weapon("Dagger", damage, speed, false);
        let expected_dps = compute_dps(damage, speed);
        assert!(desc.contains(&format!("{:.1} DPS", expected_dps)));
    }
}
