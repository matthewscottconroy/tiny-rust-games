//! grid-movement: Demonstrates tile-based movement with a queue of pending
//! directions and smooth lerp toward the target cell position.
//!
//! Teaches: queueing direction input in a `VecDeque` and lerping between grid cells so
//! movement snaps without feeling instant.

use std::collections::VecDeque;

use godot::classes::{CharacterBody2D, ICharacterBody2D, Input};
use godot::prelude::*;

struct GridMovementExtension;
#[gdextension]
unsafe impl ExtensionLibrary for GridMovementExtension {}

#[derive(GodotClass)]
#[class(base=CharacterBody2D)]
struct GridMovement {
    /// Width / height of one grid cell in pixels.
    #[export]
    cell_size: f32,
    /// Fraction of the distance to close per physics frame (0–1 range).
    #[export]
    move_speed: f32,

    // Current world position as plain floats (no Godot types in fields).
    pos_x: f32,
    pos_y: f32,
    // Target world position.
    target_x: f32,
    target_y: f32,
    // Queued directions (col_delta, row_delta), max 3.
    direction_queue: VecDeque<(i32, i32)>,
    // Current grid cell.
    grid_col: i32,
    grid_row: i32,

    base: Base<CharacterBody2D>,
}

#[godot_api]
impl ICharacterBody2D for GridMovement {
    fn init(base: Base<CharacterBody2D>) -> Self {
        Self {
            cell_size: 64.0,
            move_speed: 8.0,
            pos_x: 0.0,
            pos_y: 0.0,
            target_x: 0.0,
            target_y: 0.0,
            direction_queue: VecDeque::new(),
            grid_col: 0,
            grid_row: 0,
            base,
        }
    }

    fn ready(&mut self) {
        let pos = self.base().get_position();
        self.pos_x = pos.x;
        self.pos_y = pos.y;
        self.target_x = pos.x;
        self.target_y = pos.y;
        let (col, row) = world_to_grid(pos.x, pos.y, self.cell_size);
        self.grid_col = col;
        self.grid_row = row;
        godot_print!("GridMovement ready at cell ({}, {})", col, row);
    }

    fn physics_process(&mut self, delta: f64) {
        let input = Input::singleton();
        let cell = self.cell_size;

        // Collect direction inputs (one per frame, up to queue cap of 3)
        let dirs: [(bool, (i32, i32)); 4] = [
            (input.is_action_just_pressed("ui_right"), (1, 0)),
            (input.is_action_just_pressed("ui_left"), (-1, 0)),
            (input.is_action_just_pressed("ui_down"), (0, 1)),
            (input.is_action_just_pressed("ui_up"), (0, -1)),
        ];
        for (pressed, dir) in dirs {
            if pressed && self.direction_queue.len() < 3 {
                self.direction_queue.push_back(dir);
            }
        }

        // If we've arrived at the target, pop next direction
        let at_target =
            (self.pos_x - self.target_x).abs() < 0.5 && (self.pos_y - self.target_y).abs() < 0.5;

        if at_target && let Some((dc, dr)) = self.direction_queue.pop_front() {
            self.grid_col += dc;
            self.grid_row += dr;
            let (tx, ty) = grid_to_world(self.grid_col, self.grid_row, cell);
            self.target_x = tx;
            self.target_y = ty;
        }

        // Lerp toward target (use delta to scale speed)
        let t = (self.move_speed * delta as f32).min(1.0);
        self.pos_x = lerp_pos(self.pos_x, self.target_x, t);
        self.pos_y = lerp_pos(self.pos_y, self.target_y, t);

        // Copy to locals before calling base_mut() to avoid borrow conflict.
        let (px, py) = (self.pos_x, self.pos_y);
        self.base_mut().set_position(Vector2::new(px, py));
    }
}

// ---------------------------------------------------------------------------
// Pure helpers
// ---------------------------------------------------------------------------

/// Convert grid cell coordinates to world position (top-left of cell + half-cell offset).
pub fn grid_to_world(col: i32, row: i32, cell: f32) -> (f32, f32) {
    (
        col as f32 * cell + cell * 0.5,
        row as f32 * cell + cell * 0.5,
    )
}

/// Convert world position to the nearest grid cell (floor division).
pub fn world_to_grid(x: f32, y: f32, cell: f32) -> (i32, i32) {
    ((x / cell).floor() as i32, (y / cell).floor() as i32)
}

/// Linear interpolation between `from` and `to` by factor `t` clamped to [0, 1].
pub fn lerp_pos(from: f32, to: f32, t: f32) -> f32 {
    let t = t.clamp(0.0, 1.0);
    from + (to - from) * t
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn grid_to_world_origin() {
        let (x, y) = grid_to_world(0, 0, 64.0);
        assert_eq!(x, 32.0);
        assert_eq!(y, 32.0);
    }

    #[test]
    fn grid_to_world_offset() {
        let (x, y) = grid_to_world(2, 3, 64.0);
        assert!((x - 160.0).abs() < 0.001);
        assert!((y - 224.0).abs() < 0.001);
    }

    #[test]
    fn world_to_grid_origin() {
        let (col, row) = world_to_grid(32.0, 32.0, 64.0);
        assert_eq!(col, 0);
        assert_eq!(row, 0);
    }

    #[test]
    fn world_to_grid_second_cell() {
        let (col, row) = world_to_grid(65.0, 128.0, 64.0);
        assert_eq!(col, 1);
        assert_eq!(row, 2);
    }

    #[test]
    fn lerp_pos_full_t() {
        assert_eq!(lerp_pos(0.0, 100.0, 1.0), 100.0);
    }

    #[test]
    fn lerp_pos_zero_t() {
        assert_eq!(lerp_pos(0.0, 100.0, 0.0), 0.0);
    }

    #[test]
    fn lerp_pos_half_t() {
        assert!((lerp_pos(0.0, 100.0, 0.5) - 50.0).abs() < 0.001);
    }

    #[test]
    fn lerp_pos_clamps_above_one() {
        assert_eq!(lerp_pos(0.0, 100.0, 2.0), 100.0);
    }

    #[test]
    fn world_to_grid_negative() {
        let (col, row) = world_to_grid(-1.0, -1.0, 64.0);
        assert_eq!(col, -1);
        assert_eq!(row, -1);
    }
}
