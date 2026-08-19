//! The grid itself: cells, bounds, and how a board renders.
//!
//! [`Board`] knows nothing about turns or winning — it only stores symbols and
//! answers questions about its own shape. The rules live in [`crate::game`].

use alloc::{string::String, vec, vec::Vec};

use core::fmt;

use crate::EMPTY_SYMBOL;

/// A rectangular grid of cells, each holding a player symbol or [`EMPTY_SYMBOL`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Board {
    board: Vec<Vec<char>>,
}

impl Board {
    /// Creates an empty `row`×`column` board.
    ///
    /// # Panics
    /// Panics if either dimension is zero — a board with no cells has no
    /// meaningful width, and every accessor below would be a special case.
    pub fn new(row: usize, column: usize) -> Self {
        assert!(row > 0 && column > 0, "board must have at least one cell");
        Self {
            board: vec![vec![EMPTY_SYMBOL; column]; row],
        }
    }

    /// Writes `val` into a cell without any validation.
    ///
    /// Prefer [`TicTacToeGame::take_turn`](crate::TicTacToeGame::take_turn), which enforces the rules; this is
    /// the raw primitive it is built from.
    ///
    /// # Panics
    /// Panics if the coordinate is outside the board.
    pub fn place(&mut self, val: char, row: usize, column: usize) {
        self.board[row][column] = val;
    }

    /// Whether every cell has been played.
    pub fn is_full(&self) -> bool {
        self.board.iter().all(|row| !row.contains(&EMPTY_SYMBOL))
    }

    /// Whether the coordinate is inside the board.
    pub fn contains(&self, row: usize, column: usize) -> bool {
        row < self.height() && column < self.width()
    }

    /// Whether the cell has not been played yet.
    ///
    /// Returns `false` for coordinates outside the board, so this is safe to
    /// call on unvalidated input.
    pub fn is_entry_empty(&self, row: usize, column: usize) -> bool {
        self.get(row, column) == Some(EMPTY_SYMBOL)
    }

    /// The symbol at a coordinate, or `None` if it is outside the board.
    pub fn get(&self, row: usize, column: usize) -> Option<char> {
        self.board.get(row)?.get(column).copied()
    }

    /// The rows of the board, for frontends that want to draw it themselves.
    pub fn rows(&self) -> &[Vec<char>] {
        &self.board
    }

    /// Number of columns.
    pub fn width(&self) -> usize {
        self.board[0].len()
    }

    /// Number of rows.
    pub fn height(&self) -> usize {
        self.board.len()
    }

    /// Renders the board as rows of bare symbols, one line per row.
    ///
    /// This is the [`fmt::Display`] representation; `board.to_string()` and
    /// `format!("{board}")` both produce it.
    fn write_plain(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        for row in &self.board {
            for cell in row {
                write!(f, "{cell}")?;
            }
            writeln!(f)?;
        }
        Ok(())
    }

    /// Renders the board with `|` and `-+-` separators between cells.
    pub fn to_pretty_string(&self) -> String {
        let mut result = String::new();
        for i in 0..self.height() {
            for j in 0..self.width() {
                result.push(self.board[i][j]);
                if j != self.width() - 1 {
                    result.push('|');
                }
            }
            result.push('\n');
            if i != self.height() - 1 {
                for j in 0..self.width() {
                    result.push('-');
                    if j != self.width() - 1 {
                        result.push('+');
                    }
                }
                result.push('\n');
            }
        }
        result
    }
}

impl fmt::Display for Board {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.write_plain(f)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rows_exposes_every_row_in_order() {
        let mut board = Board::new(2, 3);
        board.place('X', 0, 0);
        board.place('O', 1, 2);

        let rows = board.rows();
        assert_eq!(rows.len(), 2);
        assert_eq!(rows[0], vec!['X', EMPTY_SYMBOL, EMPTY_SYMBOL]);
        assert_eq!(rows[1], vec![EMPTY_SYMBOL, EMPTY_SYMBOL, 'O']);
    }
}
