//! Exported Properties GDExtension demo — showcasing the full range of
//! `#[export]` field types visible in the Godot 4 inspector.
//!
//! Teaches: exporting `f64`, `i64`, `bool`, `GString`, `Color`, and `Vector2`
//! fields; the distinction between exported (inspector-visible) and private
//! (internal) fields; and periodic logic driven by `process()`.

use godot::prelude::*;

// ─── Extension entry point ───────────────────────────────────────────────────

/// Declares this crate as a GDExtension library.
struct ExportedPropertiesExt;

#[gdextension]
unsafe impl ExtensionLibrary for ExportedPropertiesExt {}

// ─── PropertyShowcase node ────────────────────────────────────────────────────

/// Demonstrates every common `#[export]` type alongside a private field.
///
/// All six exported fields appear in the Godot inspector and are readable from
/// GDScript.  The private `tick` field is internal state only — Godot never
/// sees it.
#[derive(GodotClass)]
#[class(base=Node2D)]
pub struct PropertyShowcase {
    /// Movement speed in pixels per second.
    #[export]
    speed: f64,
    /// Player health points.
    #[export]
    health: i64,
    /// Whether the node is active.
    #[export]
    enabled: bool,
    /// Text shown on the associated label.
    #[export]
    label_text: GString,
    /// Colour tint applied to sprites.
    #[export]
    tint: Color,
    /// Positional offset from the parent.
    #[export]
    offset: Vector2,
    /// Internal accumulator — not exported to Godot.
    tick: f64,
    base: Base<Node2D>,
}

#[godot_api]
impl INode2D for PropertyShowcase {
    fn init(base: Base<Node2D>) -> Self {
        Self {
            speed: 150.0,
            health: 100,
            enabled: true,
            label_text: GString::from("Showcase"),
            tint: Color::WHITE,
            offset: Vector2::ZERO,
            tick: 0.0,
            base,
        }
    }

    fn ready(&mut self) {
        godot_print!(
            "speed={} health={} enabled={} label_text={} tint=({},{},{}) offset=({},{})",
            self.speed,
            self.health,
            self.enabled,
            self.label_text,
            self.tint.r,
            self.tint.g,
            self.tint.b,
            self.offset.x,
            self.offset.y,
        );
    }

    fn process(&mut self, delta: f64) {
        self.tick += delta;
        let elapsed = ticks_elapsed(self.tick, 1.0);
        if elapsed > ticks_elapsed(self.tick - delta, 1.0) {
            godot_print!("tick: {}", elapsed);
        }
    }
}

#[godot_api]
impl PropertyShowcase {
    /// Returns a human-readable summary of all exported values.
    #[func]
    pub fn describe(&self) -> GString {
        GString::from(format_stats(self.speed, self.health, self.enabled).as_str())
    }

    /// Resets the internal tick accumulator to zero.
    #[func]
    pub fn reset_tick(&mut self) {
        self.tick = 0.0;
    }
}

// ─── Pure functions ───────────────────────────────────────────────────────────

/// Returns a human-readable summary string for a set of stats.
pub fn format_stats(speed: f64, health: i64, enabled: bool) -> String {
    let state = if enabled { "active" } else { "inactive" };
    format!("speed={speed:.1} health={health} [{state}]")
}

/// Returns `true` if `speed` is in the valid range `0.0 ..= 1000.0`.
pub fn is_valid_speed(speed: f64) -> bool {
    (0.0..=1000.0).contains(&speed)
}

/// Computes how many full ticks have elapsed given `total_time` and a
/// `tick_duration`.  Returns 0 when `tick_duration` is zero or negative.
pub fn ticks_elapsed(total_time: f64, tick_duration: f64) -> u64 {
    if tick_duration <= 0.0 {
        return 0;
    }
    (total_time / tick_duration) as u64
}

// ─── Tests ────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn format_stats_active() {
        let s = format_stats(150.0, 100, true);
        assert!(s.contains("speed=150.0"));
        assert!(s.contains("health=100"));
        assert!(s.contains("active"));
    }

    #[test]
    fn format_stats_inactive() {
        let s = format_stats(0.0, 0, false);
        assert!(s.contains("inactive"));
    }

    #[test]
    fn is_valid_speed_boundaries() {
        assert!(is_valid_speed(0.0));
        assert!(is_valid_speed(1000.0));
        assert!(is_valid_speed(150.0));
    }

    #[test]
    fn is_valid_speed_out_of_range() {
        assert!(!is_valid_speed(-1.0));
        assert!(!is_valid_speed(1000.1));
        assert!(!is_valid_speed(f64::INFINITY));
    }

    #[test]
    fn ticks_elapsed_basic() {
        assert_eq!(ticks_elapsed(0.0, 1.0), 0);
        assert_eq!(ticks_elapsed(1.0, 1.0), 1);
        assert_eq!(ticks_elapsed(2.9, 1.0), 2);
        assert_eq!(ticks_elapsed(3.0, 1.0), 3);
    }

    #[test]
    fn ticks_elapsed_fractional_duration() {
        assert_eq!(ticks_elapsed(1.0, 0.5), 2);
        assert_eq!(ticks_elapsed(1.5, 0.5), 3);
    }

    #[test]
    fn ticks_elapsed_zero_duration_guard() {
        assert_eq!(ticks_elapsed(100.0, 0.0), 0);
        assert_eq!(ticks_elapsed(100.0, -1.0), 0);
    }
}
