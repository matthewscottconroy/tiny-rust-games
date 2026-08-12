//! Minimap demo — Godot 4.3 + gdext 0.5.
//!
//! Teaches:
//! - Using `_draw()` via `INode2D::draw` to render a minimap purely from Rust.
//! - Converting world coordinates to minimap pixel coordinates with a pure function.
//! - Calling `queue_redraw()` each frame so the minimap stays in sync.
//! - Spawning enemies at fixed world positions and tracking them in a Vec.
//!
//! The world is 1600×1200 units.  The minimap is a 200×150 px inset drawn in
//! the top-right corner of the screen.
//!
//! **Controls:** WASD — move player;  minimap updates automatically.

use godot::classes::{INode2D, Input, Label, Node2D};
use godot::prelude::*;

// ─── Extension entry point ────────────────────────────────────────────────────

struct MinimapExt;
#[gdextension]
unsafe impl ExtensionLibrary for MinimapExt {}

// ─── Pure helpers ─────────────────────────────────────────────────────────────

/// Maps `world_pos` (in a world of size `world_size`) to pixel coords within a
/// minimap of size `minimap_size`.  Origin is the top-left of the minimap.
pub fn world_to_minimap(world_pos: Vector2, world_size: Vector2, minimap_size: Vector2) -> Vector2 {
    Vector2::new(
        (world_pos.x / world_size.x * minimap_size.x).clamp(0.0, minimap_size.x),
        (world_pos.y / world_size.y * minimap_size.y).clamp(0.0, minimap_size.y),
    )
}

/// Clamps `pos` so the player stays within `[0, world_size]`.
pub fn clamp_to_world(pos: Vector2, world_size: Vector2) -> Vector2 {
    Vector2::new(
        pos.x.clamp(0.0, world_size.x),
        pos.y.clamp(0.0, world_size.y),
    )
}

// ─── World constants ──────────────────────────────────────────────────────────

const WORLD_W: f32 = 1600.0;
const WORLD_H: f32 = 1200.0;
const WORLD_SIZE: Vector2 = Vector2::new(WORLD_W, WORLD_H);

const MAP_W: f32 = 200.0;
const MAP_H: f32 = 150.0;
const MAP_SIZE: Vector2 = Vector2::new(MAP_W, MAP_H);

// Top-right corner offset (screen-space origin of the minimap rectangle).
const MAP_ORIGIN: Vector2 = Vector2::new(590.0, 10.0);

// ─── MinimapArena ─────────────────────────────────────────────────────────────

#[derive(GodotClass)]
#[class(base=Node2D)]
pub struct MinimapArena {
    player_world_pos: Vector2,
    enemy_world_positions: Vec<Vector2>,
    speed: f32,
    base: Base<Node2D>,
}

#[godot_api]
impl INode2D for MinimapArena {
    fn init(base: Base<Node2D>) -> Self {
        Self {
            player_world_pos: Vector2::new(200.0, 200.0),
            enemy_world_positions: vec![
                Vector2::new(800.0, 400.0),
                Vector2::new(300.0, 900.0),
                Vector2::new(1200.0, 600.0),
                Vector2::new(600.0, 1000.0),
            ],
            speed: 180.0,
            base,
        }
    }

    fn ready(&mut self) {
        let mut hint = Label::new_alloc();
        hint.set_text("WASD — move   minimap updates live");
        hint.set_position(Vector2::new(10.0, 8.0));
        self.base_mut().add_child(&hint);

        // Player visual
        let mut player_rect = godot::classes::ColorRect::new_alloc();
        player_rect.set_name("PlayerRect");
        player_rect.set_size(Vector2::new(20.0, 20.0));
        player_rect.set_color(Color::from_rgb(0.3, 0.7, 1.0));
        self.base_mut().add_child(&player_rect);

        // Enemy visuals
        let enemy_positions: Vec<Vector2> = self.enemy_world_positions.clone();
        for (i, pos) in enemy_positions.into_iter().enumerate() {
            let mut rect = godot::classes::ColorRect::new_alloc();
            let ename = format!("Enemy{}", i);
            rect.set_name(ename.as_str());
            rect.set_size(Vector2::new(18.0, 18.0));
            rect.set_color(Color::from_rgb(0.9, 0.2, 0.2));
            // Scale world → screen (viewport is roughly 800×600, world 1600×1200)
            rect.set_position(Vector2::new(pos.x * 0.5 - 9.0, pos.y * 0.5 - 9.0));
            self.base_mut().add_child(&rect);
        }
    }

    fn process(&mut self, delta: f64) {
        let dt = delta as f32;
        let input = Input::singleton();

        let mut dir = Vector2::ZERO;
        if input.is_action_pressed("ui_right") {
            dir.x += 1.0;
        }
        if input.is_action_pressed("ui_left") {
            dir.x -= 1.0;
        }
        if input.is_action_pressed("ui_down") {
            dir.y += 1.0;
        }
        if input.is_action_pressed("ui_up") {
            dir.y -= 1.0;
        }

        if dir != Vector2::ZERO {
            self.player_world_pos += dir.normalized() * self.speed * dt;
            self.player_world_pos = clamp_to_world(self.player_world_pos, WORLD_SIZE);
        }

        // Sync player visual to screen coords
        if let Some(mut r) = self
            .base()
            .try_get_node_as::<godot::classes::ColorRect>("PlayerRect")
        {
            r.set_position(Vector2::new(
                self.player_world_pos.x * 0.5 - 10.0,
                self.player_world_pos.y * 0.5 - 10.0,
            ));
        }

        self.base_mut().queue_redraw();
    }

    fn draw(&mut self) {
        // Minimap background
        self.base_mut().draw_rect(
            Rect2::new(MAP_ORIGIN, MAP_SIZE),
            Color::from_rgba(0.0, 0.0, 0.0, 0.7),
        );

        // Border
        self.base_mut().draw_rect(
            Rect2::new(MAP_ORIGIN, MAP_SIZE),
            Color::from_rgb(0.8, 0.8, 0.8),
        );

        // Enemies (red dots)
        for &ep in &self.enemy_world_positions.clone() {
            let mp = world_to_minimap(ep, WORLD_SIZE, MAP_SIZE);
            self.base_mut()
                .draw_circle(MAP_ORIGIN + mp, 3.0, Color::from_rgb(0.9, 0.2, 0.2));
        }

        // Player (blue dot)
        let pp = world_to_minimap(self.player_world_pos, WORLD_SIZE, MAP_SIZE);
        self.base_mut()
            .draw_circle(MAP_ORIGIN + pp, 4.0, Color::from_rgb(0.3, 0.7, 1.0));
    }
}

// ─── Tests ────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    const WS: Vector2 = Vector2::new(1600.0, 1200.0);
    const MS: Vector2 = Vector2::new(200.0, 150.0);

    #[test]
    fn world_to_minimap_origin_maps_to_origin() {
        let p = world_to_minimap(Vector2::ZERO, WS, MS);
        assert!(p.x.abs() < 1e-4 && p.y.abs() < 1e-4);
    }

    #[test]
    fn world_to_minimap_far_corner_maps_to_minimap_corner() {
        let p = world_to_minimap(WS, WS, MS);
        assert!((p.x - MS.x).abs() < 1e-3 && (p.y - MS.y).abs() < 1e-3);
    }

    #[test]
    fn world_to_minimap_center_maps_to_minimap_center() {
        let center = Vector2::new(WS.x / 2.0, WS.y / 2.0);
        let p = world_to_minimap(center, WS, MS);
        assert!((p.x - MS.x / 2.0).abs() < 1e-3);
        assert!((p.y - MS.y / 2.0).abs() < 1e-3);
    }

    #[test]
    fn world_to_minimap_clamped_when_out_of_bounds() {
        let p = world_to_minimap(Vector2::new(9999.0, 9999.0), WS, MS);
        assert!((p.x - MS.x).abs() < 1e-3);
        assert!((p.y - MS.y).abs() < 1e-3);
    }

    #[test]
    fn world_to_minimap_proportional() {
        let p = world_to_minimap(Vector2::new(400.0, 300.0), WS, MS);
        // 400/1600 = 0.25 → 0.25 * 200 = 50
        assert!((p.x - 50.0).abs() < 1e-3);
        // 300/1200 = 0.25 → 0.25 * 150 = 37.5
        assert!((p.y - 37.5).abs() < 1e-3);
    }

    #[test]
    fn clamp_to_world_inside_unchanged() {
        let pos = Vector2::new(400.0, 300.0);
        let clamped = clamp_to_world(pos, WS);
        assert!((clamped.x - 400.0).abs() < 1e-4);
        assert!((clamped.y - 300.0).abs() < 1e-4);
    }

    #[test]
    fn clamp_to_world_negative_x_becomes_zero() {
        let pos = Vector2::new(-50.0, 300.0);
        let clamped = clamp_to_world(pos, WS);
        assert_eq!(clamped.x, 0.0);
    }

    #[test]
    fn clamp_to_world_over_max_clamped() {
        let pos = Vector2::new(2000.0, 1500.0);
        let clamped = clamp_to_world(pos, WS);
        assert_eq!(clamped.x, WS.x);
        assert_eq!(clamped.y, WS.y);
    }

    #[test]
    fn minimap_dot_does_not_escape_map_for_extreme_input() {
        let p = world_to_minimap(Vector2::new(-100.0, -100.0), WS, MS);
        assert!(p.x >= 0.0 && p.y >= 0.0);
    }
}
