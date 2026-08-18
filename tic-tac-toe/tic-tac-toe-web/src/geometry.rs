//! Board geometry: the only arithmetic this frontend contains.
//!
//! Kept in its own module, free of `web-sys`, so it can be unit-tested on the
//! host. Everything else here is drawing calls and an event listener, which is
//! exactly the split the repository argues for — the part worth testing is the
//! part that is not the engine's.

/// A square on the canvas, in pixels.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct CellRect {
    /// Left edge.
    pub x: f64,
    /// Top edge.
    pub y: f64,
    /// Side length.
    pub size: f64,
}

/// Maps the board onto a square canvas.
#[derive(Debug, Clone, Copy)]
pub struct Grid {
    /// Canvas side length, in pixels.
    pub canvas: f64,
    /// Board columns.
    pub columns: usize,
    /// Board rows.
    pub rows: usize,
}

impl Grid {
    /// A grid for a board of this shape drawn on a square canvas.
    pub fn new(canvas: f64, columns: usize, rows: usize) -> Self {
        Self {
            canvas,
            columns,
            rows,
        }
    }

    /// Width of one cell, in pixels.
    pub fn cell_width(&self) -> f64 {
        self.canvas / self.columns as f64
    }

    /// Height of one cell, in pixels.
    pub fn cell_height(&self) -> f64 {
        self.canvas / self.rows as f64
    }

    /// The rectangle for a cell.
    pub fn cell_rect(&self, row: usize, column: usize) -> CellRect {
        CellRect {
            x: column as f64 * self.cell_width(),
            y: row as f64 * self.cell_height(),
            size: self.cell_width().min(self.cell_height()),
        }
    }

    /// The centre of a cell, in pixels.
    pub fn cell_centre(&self, row: usize, column: usize) -> (f64, f64) {
        (
            (column as f64 + 0.5) * self.cell_width(),
            (row as f64 + 0.5) * self.cell_height(),
        )
    }

    /// The cell a click landed in, if it landed on the board at all.
    ///
    /// Returns `(row, column)`. A click exactly on the right or bottom edge
    /// would index one past the last cell, which is why this is a checked
    /// conversion rather than a cast — the browser really does report those
    /// coordinates, and a cast would panic or silently wrap.
    pub fn cell_at(&self, x: f64, y: f64) -> Option<(usize, usize)> {
        if x < 0.0 || y < 0.0 || x >= self.canvas || y >= self.canvas {
            return None;
        }
        let column = (x / self.cell_width()) as usize;
        let row = (y / self.cell_height()) as usize;
        if row < self.rows && column < self.columns {
            Some((row, column))
        } else {
            None
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn grid() -> Grid {
        Grid::new(300.0, 3, 3)
    }

    #[test]
    fn a_click_in_each_corner_lands_in_the_right_cell() {
        let g = grid();
        assert_eq!(g.cell_at(1.0, 1.0), Some((0, 0)));
        assert_eq!(g.cell_at(299.0, 1.0), Some((0, 2)));
        assert_eq!(g.cell_at(1.0, 299.0), Some((2, 0)));
        assert_eq!(g.cell_at(299.0, 299.0), Some((2, 2)));
    }

    #[test]
    fn the_centre_cell_is_the_middle_one() {
        assert_eq!(grid().cell_at(150.0, 150.0), Some((1, 1)));
    }

    #[test]
    fn clicks_outside_the_board_hit_nothing() {
        let g = grid();
        assert_eq!(g.cell_at(-1.0, 10.0), None);
        assert_eq!(g.cell_at(10.0, -1.0), None);
        // Exactly on the far edge is outside, not cell 3 — the case a plain
        // cast would get wrong.
        assert_eq!(g.cell_at(300.0, 10.0), None);
        assert_eq!(g.cell_at(10.0, 300.0), None);
    }

    #[test]
    fn every_cell_is_reachable_by_a_click_at_its_centre() {
        let g = grid();
        for row in 0..3 {
            for column in 0..3 {
                let (x, y) = g.cell_centre(row, column);
                assert_eq!(g.cell_at(x, y), Some((row, column)), "{row},{column}");
            }
        }
    }

    #[test]
    fn cells_tile_the_canvas_without_gaps() {
        let g = grid();
        let first = g.cell_rect(0, 0);
        let last = g.cell_rect(2, 2);
        assert_eq!(first.x, 0.0);
        assert_eq!(first.y, 0.0);
        assert!((last.x + last.size - g.canvas).abs() < 1e-9);
        assert!((last.y + last.size - g.canvas).abs() < 1e-9);
    }

    #[test]
    fn a_non_square_board_still_tiles() {
        // The library allows any board size, so the geometry must not assume 3x3.
        let g = Grid::new(400.0, 4, 5);
        assert_eq!(g.cell_at(399.0, 399.0), Some((4, 3)));
        assert_eq!(g.cell_at(0.0, 0.0), Some((0, 0)));
        for row in 0..5 {
            for column in 0..4 {
                let (x, y) = g.cell_centre(row, column);
                assert_eq!(g.cell_at(x, y), Some((row, column)));
            }
        }
    }
}
