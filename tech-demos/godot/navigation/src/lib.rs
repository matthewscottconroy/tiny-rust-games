//! Navigation GDExtension demo — NavigationAgent2D path-following from Rust.
//!
//! Demonstrates:
//!
//! - Subclassing `CharacterBody2D` and fetching a child `NavigationAgent2D`
//!   via `get_node_as` in `ready()`.
//! - Setting a target position each frame from mouse coordinates.
//! - Querying `get_next_path_position()` and moving toward it.
//! - Updating a `Label` child with "Navigating" / "Arrived" status.
//! - Pure movement helpers (`move_toward`, `arrived`) fully covered by tests.

use godot::classes::{CharacterBody2D, ICharacterBody2D, Label, NavigationAgent2D};
use godot::prelude::*;

// ─── Extension entry point ───────────────────────────────────────────────────

struct NavigationExt;

#[gdextension]
unsafe impl ExtensionLibrary for NavigationExt {}

// ─── Pure functions ───────────────────────────────────────────────────────────

/// Advances `from` toward `to` at `speed` pixels/sec over `delta` seconds.
///
/// Will not overshoot: if the remaining distance is smaller than `speed *
/// delta`, exactly `to` is returned.
///
/// # Examples
/// ```
/// // Moving forward
/// let x = navigation::move_toward(0.0, 10.0, 5.0, 1.0);
/// assert!((x - 5.0).abs() < 1e-5);
///
/// // Clamps at destination
/// let x = navigation::move_toward(9.0, 10.0, 5.0, 1.0);
/// assert!((x - 10.0).abs() < 1e-5);
///
/// // Moving backward
/// let x = navigation::move_toward(10.0, 0.0, 5.0, 1.0);
/// assert!((x - 5.0).abs() < 1e-5);
/// ```
pub fn move_toward(from: f32, to: f32, speed: f32, delta: f32) -> f32 {
    let step = speed * delta;
    let diff = to - from;
    if diff.abs() <= step {
        to
    } else {
        from + diff.signum() * step
    }
}

/// Returns `true` when the straight-line distance from `from` to `to` is less
/// than `threshold`.
///
/// # Examples
/// ```
/// assert!(navigation::arrived((0.0, 0.0), (3.0, 4.0), 6.0));
/// assert!(!navigation::arrived((0.0, 0.0), (3.0, 4.0), 4.9));
/// assert!(navigation::arrived((1.0, 1.0), (1.0, 1.0), 0.1));
/// ```
pub fn arrived(from: (f32, f32), to: (f32, f32), threshold: f32) -> bool {
    let dx = to.0 - from.0;
    let dy = to.1 - from.1;
    (dx * dx + dy * dy).sqrt() < threshold
}

// ─── NavigationAgent node ─────────────────────────────────────────────────────

/// A `CharacterBody2D` that follows the mouse cursor using `NavigationAgent2D`.
///
/// Requires two children in the scene:
/// - `NavigationAgent2D` (name: `"NavigationAgent2D"`)
/// - `Label`             (name: `"StatusLabel"`)
#[derive(GodotClass)]
#[class(base=CharacterBody2D)]
pub struct NavAgent {
    /// Movement speed in pixels per second.
    #[export]
    move_speed: f32,
    base: Base<CharacterBody2D>,
}

#[godot_api]
impl ICharacterBody2D for NavAgent {
    fn init(base: Base<CharacterBody2D>) -> Self {
        Self {
            move_speed: 120.0,
            base,
        }
    }

    fn ready(&mut self) {
        godot_print!("[NavAgent] Ready — speed: {}", self.move_speed);
    }

    fn process(&mut self, delta: f64) {
        // Retrieve child nodes each frame to avoid borrow conflicts.
        let mut nav_agent = self
            .base()
            .get_node_as::<NavigationAgent2D>("NavigationAgent2D");
        let mut label = self.base().get_node_as::<Label>("StatusLabel");

        // Point the agent at the current mouse position.
        let mouse_pos = self.base().get_global_mouse_position();
        nav_agent.set_target_position(mouse_pos);

        if nav_agent.is_navigation_finished() {
            label.set_text("Arrived");
            return;
        }

        label.set_text("Navigating");

        let next = nav_agent.get_next_path_position();
        let current_pos = self.base().get_global_position();
        let dx = next.x - current_pos.x;
        let dy = next.y - current_pos.y;
        let len = (dx * dx + dy * dy).sqrt();
        if len > 0.1 {
            let speed = self.move_speed;
            let vx = (dx / len) * speed;
            let vy = (dy / len) * speed;
            self.base_mut()
                .set_velocity(Vector2::new(vx, vy));
            // move_and_slide uses the velocity set above.
            let _ = delta; // delta not needed; move_and_slide uses physics delta internally
            self.base_mut().move_and_slide();
        }
    }
}

// ─── Tests ────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    // move_toward

    #[test]
    fn move_toward_forward_one_step() {
        let x = move_toward(0.0, 10.0, 5.0, 1.0);
        assert!((x - 5.0).abs() < 1e-5);
    }

    #[test]
    fn move_toward_clamps_at_destination() {
        let x = move_toward(9.0, 10.0, 5.0, 1.0);
        assert!((x - 10.0).abs() < 1e-5);
    }

    #[test]
    fn move_toward_backward() {
        let x = move_toward(10.0, 0.0, 5.0, 1.0);
        assert!((x - 5.0).abs() < 1e-5);
    }

    #[test]
    fn move_toward_already_there() {
        let x = move_toward(5.0, 5.0, 10.0, 1.0);
        assert!((x - 5.0).abs() < 1e-5);
    }

    #[test]
    fn move_toward_small_delta() {
        let x = move_toward(0.0, 100.0, 50.0, 0.1);
        assert!((x - 5.0).abs() < 1e-4);
    }

    // arrived

    #[test]
    fn arrived_within_threshold() {
        assert!(arrived((0.0, 0.0), (3.0, 4.0), 6.0));
    }

    #[test]
    fn arrived_outside_threshold() {
        assert!(!arrived((0.0, 0.0), (3.0, 4.0), 4.9));
    }

    #[test]
    fn arrived_at_same_position() {
        assert!(arrived((1.0, 1.0), (1.0, 1.0), 0.1));
    }

    #[test]
    fn arrived_exact_distance_is_not_arrived() {
        // Distance is exactly 5.0; threshold 5.0 requires strictly less.
        assert!(!arrived((0.0, 0.0), (3.0, 4.0), 5.0));
    }
}
