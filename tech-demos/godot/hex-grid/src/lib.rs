//! Hexagonal grid demo — Godot 4.3 + gdext 0.5.
//!
//! Teaches:
//! - Axial `(q, r)` hex coordinates and their conversion to pixel space.
//! - Drawing flat-top hexagons using `draw_polygon` in `_draw()`.
//! - Highlighting a selected cell and its 6 neighbours by colour.
//! - Detecting which hex was clicked with `pixel_to_axial` + rounding.
//!
//! **Controls:** left-click — select hex;  R — reset.

use godot::classes::{INode2D, Input, InputEvent, InputEventMouseButton, Label, Node2D};
use godot::global::MouseButton;
use godot::prelude::*;
use std::f32::consts::TAU;

// ── Extension entry point ─────────────────────────────────────────────────────

struct HexGridExt;
#[gdextension]
unsafe impl ExtensionLibrary for HexGridExt {}

// ── Pure hex math ─────────────────────────────────────────────────────────────

/// Distance from a hexagon's centre to a vertex, in pixels.
pub const HEX_SIZE: f32 = 32.0;

/// Flat-top axial `(q, r)` → pixel centre.
pub fn axial_to_pixel(q: i32, r: i32, size: f32) -> Vector2 {
    Vector2::new(
        size * 1.5 * q as f32,
        size * (0.8660254 * q as f32 + 1.7320508 * r as f32),
    )
}

/// Pixel position → nearest flat-top axial `(q, r)`.
pub fn pixel_to_axial(p: Vector2, size: f32) -> (i32, i32) {
    let q = (2.0 / 3.0) * p.x / size;
    let r = (-1.0 / 3.0) * p.x / size + (3.0_f32.sqrt() / 3.0) * p.y / size;
    cube_round(q, -q - r, r)
}

/// Rounds fractional cube coordinates to the nearest hex.
///
/// Cube coordinates satisfy `q + s + r == 0`, and rounding each axis
/// independently can break that. The fix is to keep the two axes that rounded
/// most accurately and re-derive the third from them. Only `(q, r)` is
/// returned, since `s` is always `-q - r`.
pub fn cube_round(q: f32, s: f32, r: f32) -> (i32, i32) {
    let (rq, rs, rr) = (q.round(), s.round(), r.round());
    let (dq, ds, dr) = ((rq - q).abs(), (rs - s).abs(), (rr - r).abs());

    if dq > ds && dq > dr {
        // `q` drifted most — rebuild it from the other two.
        ((-rs - rr) as i32, rr as i32)
    } else if ds > dr {
        // `s` drifted most, and it is the axis we discard anyway.
        (rq as i32, rr as i32)
    } else {
        // `r` drifted most — rebuild it from the other two.
        (rq as i32, (-rq - rs) as i32)
    }
}

/// The 6 axial neighbours of `(q, r)`.
pub fn hex_neighbors(q: i32, r: i32) -> [(i32, i32); 6] {
    [
        (q + 1, r),
        (q + 1, r - 1),
        (q, r - 1),
        (q - 1, r),
        (q - 1, r + 1),
        (q, r + 1),
    ]
}

/// Axial hex distance.
pub fn axial_distance(q1: i32, r1: i32, q2: i32, r2: i32) -> i32 {
    let dq = (q1 - q2).abs();
    let dr = (r1 - r2).abs();
    let ds = ((q1 + r1) - (q2 + r2)).abs();
    (dq + dr + ds) / 2
}

/// Returns a flat-top hexagon polygon (6 vertices) centred at `centre`.
pub fn hex_polygon(centre: Vector2, size: f32) -> Vec<Vector2> {
    (0..6)
        .map(|i| {
            let angle = TAU / 6.0 * i as f32; // flat-top: starts at 0°
            centre + Vector2::new(angle.cos() * size, angle.sin() * size)
        })
        .collect()
}

// ── HexGridDemo — root Node2D ─────────────────────────────────────────────────

/// Godot node drawing the hex grid and tracking the selected cell.
#[derive(GodotClass)]
#[class(base=Node2D)]
pub struct HexGridDemo {
    selected: Option<(i32, i32)>,
    cells: Vec<(i32, i32)>,
    base: Base<Node2D>,
}

#[godot_api]
impl INode2D for HexGridDemo {
    fn init(base: Base<Node2D>) -> Self {
        let mut cells = Vec::new();
        for q in -5..=5i32 {
            for r in -5..=5i32 {
                if axial_distance(q, r, 0, 0) <= 5 {
                    cells.push((q, r));
                }
            }
        }
        Self {
            selected: None,
            cells,
            base,
        }
    }

    fn ready(&mut self) {
        let mut hint = Label::new_alloc();
        hint.set_text("left-click: select   R: reset");
        hint.set_position(Vector2::new(-390.0, -290.0));
        self.base_mut().add_child(&hint);
    }

    fn input(&mut self, event: Gd<InputEvent>) {
        if let Ok(mb) = event.try_cast::<InputEventMouseButton>()
            && mb.get_button_index() == MouseButton::LEFT
            && mb.is_pressed()
        {
            let local = mb.get_position() - Vector2::new(400.0, 300.0);
            let (q, r) = pixel_to_axial(local, HEX_SIZE);
            if self.cells.contains(&(q, r)) {
                self.selected = Some((q, r));
            } else {
                self.selected = None;
            }
            self.base_mut().queue_redraw();
        }

        let input = Input::singleton();
        if input.is_action_just_pressed("ui_cancel") {
            self.selected = None;
            self.base_mut().queue_redraw();
        }
    }

    fn draw(&mut self) {
        let neighbors: std::collections::HashSet<(i32, i32)> = self
            .selected
            .map(|(q, r)| hex_neighbors(q, r).into_iter().collect())
            .unwrap_or_default();

        let cells = self.cells.clone();
        for (q, r) in cells {
            let centre = axial_to_pixel(q, r, HEX_SIZE);
            let verts: Vec<Vector2> = hex_polygon(centre, HEX_SIZE - 2.0);
            let packed: PackedVector2Array = verts.into_iter().collect();
            let color = if self.selected == Some((q, r)) {
                Color::from_rgb(1.0, 0.75, 0.15)
            } else if neighbors.contains(&(q, r)) {
                Color::from_rgb(0.35, 0.55, 0.85)
            } else {
                Color::from_rgb(0.22, 0.22, 0.3)
            };
            self.base_mut().draw_colored_polygon(&packed, color);
            // thin outline
            self.base_mut()
                .draw_polyline(&packed, Color::from_rgb(0.4, 0.4, 0.5));
        }
    }
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn axial_to_pixel_origin_is_zero() {
        let p = axial_to_pixel(0, 0, 32.0);
        assert!(p.x.abs() < 1e-4 && p.y.abs() < 1e-4);
    }

    #[test]
    fn pixel_to_axial_round_trips() {
        for (q, r) in [(2, -1), (-3, 1), (0, 4), (5, -5)] {
            let p = axial_to_pixel(q, r, 32.0);
            assert_eq!(pixel_to_axial(p, 32.0), (q, r), "failed ({q},{r})");
        }
    }

    #[test]
    fn hex_neighbors_count_is_six() {
        assert_eq!(hex_neighbors(0, 0).len(), 6);
    }

    #[test]
    fn hex_neighbors_all_distance_one() {
        for (nq, nr) in hex_neighbors(1, -2) {
            assert_eq!(axial_distance(1, -2, nq, nr), 1);
        }
    }

    #[test]
    fn axial_distance_same_is_zero() {
        assert_eq!(axial_distance(3, -1, 3, -1), 0);
    }

    #[test]
    fn axial_distance_known() {
        assert_eq!(axial_distance(0, 0, 0, 3), 3);
    }

    #[test]
    fn hex_polygon_has_six_vertices() {
        assert_eq!(hex_polygon(Vector2::ZERO, 32.0).len(), 6);
    }

    #[test]
    fn hex_polygon_vertices_are_at_radius() {
        for v in hex_polygon(Vector2::ZERO, 32.0) {
            assert!(
                (v.length() - 32.0).abs() < 1e-3,
                "radius mismatch: {}",
                v.length()
            );
        }
    }
}
