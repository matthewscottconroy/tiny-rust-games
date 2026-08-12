//! Signals GDExtension demo — Rust-defined `#[signal]` declarations, typed
//! `emit_signal` calls, and connecting signals between nodes in `ready()`.
//!
//! Teaches: `#[signal]` declaration syntax, `emit_signal` with variant arguments,
//! `connect` for wiring nodes together, and the node-tree relationship between a
//! parent scene node and a reusable component child.

use godot::prelude::*;

// ─── Extension entry point ────────────────────────────────────────────────────

/// Declares this crate as a GDExtension library.
struct SignalsExt;

#[gdextension]
unsafe impl ExtensionLibrary for SignalsExt {}

// ─── HealthComponent node ─────────────────────────────────────────────────────

/// A reusable `Node` that tracks hit points and emits signals on health changes.
///
/// Attach this as a child of any scene node that needs health tracking. Connect
/// `health_changed` and `health_depleted` to react to damage and death events
/// without coupling the component to its parent.
#[derive(GodotClass)]
#[class(base=Node)]
pub struct HealthComponent {
    /// Maximum health points. Also used as the starting value when the scene loads.
    #[export]
    max_health: i64,
    current_health: i64,
    base: Base<Node>,
}

#[godot_api]
impl INode for HealthComponent {
    fn init(base: Base<Node>) -> Self {
        Self {
            max_health: 100,
            current_health: 100,
            base,
        }
    }

    fn ready(&mut self) {
        self.current_health = self.max_health;
        godot_print!("HealthComponent ready — max_health={}", self.max_health);
    }
}

#[godot_api]
impl HealthComponent {
    /// Emitted whenever health changes. `new_health` is the value after the
    /// change; `delta` is the signed amount (negative for damage, positive for
    /// healing).
    #[signal]
    fn health_changed(new_health: i64, delta: i64);

    /// Emitted once when health reaches zero or below.
    #[signal]
    fn health_depleted();

    /// Reduces current health by `amount`. Clamps to zero, then emits
    /// `health_changed` (and `health_depleted` if health has reached zero).
    #[func]
    pub fn take_damage(&mut self, amount: i64) {
        let before = self.current_health;
        self.current_health = clamp_health(self.current_health - amount, self.max_health);
        let new_health = self.current_health;
        let delta = new_health - before;
        let depleted = new_health <= 0;
        self.base_mut().emit_signal(
            "health_changed",
            &[new_health.to_variant(), delta.to_variant()],
        );
        if depleted {
            self.base_mut().emit_signal("health_depleted", &[]);
        }
    }

    /// Increases current health by `amount`, clamped to `max_health`. Emits
    /// `health_changed` with a positive delta.
    #[func]
    pub fn heal(&mut self, amount: i64) {
        let before = self.current_health;
        self.current_health = clamp_health(self.current_health + amount, self.max_health);
        let new_health = self.current_health;
        let delta = new_health - before;
        self.base_mut().emit_signal(
            "health_changed",
            &[new_health.to_variant(), delta.to_variant()],
        );
    }

    /// Returns the current health value.
    #[func]
    pub fn get_health(&self) -> i64 {
        self.current_health
    }

    /// Returns current health as a fraction of max health in `[0.0, 1.0]`.
    #[func]
    pub fn get_health_fraction(&self) -> f64 {
        health_fraction(self.current_health, self.max_health)
    }
}

// ─── SignalDemo node ──────────────────────────────────────────────────────────

/// A `Node2D` that owns a `HealthComponent` child, connects its signals in
/// `ready()`, and demonstrates how to wire Rust-defined signals to Rust methods.
///
/// Place this as the root of the demo scene with a `HealthComponent` child named
/// `"HealthComponent"`. The component will deal 30 damage after a short delay
/// (controlled by the `damage_timer` accumulator) to show the signal flow.
#[derive(GodotClass)]
#[class(base=Node2D)]
pub struct SignalDemo {
    damage_timer: f64,
    base: Base<Node2D>,
}

#[godot_api]
impl INode2D for SignalDemo {
    fn init(base: Base<Node2D>) -> Self {
        Self {
            damage_timer: 0.0,
            base,
        }
    }

    fn ready(&mut self) {
        godot_print!("SignalDemo ready — connecting health signals");

        // Connect the HealthComponent's signal to our handler method.
        let callable = self.base().callable("on_health_changed");
        let mut health = self
            .base()
            .get_node_as::<HealthComponent>("HealthComponent");
        health.connect("health_changed", &callable);
    }

    fn process(&mut self, delta: f64) {
        self.damage_timer += delta;
        // Deal 30 damage once, approximately one second after the scene loads.
        if self.damage_timer > 1.0 && self.damage_timer < 1.0 + delta + 0.001 {
            let mut health = self
                .base()
                .get_node_as::<HealthComponent>("HealthComponent");
            health.bind_mut().take_damage(30);
        }
    }
}

#[godot_api]
impl SignalDemo {
    /// Called when the `HealthComponent` emits `health_changed`.
    #[func]
    pub fn on_health_changed(&mut self, new_health: i64, delta: i64) {
        godot_print!("Health changed: new_health={}, delta={}", new_health, delta);
    }
}

// ─── Pure functions ───────────────────────────────────────────────────────────

/// Clamps `value` to `[0, max]`. Negative `max` is treated as zero.
pub fn clamp_health(value: i64, max: i64) -> i64 {
    let ceiling = max.max(0);
    value.clamp(0, ceiling)
}

/// Returns current health as a fraction of max health in `[0.0, 1.0]`.
/// Returns `0.0` when `max` is zero or negative to avoid division by zero.
pub fn health_fraction(current: i64, max: i64) -> f64 {
    if max <= 0 {
        return 0.0;
    }
    (current as f64 / max as f64).clamp(0.0, 1.0)
}

/// Returns the effective damage after applying a damage-reduction factor in
/// `[0, 1]`. A `reduction` of `0.0` passes the full `raw` damage through;
/// `1.0` negates all damage. Values outside `[0, 1]` are clamped.
pub fn effective_damage(raw: i64, reduction: f64) -> i64 {
    let r = reduction.clamp(0.0, 1.0);
    (raw as f64 * (1.0 - r)).round() as i64
}

// ─── Tests ────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    // clamp_health ─────────────────────────────────────────────────────────────

    #[test]
    fn clamp_health_normal_range() {
        assert_eq!(clamp_health(50, 100), 50);
    }

    #[test]
    fn clamp_health_below_zero_clamps_to_zero() {
        assert_eq!(clamp_health(-10, 100), 0);
    }

    #[test]
    fn clamp_health_above_max_clamps_to_max() {
        assert_eq!(clamp_health(150, 100), 100);
    }

    #[test]
    fn clamp_health_exactly_zero() {
        assert_eq!(clamp_health(0, 100), 0);
    }

    #[test]
    fn clamp_health_exactly_max() {
        assert_eq!(clamp_health(100, 100), 100);
    }

    #[test]
    fn clamp_health_negative_max_treated_as_zero() {
        assert_eq!(clamp_health(50, -1), 0);
    }

    // health_fraction ──────────────────────────────────────────────────────────

    #[test]
    fn health_fraction_full_health() {
        assert_eq!(health_fraction(100, 100), 1.0);
    }

    #[test]
    fn health_fraction_half_health() {
        assert!((health_fraction(50, 100) - 0.5).abs() < 1e-9);
    }

    #[test]
    fn health_fraction_zero_health() {
        assert_eq!(health_fraction(0, 100), 0.0);
    }

    #[test]
    fn health_fraction_zero_max_returns_zero() {
        assert_eq!(health_fraction(50, 0), 0.0);
    }

    #[test]
    fn health_fraction_over_max_clamped_to_one() {
        assert_eq!(health_fraction(150, 100), 1.0);
    }

    // effective_damage ─────────────────────────────────────────────────────────

    #[test]
    fn effective_damage_no_reduction() {
        assert_eq!(effective_damage(100, 0.0), 100);
    }

    #[test]
    fn effective_damage_full_reduction() {
        assert_eq!(effective_damage(100, 1.0), 0);
    }

    #[test]
    fn effective_damage_half_reduction() {
        assert_eq!(effective_damage(100, 0.5), 50);
    }

    #[test]
    fn effective_damage_reduction_over_one_clamped() {
        assert_eq!(effective_damage(100, 2.0), 0);
    }

    #[test]
    fn effective_damage_reduction_below_zero_clamped() {
        assert_eq!(effective_damage(100, -1.0), 100);
    }

    #[test]
    fn effective_damage_rounds_correctly() {
        // 10 * (1 - 0.33) = 6.7 → rounds to 7
        assert_eq!(effective_damage(10, 0.33), 7);
    }
}
