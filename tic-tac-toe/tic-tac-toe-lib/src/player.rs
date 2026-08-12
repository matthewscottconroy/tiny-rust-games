//! Who is playing, and what they have played.
//!
//! [`Player`] is a participant; [`Placement`] is a coordinate; [`Turn`] pairs
//! the two and is what [`TicTacToeGame::turn_history`] records.
//!
//! [`TicTacToeGame::turn_history`]: crate::TicTacToeGame::turn_history

/// One participant: a display name and the symbol they place.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Player {
    name: String,
    symbol: char,
}

impl Player {
    /// Creates a player. `symbol` should not be [`EMPTY_SYMBOL`], and should be
    /// distinct from every other player's symbol.
    pub fn new(name: String, symbol: char) -> Self {
        Self { name, symbol }
    }

    /// The player's display name.
    pub fn name(&self) -> &str {
        &self.name
    }

    /// The character this player places on the board.
    pub fn symbol(&self) -> char {
        self.symbol
    }
}

/// A zero-indexed board coordinate.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Placement {
    row: usize,
    column: usize,
}

impl Placement {
    /// Creates a coordinate.
    ///
    /// Crate-visible: a `Placement` is only ever produced by recording a move,
    /// so nothing outside this crate should be able to invent one.
    pub(crate) fn new(row: usize, column: usize) -> Self {
        Self { row, column }
    }

    /// The row index.
    pub fn row(&self) -> usize {
        self.row
    }

    /// The column index.
    pub fn column(&self) -> usize {
        self.column
    }
}

/// A completed move, recorded in [`TicTacToeGame::turn_history`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Turn {
    player: Player,
    placement: Placement,
}

impl Turn {
    /// Records a completed move.
    ///
    /// Crate-visible for the same reason as [`Placement::new`]: turn history is
    /// written by [`TicTacToeGame::take_turn`], never by a frontend.
    ///
    /// [`TicTacToeGame::take_turn`]: crate::TicTacToeGame::take_turn
    pub(crate) fn new(player: Player, placement: Placement) -> Self {
        Self { player, placement }
    }

    /// The player who made this move.
    pub fn player(&self) -> &Player {
        &self.player
    }

    /// Where the move was made.
    pub fn placement(&self) -> Placement {
        self.placement
    }
}
