//! Why a move can be rejected.

use std::fmt;

/// Why a call to [`TicTacToeGame::take_turn`] was rejected.
///
/// Move validation lives in the library so every frontend enforces the same
/// rules; a frontend should surface these rather than pre-checking itself.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MoveError {
    /// The requested cell lies outside the board.
    OutOfBounds {
        /// Requested row.
        row: usize,
        /// Requested column.
        column: usize,
    },
    /// The requested cell has already been played.
    CellOccupied {
        /// Requested row.
        row: usize,
        /// Requested column.
        column: usize,
    },
    /// The game has already been won or drawn; no further moves are legal.
    GameOver,
}

impl fmt::Display for MoveError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::OutOfBounds { row, column } => {
                write!(f, "({row}, {column}) is outside the board")
            }
            Self::CellOccupied { row, column } => {
                write!(f, "({row}, {column}) is already taken")
            }
            Self::GameOver => write!(f, "the game is already over"),
        }
    }
}

impl std::error::Error for MoveError {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn out_of_bounds_names_the_coordinate() {
        let e = MoveError::OutOfBounds { row: 3, column: 9 };
        assert_eq!(e.to_string(), "(3, 9) is outside the board");
    }

    #[test]
    fn cell_occupied_names_the_coordinate() {
        let e = MoveError::CellOccupied { row: 1, column: 2 };
        assert_eq!(e.to_string(), "(1, 2) is already taken");
    }

    #[test]
    fn game_over_explains_itself() {
        assert_eq!(MoveError::GameOver.to_string(), "the game is already over");
    }

    #[test]
    fn is_a_std_error() {
        // Frontends propagate this with `?`, so the trait impl must hold.
        fn assert_error<E: std::error::Error>(_: &E) {}
        assert_error(&MoveError::GameOver);
    }
}
