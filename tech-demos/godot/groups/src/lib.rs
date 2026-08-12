//! Groups GDExtension demo — demonstrates Godot node groups from Rust.
//!
//! Groups are Godot's broadcast mechanism: nodes opt in to named groups via
//! `add_to_group`, and any code with access to the `SceneTree` can query all
//! members and call methods on them.  This demo shows:
//!
//! - Registering nodes in multiple groups during `ready()`.
//! - Querying group membership with `get_nodes_in_group`.
//! - Dispatching a method call to every member of a group.
//! - A "manager" node that orchestrates periodic group-wide damage.
//!
//! Teaches: adding nodes to groups, querying group membership, and calling methods
//! across a whole group at once.

use godot::prelude::*;

// ─── Extension entry point ───────────────────────────────────────────────────

/// Declares this crate as a GDExtension library.
struct GroupsExt;

#[gdextension]
unsafe impl ExtensionLibrary for GroupsExt {}

// ─── Pure functions ───────────────────────────────────────────────────────────

/// Returns the group name string for a given team index.
///
/// # Examples
/// ```
/// assert_eq!(groups::team_group_name(0), "team_0");
/// assert_eq!(groups::team_group_name(3), "team_3");
/// ```
pub fn team_group_name(team: i64) -> String {
    format!("team_{}", team)
}

/// Returns `true` if `health` is above the alive threshold (> 0.0).
///
/// # Examples
/// ```
/// assert!(groups::is_alive(50.0));
/// assert!(!groups::is_alive(0.0));
/// assert!(!groups::is_alive(-1.0));
/// ```
pub fn is_alive(health: f64) -> bool {
    health > 0.0
}

/// Returns how many full intervals have elapsed given `total_time` and `interval`.
///
/// Returns 0 if `interval` is zero or negative to avoid division by zero.
///
/// # Examples
/// ```
/// assert_eq!(groups::intervals_elapsed(5.0, 2.0), 2);
/// assert_eq!(groups::intervals_elapsed(2.0, 2.0), 1);
/// assert_eq!(groups::intervals_elapsed(1.9, 2.0), 0);
/// assert_eq!(groups::intervals_elapsed(6.0, 2.0), 3);
/// ```
pub fn intervals_elapsed(total_time: f64, interval: f64) -> u64 {
    if interval <= 0.0 {
        return 0;
    }
    (total_time / interval) as u64
}

// ─── Enemy node ──────────────────────────────────────────────────────────────

/// A hostile unit that registers itself in the `"enemies"` group and its
/// team-specific group during `ready()`.
///
/// Place one or more `Enemy` nodes as children of a `GroupManager` scene root.
/// The `team` export lets you assign enemies to sub-groups (e.g. `"team_0"`).
#[derive(GodotClass)]
#[class(base=Node2D)]
pub struct Enemy {
    /// Team index; determines which `"team_N"` group this enemy joins.
    #[export]
    team: i64,
    /// Current hit points.  Starts at 100.0; falls to 0 when dead.
    #[export]
    health: f64,
    base: Base<Node2D>,
}

#[godot_api]
impl INode2D for Enemy {
    fn init(base: Base<Node2D>) -> Self {
        Self {
            team: 0,
            health: 100.0,
            base,
        }
    }

    fn ready(&mut self) {
        self.base_mut().add_to_group("enemies");
        let team_group = team_group_name(self.team);
        self.base_mut().add_to_group(&*team_group);
        godot_print!(
            "[Enemy] {} joined groups: enemies, team_{}",
            self.base().get_name(),
            self.team
        );
    }
}

#[godot_api]
impl Enemy {
    /// Reduces health by `damage` and prints the result.
    #[func]
    pub fn take_hit(&mut self, damage: f64) {
        self.health -= damage;
        godot_print!(
            "[Enemy] {} took {:.1} damage — health: {:.1}",
            self.base().get_name(),
            damage,
            self.health
        );
    }

    /// Returns `true` while this enemy still has positive health.
    #[func]
    pub fn is_alive(&self) -> bool {
        is_alive(self.health)
    }

    /// Returns the group name string for a given team index.
    ///
    /// Exposed as a static-style GDScript callable: `Enemy.get_team_group_name(1)`.
    #[func]
    pub fn get_team_group_name(team: i64) -> GString {
        (&team_group_name(team)).into()
    }
}

// ─── GroupManager node ───────────────────────────────────────────────────────

/// Scene root that demonstrates querying and dispatching to a node group.
///
/// Every 2 seconds it deals 10 damage to every node in the `"enemies"` group.
/// `nuke_all()` is a callable that instantly kills all enemies.
#[derive(GodotClass)]
#[class(base=Node2D)]
pub struct GroupManager {
    /// Accumulated time since the scene started.
    elapsed: f64,
    /// Tracks which 2-second interval we last processed.
    last_interval: u64,
    base: Base<Node2D>,
}

#[godot_api]
impl INode2D for GroupManager {
    fn init(base: Base<Node2D>) -> Self {
        Self {
            elapsed: 0.0,
            last_interval: 0,
            base,
        }
    }

    fn ready(&mut self) {
        let count = self.count_enemies();
        godot_print!("[GroupManager] Scene ready — {} enemies registered.", count);
    }

    fn process(&mut self, delta: f64) {
        self.elapsed += delta;
        let current_interval = intervals_elapsed(self.elapsed, 2.0);
        if current_interval > self.last_interval {
            self.last_interval = current_interval;
            self.deal_damage_to_all(10.0);
        }
    }
}

#[godot_api]
impl GroupManager {
    /// Returns the number of nodes currently in the `"enemies"` group.
    #[func]
    pub fn count_enemies(&mut self) -> i64 {
        let tree = self.base().get_tree();
        let members = tree.get_nodes_in_group("enemies");
        members.len() as i64
    }

    /// Deals `damage` to every enemy in the `"enemies"` group.
    #[func]
    pub fn deal_damage_to_all(&mut self, damage: f64) {
        let tree = self.base().get_tree();
        let members = tree.get_nodes_in_group("enemies");
        godot_print!(
            "[GroupManager] Dealing {:.1} damage to {} enemies.",
            damage,
            members.len()
        );
        for node in members.iter_shared() {
            if let Ok(mut enemy) = node.try_cast::<Enemy>() {
                enemy.bind_mut().take_hit(damage);
            }
        }
    }

    /// Instantly kills every enemy by dealing 9999 damage to each.
    #[func]
    pub fn nuke_all(&mut self) {
        godot_print!("[GroupManager] NUKE — wiping all enemies.");
        self.deal_damage_to_all(9999.0);
    }
}

// ─── Tests ────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    // team_group_name

    #[test]
    fn team_group_name_zero() {
        assert_eq!(team_group_name(0), "team_0");
    }

    #[test]
    fn team_group_name_positive() {
        assert_eq!(team_group_name(3), "team_3");
        assert_eq!(team_group_name(99), "team_99");
    }

    #[test]
    fn team_group_name_negative() {
        assert_eq!(team_group_name(-1), "team_-1");
    }

    // is_alive

    #[test]
    fn is_alive_full_health() {
        assert!(is_alive(100.0));
    }

    #[test]
    fn is_alive_low_health() {
        assert!(is_alive(0.001));
    }

    #[test]
    fn is_alive_exactly_zero() {
        assert!(!is_alive(0.0));
    }

    #[test]
    fn is_alive_negative_health() {
        assert!(!is_alive(-50.0));
    }

    // intervals_elapsed

    #[test]
    fn intervals_elapsed_none() {
        assert_eq!(intervals_elapsed(1.9, 2.0), 0);
    }

    #[test]
    fn intervals_elapsed_exact_boundary() {
        assert_eq!(intervals_elapsed(2.0, 2.0), 1);
    }

    #[test]
    fn intervals_elapsed_multiple() {
        assert_eq!(intervals_elapsed(5.0, 2.0), 2);
        assert_eq!(intervals_elapsed(6.0, 2.0), 3);
    }

    #[test]
    fn intervals_elapsed_zero_interval_returns_zero() {
        assert_eq!(intervals_elapsed(100.0, 0.0), 0);
    }

    #[test]
    fn intervals_elapsed_negative_interval_returns_zero() {
        assert_eq!(intervals_elapsed(10.0, -1.0), 0);
    }

    #[test]
    fn intervals_elapsed_zero_time() {
        assert_eq!(intervals_elapsed(0.0, 2.0), 0);
    }
}
