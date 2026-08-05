//! Hitbox / hurtbox combat demo — Godot 4.3 + gdext 0.5.
//!
//! Teaches:
//! - Separating the attack hitbox from the character body using Area2D.
//! - Detecting overlaps between a melee hitbox and enemy hurtboxes from Rust.
//! - Applying damage with a critical-hit multiplier via a pure function.
//! - Applying knockback to the victim's velocity as a pure function.
//! - The `area_entered` signal wired from Rust in `ready()`.
//!
//! The scene contains a single root node `HitboxArena` (Node2D) that spawns
//! the player and one enemy at runtime, letting Rust own all the layout logic.
//!
//! **Controls:** WASD — move player;  SPACE — swing attack.

use godot::classes::{
    Area2D, CircleShape2D, CollisionShape2D, IArea2D, INode2D, Input, Label, Node2D,
};
use godot::prelude::*;

// ─── Extension entry point ────────────────────────────────────────────────────

struct HitboxHurtboxExt;
#[gdextension]
unsafe impl ExtensionLibrary for HitboxHurtboxExt {}

// ─── Pure combat helpers ──────────────────────────────────────────────────────

/// Returns base damage, multiplied by 1.5 and rounded when `critical` is true.
pub fn compute_damage(base: i32, critical: bool) -> i32 {
    if critical {
        (base as f32 * 1.5).round() as i32
    } else {
        base
    }
}

/// Returns `velocity` with a knockback impulse added in `direction`.
pub fn apply_knockback(velocity: Vector2, direction: Vector2, force: f32) -> Vector2 {
    velocity + direction * force
}

/// Returns `true` when `health` is at or below zero.
pub fn is_dead(health: i32) -> bool {
    health <= 0
}

/// Clamps `health` into `[0, max_health]`.
pub fn clamp_health(health: i32, max_health: i32) -> i32 {
    health.clamp(0, max_health)
}

// ─── HitboxArena — root Node2D that owns spawning and the HUD ─────────────────

#[derive(GodotClass)]
#[class(base=Node2D)]
pub struct HitboxArena {
    base: Base<Node2D>,
}

#[godot_api]
impl INode2D for HitboxArena {
    fn init(base: Base<Node2D>) -> Self {
        Self { base }
    }

    fn ready(&mut self) {
        // HUD label
        let mut label = Label::new_alloc();
        label.set_text("WASD — move   SPACE — attack");
        label.set_position(Vector2::new(10.0, 8.0));
        self.base_mut().add_child(&label);

        // Spawn player
        let mut player = HitboxPlayer::new_alloc();
        player.set_position(Vector2::new(150.0, 300.0));
        self.base_mut().add_child(&player);

        // Spawn enemy
        let mut enemy = HurtboxEnemy::new_alloc();
        enemy.set_position(Vector2::new(500.0, 300.0));
        self.base_mut().add_child(&enemy);
    }
}

// ─── HitboxPlayer ─────────────────────────────────────────────────────────────

/// A Node2D player that can move and swing a melee hitbox.
#[derive(GodotClass)]
#[class(base=Node2D)]
pub struct HitboxPlayer {
    speed: f32,
    attack_cooldown: f32,
    attack_timer: f32,
    facing: Vector2,
    #[export]
    health: i32,
    base: Base<Node2D>,
}

#[godot_api]
impl INode2D for HitboxPlayer {
    fn init(base: Base<Node2D>) -> Self {
        Self {
            speed: 130.0,
            attack_cooldown: 0.35,
            attack_timer: 0.0,
            facing: Vector2::RIGHT,
            health: 100,
            base,
        }
    }

    fn ready(&mut self) {
        // Body visual
        let mut body = ColorRect::new_alloc();
        body.set_size(Vector2::new(28.0, 28.0));
        body.set_position(Vector2::new(-14.0, -14.0));
        body.set_color(Color::from_rgb(0.25, 0.6, 1.0));
        self.base_mut().add_child(&body);

        // Hitbox Area2D (starts invisible/disabled)
        let mut area = Area2D::new_alloc();
        area.set_name("Hitbox");
        let mut shape = CircleShape2D::new_gd();
        shape.set_radius(28.0);
        let mut col = CollisionShape2D::new_alloc();
        col.set_shape(&shape);
        col.set_disabled(true);
        area.add_child(&col);
        self.base_mut().add_child(&area);
    }

    fn process(&mut self, delta: f64) {
        let dt = delta as f32;
        let input = Input::singleton();

        // Movement
        let mut dir = Vector2::ZERO;
        if input.is_action_pressed("ui_right") { dir.x += 1.0; }
        if input.is_action_pressed("ui_left")  { dir.x -= 1.0; }
        if input.is_action_pressed("ui_down")  { dir.y += 1.0; }
        if input.is_action_pressed("ui_up")    { dir.y -= 1.0; }
        if dir != Vector2::ZERO {
            self.facing = dir.normalized();
        }
        // Copy the field first — `base_mut()` holds a mutable borrow of self.
        let speed = self.speed;
        let pos = self.base().get_position();
        self.base_mut().set_position(pos + dir.normalized() * speed * dt);

        // Attack cooldown
        if self.attack_timer > 0.0 {
            self.attack_timer -= dt;
        }

        // Swing attack — enable hitbox for one frame when SPACE is pressed
        if input.is_action_just_pressed("ui_accept") && self.attack_timer <= 0.0 {
            self.attack_timer = self.attack_cooldown;
            // Position hitbox in front of player and briefly enable it
            if let Some(mut area) = self.base().try_get_node_as::<Area2D>("Hitbox") {
                let offset = self.facing * 36.0;
                area.set_position(offset);
                // Enable the collision shape
                if let Some(child) = area.get_child(0) {
                    if let Ok(mut col) = child.try_cast::<CollisionShape2D>() {
                        col.set_disabled(false);
                    }
                }
            }
        } else if self.attack_timer <= self.attack_cooldown - 0.05 {
            // Disable hitbox after first frame of attack
            if let Some(area) = self.base().try_get_node_as::<Area2D>("Hitbox") {
                if let Some(child) = area.get_child(0) {
                    if let Ok(mut col) = child.try_cast::<CollisionShape2D>() {
                        col.set_disabled(true);
                    }
                }
            }
        }
    }
}

use godot::classes::ColorRect;

#[godot_api]
impl HitboxPlayer {
    // gdext auto-generates `get_health()` for the `#[export]` field, so this
    // accessor must use a different name.
    pub fn current_health(&self) -> i32 { self.health }
    pub fn get_facing(&self) -> Vector2 { self.facing }
}

// ─── HurtboxEnemy ─────────────────────────────────────────────────────────────

/// A Node2D enemy with an Area2D hurtbox that reacts to being hit.
#[derive(GodotClass)]
#[class(base=Node2D)]
pub struct HurtboxEnemy {
    #[export]
    health: i32,
    knockback: Vector2,
    base: Base<Node2D>,
}

#[godot_api]
impl INode2D for HurtboxEnemy {
    fn init(base: Base<Node2D>) -> Self {
        Self { health: 50, knockback: Vector2::ZERO, base }
    }

    fn ready(&mut self) {
        // Body visual
        let mut body = ColorRect::new_alloc();
        body.set_size(Vector2::new(32.0, 32.0));
        body.set_position(Vector2::new(-16.0, -16.0));
        body.set_color(Color::from_rgb(0.85, 0.22, 0.22));
        self.base_mut().add_child(&body);

        // Hurtbox Area2D
        let mut area = Area2D::new_alloc();
        area.set_name("Hurtbox");
        let mut shape = CircleShape2D::new_gd();
        shape.set_radius(22.0);
        let mut col = CollisionShape2D::new_alloc();
        col.set_shape(&shape);
        area.add_child(&col);
        self.base_mut().add_child(&area);

        // HP label
        let mut label = Label::new_alloc();
        label.set_name("HpLabel");
        let hp_text = format!("HP: {}", self.health);
        label.set_text(hp_text.as_str());
        label.set_position(Vector2::new(-20.0, -40.0));
        self.base_mut().add_child(&label);
    }

    fn process(&mut self, delta: f64) {
        // Drain knockback with friction
        if self.knockback.length() > 1.0 {
            // Copy the field first — `base_mut()` holds a mutable borrow of self.
            let knockback = self.knockback;
            let pos = self.base().get_position();
            self.base_mut().set_position(pos + knockback * delta as f32);
            self.knockback *= 0.80;
        }
    }
}

#[godot_api]
impl HurtboxEnemy {
    /// Called by the hitbox overlap signal or directly by arena logic.
    #[func]
    pub fn take_damage(&mut self, amount: i32, from_direction: Vector2) {
        let is_crit = amount > 12; // crits happen above threshold for demo purposes
        let dmg = compute_damage(amount, is_crit);
        self.health = clamp_health(self.health - dmg, 50);
        self.knockback = apply_knockback(Vector2::ZERO, from_direction, 220.0);

        // Update HP label
        let hp_text = format!("HP: {}", self.health);
        if let Some(mut label) = self.base().try_get_node_as::<Label>("HpLabel") {
            label.set_text(hp_text.as_str());
        }

        if is_dead(self.health) {
            self.base_mut().queue_free();
        }
    }

    // gdext auto-generates `get_health()` for the `#[export]` field, so this
    // accessor must use a different name.
    pub fn current_health(&self) -> i32 { self.health }
}

// ─── HurtboxArea — the Area2D that detects hitbox entry ───────────────────────

/// An Area2D subclass that calls `take_damage` on its parent when a hitbox enters.
#[derive(GodotClass)]
#[class(base=Area2D)]
pub struct HurtboxArea {
    base: Base<Area2D>,
}

#[godot_api]
impl IArea2D for HurtboxArea {
    fn init(base: Base<Area2D>) -> Self {
        Self { base }
    }

    fn ready(&mut self) {
        let callable = self.base().callable("_on_area_entered");
        self.base_mut().connect("area_entered", &callable);
    }
}

#[godot_api]
impl HurtboxArea {
    #[func]
    fn _on_area_entered(&mut self, _area: Gd<Area2D>) {
        if let Some(parent) = self.base().get_parent() {
            if let Ok(mut enemy) = parent.try_cast::<HurtboxEnemy>() {
                enemy.bind_mut().take_damage(10, Vector2::LEFT);
            }
        }
    }
}

// ─── Tests ────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn compute_damage_normal_hit() {
        assert_eq!(compute_damage(10, false), 10);
    }

    #[test]
    fn compute_damage_critical_hit_is_one_and_half() {
        assert_eq!(compute_damage(10, true), 15);
    }

    #[test]
    fn compute_damage_critical_rounds() {
        // 7 × 1.5 = 10.5 → rounds to 11
        assert_eq!(compute_damage(7, true), 11);
    }

    #[test]
    fn apply_knockback_adds_impulse() {
        let result = apply_knockback(Vector2::ZERO, Vector2::new(1.0, 0.0), 200.0);
        assert!((result.x - 200.0).abs() < 1e-4);
        assert!(result.y.abs() < 1e-4);
    }

    #[test]
    fn apply_knockback_accumulates_with_existing_velocity() {
        let vel = Vector2::new(50.0, 0.0);
        let result = apply_knockback(vel, Vector2::new(1.0, 0.0), 100.0);
        assert!((result.x - 150.0).abs() < 1e-4);
    }

    #[test]
    fn is_dead_at_zero() {
        assert!(is_dead(0));
    }

    #[test]
    fn is_dead_negative_health() {
        assert!(is_dead(-5));
    }

    #[test]
    fn is_not_dead_positive_health() {
        assert!(!is_dead(1));
    }

    #[test]
    fn clamp_health_within_range() {
        assert_eq!(clamp_health(30, 100), 30);
    }

    #[test]
    fn clamp_health_below_zero() {
        assert_eq!(clamp_health(-10, 100), 0);
    }

    #[test]
    fn clamp_health_above_max() {
        assert_eq!(clamp_health(150, 100), 100);
    }
}
