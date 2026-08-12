//! Spawner GDExtension demo — demonstrates dynamic node creation from Rust.
//!
//! Shows the core patterns for runtime node instantiation in Godot 4 via gdext:
//!
//! - Creating a custom Rust class instance with `Gd::from_init_fn`.
//! - Adding it as a child of the scene root with `add_child`.
//! - Moving spawned nodes each frame by mutating their position.
//! - Removing nodes with `queue_free` when their lifetime expires.
//!
//! Useful as a foundation for projectile systems, particle effects, and
//! procedural generation patterns.

use godot::prelude::*;

// ─── Extension entry point ───────────────────────────────────────────────────

/// Declares this crate as a GDExtension library.
struct SpawnerExt;

#[gdextension]
unsafe impl ExtensionLibrary for SpawnerExt {}

// ─── Pure functions ───────────────────────────────────────────────────────────

/// Returns `true` if `age` has reached or exceeded `lifetime`.
///
/// # Examples
/// ```
/// assert!(spawner::is_expired(2.0, 2.0));
/// assert!(spawner::is_expired(3.0, 2.0));
/// assert!(!spawner::is_expired(1.9, 2.0));
/// assert!(!spawner::is_expired(0.0, 2.0));
/// ```
pub fn is_expired(age: f64, lifetime: f64) -> bool {
    age >= lifetime
}

/// Returns whether spawning should occur: `time_since >= interval` AND `current_count < max_count`.
///
/// # Examples
/// ```
/// assert!(spawner::should_spawn(0.5, 0.5, 0, 20));
/// assert!(!spawner::should_spawn(0.4, 0.5, 0, 20));
/// assert!(!spawner::should_spawn(1.0, 0.5, 20, 20));
/// assert!(spawner::should_spawn(1.0, 0.5, 19, 20));
/// ```
pub fn should_spawn(
    time_since_last: f64,
    interval: f64,
    current_count: i64,
    max_count: i64,
) -> bool {
    time_since_last >= interval && current_count < max_count
}

/// Computes the new position after moving at velocity `(vx, vy)` for `delta` seconds.
///
/// # Examples
/// ```
/// let (nx, ny) = spawner::move_by(0.0, 0.0, 0.0, -300.0, 1.0);
/// assert!((nx - 0.0).abs() < 1e-4);
/// assert!((ny - (-300.0)).abs() < 1e-2);
/// ```
pub fn move_by(x: f32, y: f32, vx: f32, vy: f32, delta: f32) -> (f32, f32) {
    (x + vx * delta, y + vy * delta)
}

// ─── Bullet node ─────────────────────────────────────────────────────────────

/// A projectile spawned at runtime by `BulletSpawner`.
///
/// Each frame the bullet moves in the direction of `velocity` and accumulates
/// age.  When `age` reaches `lifetime` it removes itself from the scene with
/// `queue_free`.
#[derive(GodotClass)]
#[class(base=Node2D)]
pub struct Bullet {
    /// Direction and speed of travel (pixels per second).
    #[export]
    pub velocity: Vector2,
    /// How many seconds the bullet lives before being freed.
    #[export]
    pub lifetime: f64,
    /// Time in seconds since this bullet was spawned (not exported).
    pub age: f64,
    base: Base<Node2D>,
}

#[godot_api]
impl INode2D for Bullet {
    fn init(base: Base<Node2D>) -> Self {
        Self {
            velocity: Vector2::new(0.0, -300.0),
            lifetime: 2.0,
            age: 0.0,
            base,
        }
    }

    fn process(&mut self, delta: f64) {
        let pos = self.base().get_position();
        let (nx, ny) = move_by(pos.x, pos.y, self.velocity.x, self.velocity.y, delta as f32);
        self.base_mut().set_position(Vector2::new(nx, ny));
        self.age += delta;
        if is_expired(self.age, self.lifetime) {
            self.base_mut().queue_free();
        }
    }
}

#[godot_api]
impl Bullet {
    /// Returns `true` if this bullet has exceeded its lifetime.
    #[func]
    pub fn is_expired(&self) -> bool {
        is_expired(self.age, self.lifetime)
    }
}

// ─── BulletSpawner node ───────────────────────────────────────────────────────

/// Scene root that dynamically creates `Bullet` children at a configurable rate.
///
/// Every `spawn_interval` seconds, one `Bullet` is added as a child of this
/// node — up to `max_bullets` live children at once.  Bullets remove themselves
/// when their lifetime expires.
#[derive(GodotClass)]
#[class(base=Node2D)]
pub struct BulletSpawner {
    /// Seconds between each spawn.
    #[export]
    spawn_interval: f64,
    /// Maximum number of live bullet children at any moment.
    #[export]
    max_bullets: i64,
    /// Time accumulated since the last spawn.
    time_since_spawn: f64,
    /// Total bullets ever spawned (monotonically increasing).
    spawn_count: i64,
    base: Base<Node2D>,
}

#[godot_api]
impl INode2D for BulletSpawner {
    fn init(base: Base<Node2D>) -> Self {
        Self {
            spawn_interval: 0.5,
            max_bullets: 20,
            time_since_spawn: 0.0,
            spawn_count: 0,
            base,
        }
    }

    fn ready(&mut self) {
        godot_print!(
            "[BulletSpawner] Ready — interval: {:.2}s, max: {}",
            self.spawn_interval,
            self.max_bullets
        );
    }

    fn process(&mut self, delta: f64) {
        self.time_since_spawn += delta;
        let child_count = self.base().get_child_count() as i64;
        if should_spawn(
            self.time_since_spawn,
            self.spawn_interval,
            child_count,
            self.max_bullets,
        ) {
            self.time_since_spawn = 0.0;
            self.spawn_bullet();
        }
    }
}

#[godot_api]
impl BulletSpawner {
    /// Returns the total number of bullets ever spawned by this node.
    #[func]
    pub fn get_spawn_count(&self) -> i64 {
        self.spawn_count
    }

    /// Removes all live `Bullet` children by calling `queue_free` on each.
    #[func]
    pub fn clear_bullets(&mut self) {
        let child_count = self.base().get_child_count();
        let children: Vec<Gd<Node>> = (0..child_count)
            .filter_map(|i| self.base().get_child(i))
            .collect();
        for child in children {
            if let Ok(mut bullet) = child.try_cast::<Bullet>() {
                bullet.queue_free();
            }
        }
        godot_print!("[BulletSpawner] Cleared all bullets.");
    }

    /// Creates one `Bullet` instance at the origin and adds it as a child.
    fn spawn_bullet(&mut self) {
        let bullet = Gd::<Bullet>::from_init_fn(|base| Bullet {
            velocity: Vector2::new(0.0, -300.0),
            lifetime: 2.0,
            age: 0.0,
            base,
        });
        let node: Gd<Node> = bullet.upcast();
        self.base_mut().add_child(&node);
        self.spawn_count += 1;
        godot_print!("[BulletSpawner] Spawned bullet #{}", self.spawn_count);
    }
}

// ─── Tests ────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    // is_expired

    #[test]
    fn is_expired_at_exact_lifetime() {
        assert!(is_expired(2.0, 2.0));
    }

    #[test]
    fn is_expired_past_lifetime() {
        assert!(is_expired(3.5, 2.0));
    }

    #[test]
    fn is_expired_before_lifetime() {
        assert!(!is_expired(1.9, 2.0));
    }

    #[test]
    fn is_expired_zero_age() {
        assert!(!is_expired(0.0, 2.0));
    }

    #[test]
    fn is_expired_zero_lifetime() {
        assert!(is_expired(0.0, 0.0));
    }

    // should_spawn

    #[test]
    fn should_spawn_ready_with_room() {
        assert!(should_spawn(0.5, 0.5, 0, 20));
    }

    #[test]
    fn should_spawn_time_not_elapsed() {
        assert!(!should_spawn(0.4, 0.5, 0, 20));
    }

    #[test]
    fn should_spawn_at_max_capacity() {
        assert!(!should_spawn(1.0, 0.5, 20, 20));
    }

    #[test]
    fn should_spawn_one_below_max() {
        assert!(should_spawn(1.0, 0.5, 19, 20));
    }

    #[test]
    fn should_spawn_over_capacity_blocked() {
        assert!(!should_spawn(99.0, 0.5, 25, 20));
    }

    // move_by

    #[test]
    fn move_by_zero_velocity() {
        let (nx, ny) = move_by(10.0, 20.0, 0.0, 0.0, 0.016);
        assert!((nx - 10.0).abs() < 1e-5);
        assert!((ny - 20.0).abs() < 1e-5);
    }

    #[test]
    fn move_by_upward_velocity() {
        let (nx, ny) = move_by(0.0, 0.0, 0.0, -300.0, 1.0);
        assert!((nx - 0.0).abs() < 1e-4);
        assert!((ny - (-300.0)).abs() < 1e-2);
    }

    #[test]
    fn move_by_diagonal() {
        let (nx, ny) = move_by(100.0, 100.0, 200.0, -100.0, 0.5);
        assert!((nx - 200.0).abs() < 1e-3);
        assert!((ny - 50.0).abs() < 1e-3);
    }

    #[test]
    fn move_by_small_delta() {
        let (_nx, ny) = move_by(0.0, 0.0, 0.0, -300.0, 0.016);
        assert!((ny - (-4.8)).abs() < 0.01);
    }
}
