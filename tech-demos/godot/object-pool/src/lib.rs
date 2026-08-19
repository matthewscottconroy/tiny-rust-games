//! Object Pool demo — pre-spawned nodes recycled via visibility and
//! ProcessMode instead of repeated spawn/free cycles.
//!
//! Teaches: pre-allocating a fixed pool of `Sprite2D` child nodes in `ready()`;
//! tracking active/inactive state through `is_visible()`; moving active nodes
//! each frame with a parallel velocity vector; deactivating out-of-bounds nodes;
//! and resetting the entire pool in one call.
//!
//! The pool avoids the overhead of `new_alloc` + `queue_free` per projectile —
//! nodes are simply shown/hidden and their process mode toggled.
//!
//! Concept: object-pooling

use godot::classes::{INode2D, Label, Node, Node2D, Sprite2D};
use godot::prelude::*;

// ---------------------------------------------------------------------------
// Extension entry-point
// ---------------------------------------------------------------------------

struct ObjectPoolExtension;
#[gdextension]
unsafe impl ExtensionLibrary for ObjectPoolExtension {}

// ---------------------------------------------------------------------------
// ObjectPool node
// ---------------------------------------------------------------------------

/// Manages a pre-allocated pool of `Sprite2D` "bullet" nodes.
///
/// Place this as the scene root.  A child `Label` named `"StatsLabel"` shows
/// active/total counts each frame.
#[derive(GodotClass)]
#[class(base=Node2D)]
pub struct ObjectPool {
    /// Number of slots pre-allocated in the pool.
    #[export]
    pool_size: i32,

    /// Movement speed of active projectiles in pixels per second.
    #[export]
    projectile_speed: f32,

    /// Pre-allocated child nodes.
    pool: Vec<Gd<Sprite2D>>,

    /// Per-slot velocity (dx, dy).  Index matches `pool`.
    velocities: Vec<(f32, f32)>,

    base: Base<Node2D>,
}

#[godot_api]
impl INode2D for ObjectPool {
    fn init(base: Base<Node2D>) -> Self {
        Self {
            pool_size: 10,
            projectile_speed: 300.0,
            pool: Vec::new(),
            velocities: Vec::new(),
            base,
        }
    }

    fn ready(&mut self) {
        let size = self.pool_size as usize;
        godot_print!("[ObjectPool] pre-allocating {} slots", size);

        for i in 0..size {
            let mut sprite = Sprite2D::new_alloc();
            sprite.set_visible(false);
            // Disable processing while inactive so the node costs nothing.
            sprite.set_process_mode(godot::classes::node::ProcessMode::DISABLED);

            let node = sprite.upcast::<Node>();
            self.base_mut().add_child(&node);

            // Retrieve the node back by its auto-assigned name ("Sprite2D",
            // "Sprite2D2", …).  We store the Gd handle in our pool Vec.
            let child: Gd<Sprite2D> = self
                .base()
                .get_child(i as i32)
                .expect("child just added")
                .cast::<Sprite2D>();
            self.pool.push(child);
            self.velocities.push((0.0, 0.0));
        }

        self.refresh_label();
    }

    fn process(&mut self, delta: f64) {
        let speed = self.projectile_speed;
        let limit = 600.0_f32;

        let size = self.pool.len();
        for i in 0..size {
            let visible = self.pool[i].is_visible();
            if !visible {
                continue;
            }

            let (vx, vy) = self.velocities[i];
            let pos = self.pool[i].get_position();
            let (nx, ny) = move_projectile(pos.x, pos.y, vx * speed, vy * speed, delta as f32);

            if is_out_of_bounds(nx, ny, limit) {
                // Return to pool.
                self.pool[i].set_visible(false);
                self.pool[i].set_process_mode(godot::classes::node::ProcessMode::DISABLED);
                self.velocities[i] = (0.0, 0.0);
            } else {
                self.pool[i].set_position(Vector2::new(nx, ny));
            }
        }

        self.refresh_label();
    }
}

#[godot_api]
impl ObjectPool {
    /// Fires a projectile in the given direction if a free pool slot exists.
    ///
    /// The direction vector is normalised inside this function so callers
    /// don't need to normalise it themselves.
    #[func]
    pub fn fire(&mut self, direction_x: f32, direction_y: f32) {
        let active: Vec<bool> = self.pool.iter().map(|n| n.is_visible()).collect();
        if let Some(idx) = find_inactive(&active) {
            // Normalise the direction.
            let len = (direction_x * direction_x + direction_y * direction_y).sqrt();
            let (nx, ny) = if len > 1e-6 {
                (direction_x / len, direction_y / len)
            } else {
                (0.0, -1.0) // Default: upward.
            };

            self.pool[idx].set_position(Vector2::ZERO);
            self.pool[idx].set_visible(true);
            self.pool[idx].set_process_mode(godot::classes::node::ProcessMode::INHERIT);
            self.velocities[idx] = (nx, ny);
            godot_print!("[ObjectPool] slot {} fired dir=({:.2},{:.2})", idx, nx, ny);
        } else {
            godot_print!("[ObjectPool] pool exhausted — no free slots");
        }
    }

    /// Returns the number of currently active (visible) pool slots.
    #[func]
    pub fn get_active_count(&self) -> i32 {
        let active: Vec<bool> = self.pool.iter().map(|n| n.is_visible()).collect();
        pool_stats(&active).0 as i32
    }

    /// Deactivates all pool slots, returning them to standby.
    #[func]
    pub fn reset_pool(&mut self) {
        for i in 0..self.pool.len() {
            self.pool[i].set_visible(false);
            self.pool[i].set_process_mode(godot::classes::node::ProcessMode::DISABLED);
            self.velocities[i] = (0.0, 0.0);
        }
        godot_print!("[ObjectPool] pool reset");
        self.refresh_label();
    }

    // Internal: refresh the stats label.
    fn refresh_label(&mut self) {
        if let Some(mut label) = self.base().try_get_node_as::<Label>("StatsLabel") {
            let active: Vec<bool> = self.pool.iter().map(|n| n.is_visible()).collect();
            let (a, t) = pool_stats(&active);
            let text = format!("Active: {}/{}", a, t);
            label.set_text(text.as_str());
        }
    }
}

// ---------------------------------------------------------------------------
// Pure helper functions
// ---------------------------------------------------------------------------

/// Returns the index of the first `false` entry in `active`, or `None` if all
/// slots are occupied.
///
/// # Examples
/// ```
/// assert_eq!(object_pool::find_inactive(&[true, false, true]), Some(1));
/// assert_eq!(object_pool::find_inactive(&[true, true]), None);
/// assert_eq!(object_pool::find_inactive(&[false]), Some(0));
/// ```
pub fn find_inactive(active: &[bool]) -> Option<usize> {
    active.iter().position(|&a| !a)
}

/// Returns `true` when the position exceeds `limit` units from the origin in
/// either axis.
///
/// # Examples
/// ```
/// assert!(object_pool::is_out_of_bounds(700.0, 0.0, 600.0));
/// assert!(!object_pool::is_out_of_bounds(100.0, 200.0, 600.0));
/// assert!(object_pool::is_out_of_bounds(0.0, -700.0, 600.0));
/// ```
pub fn is_out_of_bounds(x: f32, y: f32, limit: f32) -> bool {
    x.abs() > limit || y.abs() > limit
}

/// Returns `(active_count, total_count)` from a boolean activity slice.
///
/// # Examples
/// ```
/// assert_eq!(object_pool::pool_stats(&[true, false, true, false, false]), (2, 5));
/// assert_eq!(object_pool::pool_stats(&[]), (0, 0));
/// ```
pub fn pool_stats(active: &[bool]) -> (usize, usize) {
    let total = active.len();
    let count = active.iter().filter(|&&a| a).count();
    (count, total)
}

/// Advances a projectile position by velocity × delta.
pub fn move_projectile(x: f32, y: f32, vx: f32, vy: f32, delta: f32) -> (f32, f32) {
    (x + vx * delta, y + vy * delta)
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    // find_inactive -----------------------------------------------------------

    #[test]
    fn find_inactive_first_slot_free() {
        assert_eq!(find_inactive(&[false, true, true]), Some(0));
    }

    #[test]
    fn find_inactive_middle_slot_free() {
        assert_eq!(find_inactive(&[true, false, true]), Some(1));
    }

    #[test]
    fn find_inactive_all_active_returns_none() {
        assert_eq!(find_inactive(&[true, true, true]), None);
    }

    #[test]
    fn find_inactive_empty_slice_returns_none() {
        assert_eq!(find_inactive(&[]), None);
    }

    #[test]
    fn find_inactive_single_false() {
        assert_eq!(find_inactive(&[false]), Some(0));
    }

    #[test]
    fn find_inactive_returns_first_not_last() {
        assert_eq!(find_inactive(&[false, false, true]), Some(0));
    }

    // is_out_of_bounds --------------------------------------------------------

    #[test]
    fn is_out_of_bounds_within_limit() {
        assert!(!is_out_of_bounds(100.0, 200.0, 600.0));
    }

    #[test]
    fn is_out_of_bounds_exceeds_x() {
        assert!(is_out_of_bounds(700.0, 0.0, 600.0));
    }

    #[test]
    fn is_out_of_bounds_exceeds_y_negative() {
        assert!(is_out_of_bounds(0.0, -700.0, 600.0));
    }

    #[test]
    fn is_out_of_bounds_exactly_at_limit_not_out() {
        assert!(!is_out_of_bounds(600.0, 0.0, 600.0));
    }

    // pool_stats --------------------------------------------------------------

    #[test]
    fn pool_stats_mixed() {
        assert_eq!(pool_stats(&[true, false, true, false, false]), (2, 5));
    }

    #[test]
    fn pool_stats_all_active() {
        assert_eq!(pool_stats(&[true, true, true]), (3, 3));
    }

    #[test]
    fn pool_stats_none_active() {
        assert_eq!(pool_stats(&[false, false]), (0, 2));
    }

    #[test]
    fn pool_stats_empty() {
        assert_eq!(pool_stats(&[]), (0, 0));
    }
}
