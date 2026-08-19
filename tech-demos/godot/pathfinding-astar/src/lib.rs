//! Pathfinding A* GDExtension demo — interactive grid path-finding from Rust.
//!
//! Teaches:
//!
//! - Creating an `AStar2D` instance via `AStar2D::new_gd()`.
//! - Adding grid points and connecting orthogonal neighbours.
//! - Left-click to set start, right-click to set goal, then querying the path.
//! - Implementing `_draw()` to colour-code cells (empty / start / goal / path)
//!   and draw a polyline along the found path.
//! - Pure helpers (`grid_to_id`, `id_to_grid`, `grid_to_world`, `manhattan`)
//!   fully covered by unit tests.
//!
//! Concept: pathfinding
//!
//! Counterpart: tech-demos/bevy/pathfinding — the same concept in Bevy.

use godot::classes::{AStar2D, INode2D, InputEvent, InputEventMouseButton};
use godot::prelude::*;

// ─── Extension entry point ───────────────────────────────────────────────────

struct PathfindingAStarExt;

#[gdextension]
unsafe impl ExtensionLibrary for PathfindingAStarExt {}

// ─── Pure functions ───────────────────────────────────────────────────────────

/// Converts a `(col, row)` grid coordinate to a flat integer ID.
///
/// # Examples
/// ```
/// assert_eq!(pathfinding_astar::grid_to_id(0, 0, 10), 0);
/// assert_eq!(pathfinding_astar::grid_to_id(3, 2, 10), 23);
/// assert_eq!(pathfinding_astar::grid_to_id(9, 7, 10), 79);
/// ```
pub fn grid_to_id(col: i32, row: i32, cols: i32) -> i32 {
    row * cols + col
}

/// Converts a flat integer ID back to `(col, row)`.
///
/// # Examples
/// ```
/// assert_eq!(pathfinding_astar::id_to_grid(0, 10), (0, 0));
/// assert_eq!(pathfinding_astar::id_to_grid(23, 10), (3, 2));
/// assert_eq!(pathfinding_astar::id_to_grid(79, 10), (9, 7));
/// ```
pub fn id_to_grid(id: i32, cols: i32) -> (i32, i32) {
    (id % cols, id / cols)
}

/// Converts a grid `(col, row)` to world-space centre coordinates.
///
/// # Examples
/// ```
/// assert_eq!(pathfinding_astar::grid_to_world(0, 0, 50.0), (25.0, 25.0));
/// assert_eq!(pathfinding_astar::grid_to_world(1, 0, 50.0), (75.0, 25.0));
/// assert_eq!(pathfinding_astar::grid_to_world(0, 1, 50.0), (25.0, 75.0));
/// ```
pub fn grid_to_world(col: i32, row: i32, cell: f32) -> (f32, f32) {
    (
        col as f32 * cell + cell * 0.5,
        row as f32 * cell + cell * 0.5,
    )
}

/// Returns the Manhattan distance between two grid cells.
///
/// # Examples
/// ```
/// assert_eq!(pathfinding_astar::manhattan(0, 0, 3, 4), 7);
/// assert_eq!(pathfinding_astar::manhattan(2, 2, 2, 2), 0);
/// assert_eq!(pathfinding_astar::manhattan(5, 1, 1, 4), 7);
/// ```
pub fn manhattan(ax: i32, ay: i32, bx: i32, by: i32) -> i32 {
    (ax - bx).abs() + (ay - by).abs()
}

// ─── AStarGrid node ───────────────────────────────────────────────────────────

/// Scene root that displays an interactive A* pathfinding grid.
///
/// Left-click sets the start cell; right-click sets the goal cell.  The path
/// is recomputed after each click and rendered in `_draw()`.
#[derive(GodotClass)]
#[class(base=Node2D)]
pub struct AStarGrid {
    /// Number of grid columns.
    #[export]
    grid_cols: i32,
    /// Number of grid rows.
    #[export]
    grid_rows: i32,
    /// Size in pixels of each cell.
    #[export]
    cell_size: f32,
    /// Start cell `(col, row)`.
    start: (i32, i32),
    /// Goal cell `(col, row)`.
    goal: (i32, i32),
    /// World-space positions making up the current path.
    path: Vec<(f32, f32)>,
    base: Base<Node2D>,
}

#[godot_api]
impl INode2D for AStarGrid {
    fn init(base: Base<Node2D>) -> Self {
        Self {
            grid_cols: 10,
            grid_rows: 8,
            cell_size: 50.0,
            start: (0, 0),
            goal: (9, 7),
            path: Vec::new(),
            base,
        }
    }

    fn ready(&mut self) {
        self.recompute_path();
        godot_print!(
            "[AStarGrid] Ready — {}×{} grid, cell {}px.",
            self.grid_cols,
            self.grid_rows,
            self.cell_size
        );
    }

    fn input(&mut self, event: Gd<InputEvent>) {
        if let Ok(mb) = event.try_cast::<InputEventMouseButton>() {
            if !mb.is_pressed() {
                return;
            }
            let pos = mb.get_position();
            let cell = self.cell_size;
            let col = (pos.x / cell) as i32;
            let row = (pos.y / cell) as i32;
            let cols = self.grid_cols;
            let rows = self.grid_rows;
            if col < 0 || col >= cols || row < 0 || row >= rows {
                return;
            }
            match mb.get_button_index() {
                godot::global::MouseButton::LEFT => {
                    self.start = (col, row);
                }
                godot::global::MouseButton::RIGHT => {
                    self.goal = (col, row);
                }
                _ => {}
            }
            self.recompute_path();
            self.base_mut().queue_redraw();
        }
    }

    fn draw(&mut self) {
        let cell = self.cell_size;
        let cols = self.grid_cols;
        let rows = self.grid_rows;
        let start = self.start;
        let goal = self.goal;
        let path = self.path.clone();

        // Draw all cells.
        for row in 0..rows {
            for col in 0..cols {
                let (wx, wy) = grid_to_world(col, row, cell);
                let is_start = (col, row) == start;
                let is_goal = (col, row) == goal;
                let in_path = path.contains(&(wx, wy));

                let color = if is_start {
                    Color::from_rgb(0.2, 0.8, 0.2) // green
                } else if is_goal {
                    Color::from_rgb(0.8, 0.2, 0.2) // red
                } else if in_path {
                    Color::from_rgb(0.4, 0.6, 1.0) // blue
                } else {
                    Color::from_rgb(0.3, 0.3, 0.3) // dark grey
                };

                let rect = Rect2::new(
                    Vector2::new(wx - cell * 0.5, wy - cell * 0.5),
                    Vector2::new(cell - 1.0, cell - 1.0),
                );
                self.base_mut().draw_rect(rect, color);
            }
        }

        // Draw the path polyline.
        if path.len() >= 2 {
            let pts: Vec<Vector2> = path.iter().map(|&(x, y)| Vector2::new(x, y)).collect();
            let packed: PackedVector2Array = pts.iter().cloned().collect();
            self.base_mut()
                .draw_polyline(&packed, Color::from_rgb(1.0, 1.0, 0.0));
        }
    }
}

#[godot_api]
impl AStarGrid {
    /// Rebuilds the `AStar2D` graph and recomputes the path from start to goal.
    fn recompute_path(&mut self) {
        let cols = self.grid_cols;
        let rows = self.grid_rows;
        let cell = self.cell_size;

        let mut astar = AStar2D::new_gd();

        // Add all grid points.
        for row in 0..rows {
            for col in 0..cols {
                let id = grid_to_id(col, row, cols);
                let (wx, wy) = grid_to_world(col, row, cell);
                astar.add_point(id as i64, Vector2::new(wx, wy));
            }
        }

        // Connect orthogonal neighbours.
        for row in 0..rows {
            for col in 0..cols {
                let id = grid_to_id(col, row, cols);
                for (dc, dr) in [(1i32, 0i32), (0, 1)] {
                    let nc = col + dc;
                    let nr = row + dr;
                    if nc < cols && nr < rows {
                        let nid = grid_to_id(nc, nr, cols);
                        astar.connect_points(id as i64, nid as i64);
                    }
                }
            }
        }

        // Query the path.
        let start_id = grid_to_id(self.start.0, self.start.1, cols) as i64;
        let goal_id = grid_to_id(self.goal.0, self.goal.1, cols) as i64;
        let raw = astar.get_point_path(start_id, goal_id);

        self.path = raw.to_vec().iter().map(|v| (v.x, v.y)).collect();
        godot_print!(
            "[AStarGrid] Path recomputed — {} waypoints.",
            self.path.len()
        );
    }
}

// ─── Tests ────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    // grid_to_id

    #[test]
    fn grid_to_id_origin() {
        assert_eq!(grid_to_id(0, 0, 10), 0);
    }

    #[test]
    fn grid_to_id_mid() {
        assert_eq!(grid_to_id(3, 2, 10), 23);
    }

    #[test]
    fn grid_to_id_last_cell() {
        assert_eq!(grid_to_id(9, 7, 10), 79);
    }

    // id_to_grid

    #[test]
    fn id_to_grid_zero() {
        assert_eq!(id_to_grid(0, 10), (0, 0));
    }

    #[test]
    fn id_to_grid_mid() {
        assert_eq!(id_to_grid(23, 10), (3, 2));
    }

    #[test]
    fn id_to_grid_last() {
        assert_eq!(id_to_grid(79, 10), (9, 7));
    }

    #[test]
    fn grid_id_round_trip() {
        for row in 0..8 {
            for col in 0..10 {
                let id = grid_to_id(col, row, 10);
                assert_eq!(id_to_grid(id, 10), (col, row));
            }
        }
    }

    // grid_to_world

    #[test]
    fn grid_to_world_origin() {
        assert_eq!(grid_to_world(0, 0, 50.0), (25.0, 25.0));
    }

    #[test]
    fn grid_to_world_col_one() {
        assert_eq!(grid_to_world(1, 0, 50.0), (75.0, 25.0));
    }

    #[test]
    fn grid_to_world_row_one() {
        assert_eq!(grid_to_world(0, 1, 50.0), (25.0, 75.0));
    }

    // manhattan

    #[test]
    fn manhattan_zero() {
        assert_eq!(manhattan(2, 2, 2, 2), 0);
    }

    #[test]
    fn manhattan_horizontal() {
        assert_eq!(manhattan(0, 0, 5, 0), 5);
    }

    #[test]
    fn manhattan_diagonal() {
        assert_eq!(manhattan(0, 0, 3, 4), 7);
    }

    #[test]
    fn manhattan_negative_direction() {
        assert_eq!(manhattan(5, 1, 1, 4), 7);
    }
}
