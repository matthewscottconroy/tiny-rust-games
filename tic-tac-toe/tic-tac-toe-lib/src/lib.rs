//! Engine-agnostic tic-tac-toe rules.
//!
//! This crate is the reference implementation of goal #4 in the repository
//! README: the rules of the game live here, with **no dependency on any game
//! engine**, so the same logic drives the terminal frontend
//! (`tic-tac-toe-cli`) and the bracket-lib frontend (`tic-tac-toe-brackets`).
//!
//! A frontend is responsible only for input and drawing. Everything that
//! decides *what is legal* and *who has won* is here — if a frontend ever has
//! to re-derive a rule, that rule belongs in this crate instead.
//!
//! The board is generalised beyond classic 3×3: any width and height, any
//! number of players, and a configurable run length needed to win (e.g.
//! 4-in-a-row on a 5×5 board).
//!
//! # Example
//! ```
//! use tic_tac_toe_lib::{Board, GameStatus, Player, TicTacToeGame};
//!
//! let mut game = TicTacToeGame::new(
//!     Board::new(3, 3),
//!     vec![Player::new("X".to_string(), 'X'), Player::new("O".to_string(), 'O')],
//!     3,
//! );
//!
//! // X down the first column, O alongside it.
//! game.take_turn(0, 0).unwrap(); // X
//! game.take_turn(0, 1).unwrap(); // O
//! game.take_turn(1, 0).unwrap(); // X
//! game.take_turn(1, 1).unwrap(); // O
//! game.take_turn(2, 0).unwrap(); // X — three in a column
//!
//! assert!(matches!(game.status(), GameStatus::Won(_)));
//! assert_eq!(game.winner().map(Player::symbol), Some('X'));
//! ```

use std::fmt;

/// The character stored in a cell that has not been played yet.
pub const EMPTY_SYMBOL: char = ' ';

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

/// Where the game currently stands.
///
/// Prefer this over calling [`TicTacToeGame::winner`] and
/// [`TicTacToeGame::is_draw`] separately — it makes the three outcomes
/// exhaustive, so a frontend cannot forget one.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GameStatus<'a> {
    /// Legal moves remain and nobody has won.
    InProgress,
    /// The given player has completed a winning run.
    Won(&'a Player),
    /// The board is full with no winner.
    Draw,
}

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
    /// The player who made this move.
    pub fn player(&self) -> &Player {
        &self.player
    }

    /// Where the move was made.
    pub fn placement(&self) -> Placement {
        self.placement
    }
}

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
    /// Prefer [`TicTacToeGame::take_turn`], which enforces the rules; this is
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

/// A game in progress: the board, the players, and the moves played so far.
#[derive(Debug, Clone)]
pub struct TicTacToeGame {
    players: Vec<Player>,
    turn_history: Vec<Turn>,
    how_many_to_win: usize,
    board: Board,
}

impl TicTacToeGame {
    /// Starts a game on `board` with `players` taking turns in order, where
    /// `how_many` symbols in a row wins.
    ///
    /// # Panics
    /// Panics if `players` is empty or `how_many` is zero.
    pub fn new(board: Board, players: Vec<Player>, how_many: usize) -> Self {
        assert!(!players.is_empty(), "a game needs at least one player");
        assert!(how_many > 0, "winning run length must be at least 1");
        Self {
            board,
            players,
            how_many_to_win: how_many,
            turn_history: Vec::new(),
        }
    }

    /// Clears the board and the turn history, keeping the players and rules.
    pub fn reset(&mut self) {
        let height = self.board.height();
        let width = self.board.width();
        self.board = Board::new(height, width);
        self.turn_history.clear();
    }

    /// How many moves have been played.
    pub fn turn_count(&self) -> usize {
        self.turn_history.len()
    }

    /// Every move played so far, oldest first.
    pub fn turn_history(&self) -> &[Turn] {
        &self.turn_history
    }

    /// How many symbols in a row are needed to win.
    pub fn how_many_to_win(&self) -> usize {
        self.how_many_to_win
    }

    /// The players, in turn order.
    pub fn players(&self) -> &[Player] {
        &self.players
    }

    /// How many players are in the game.
    pub fn number_of_players(&self) -> usize {
        self.players.len()
    }

    /// The player whose turn it is.
    pub fn current_player(&self) -> &Player {
        &self.players[self.turn_count() % self.number_of_players()]
    }

    /// The symbol the current player will place.
    pub fn current_symbol(&self) -> char {
        self.current_player().symbol()
    }

    /// Plays the current player's symbol at `(row, column)` and advances the turn.
    ///
    /// This is the only sanctioned way to mutate the board: it rejects moves
    /// off the board, onto an occupied cell, or after the game has ended, so
    /// every frontend enforces identical rules.
    ///
    /// # Errors
    /// Returns the [`MoveError`] describing why the move was rejected. The game
    /// is left untouched when a move is rejected.
    pub fn take_turn(&mut self, row: usize, column: usize) -> Result<(), MoveError> {
        if self.is_game_over() {
            return Err(MoveError::GameOver);
        }
        if !self.board.contains(row, column) {
            return Err(MoveError::OutOfBounds { row, column });
        }
        if !self.board.is_entry_empty(row, column) {
            return Err(MoveError::CellOccupied { row, column });
        }

        let player = self.current_player().clone();
        self.board.place(player.symbol(), row, column);
        self.turn_history.push(Turn {
            player,
            placement: Placement { row, column },
        });
        Ok(())
    }

    /// The board being played on.
    pub fn board(&self) -> &Board {
        &self.board
    }

    /// Number of columns on the board.
    pub fn width(&self) -> usize {
        self.board.width()
    }

    /// Number of rows on the board.
    pub fn height(&self) -> usize {
        self.board.height()
    }

    /// The board as rows of bare symbols.
    pub fn board_string(&self) -> String {
        self.board.to_string()
    }

    /// The board drawn with `|` and `-+-` separators.
    pub fn pretty_board(&self) -> String {
        self.board.to_pretty_string()
    }

    /// The symbol at a coordinate, or `None` if it is outside the board.
    pub fn get(&self, row: usize, column: usize) -> Option<char> {
        self.board.get(row, column)
    }

    /// Whether the cell at `(row, column)` completes a run of
    /// [`how_many_to_win`](Self::how_many_to_win) matching symbols.
    ///
    /// Walks the four axes (horizontal, vertical, and both diagonals) outward
    /// in both directions from the cell, counting matches. Returns `false` for
    /// an empty or out-of-bounds cell.
    pub fn check_for_win(&self, row: usize, column: usize) -> bool {
        let Some(symbol) = self.get(row, column) else {
            return false;
        };
        if symbol == EMPTY_SYMBOL {
            return false;
        }
        let height = self.board.height() as isize;
        let width = self.board.width() as isize;

        for (dr, dc) in [(0isize, 1isize), (1, 0), (1, 1), (1, -1)] {
            let mut count = 1;
            for sign in [-1isize, 1] {
                let mut r = row as isize + sign * dr;
                let mut c = column as isize + sign * dc;
                while r >= 0
                    && r < height
                    && c >= 0
                    && c < width
                    && self.get(r as usize, c as usize) == Some(symbol)
                {
                    count += 1;
                    r += sign * dr;
                    c += sign * dc;
                }
            }
            if count >= self.how_many_to_win {
                return true;
            }
        }
        false
    }

    /// The player who has won, or `None` if nobody has.
    ///
    /// Scans for a winning run and maps its symbol back to the player who owns
    /// it, so a frontend never has to infer the winner from the turn order.
    pub fn winner(&self) -> Option<&Player> {
        for row in 0..self.board.height() {
            for col in 0..self.board.width() {
                if self.check_for_win(row, col) {
                    let symbol = self.board.get(row, col)?;
                    return self.players.iter().find(|p| p.symbol() == symbol);
                }
            }
        }
        None
    }

    /// Whether any player has completed a winning run.
    pub fn has_winner(&self) -> bool {
        self.winner().is_some()
    }

    /// Whether the board is full with no winner.
    pub fn is_draw(&self) -> bool {
        self.board.is_full() && !self.has_winner()
    }

    /// Whether the game has ended, by win or by draw.
    pub fn is_game_over(&self) -> bool {
        self.board.is_full() || self.has_winner()
    }

    /// Where the game stands, as one exhaustive value.
    pub fn status(&self) -> GameStatus<'_> {
        match self.winner() {
            Some(player) => GameStatus::Won(player),
            None if self.board.is_full() => GameStatus::Draw,
            None => GameStatus::InProgress,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A standard 3×3, two-player, three-in-a-row game.
    fn game() -> TicTacToeGame {
        TicTacToeGame::new(
            Board::new(3, 3),
            vec![
                Player::new("Xavier".to_string(), 'X'),
                Player::new("Olive".to_string(), 'O'),
            ],
            3,
        )
    }

    /// Plays the given coordinates in order, asserting each is legal.
    fn play(game: &mut TicTacToeGame, moves: &[(usize, usize)]) {
        for &(row, column) in moves {
            game.take_turn(row, column)
                .unwrap_or_else(|e| panic!("({row}, {column}) should be legal: {e}"));
        }
    }

    #[test]
    fn new_board_is_empty_and_sized() {
        let board = Board::new(2, 5);
        assert_eq!(board.height(), 2);
        assert_eq!(board.width(), 5);
        assert!(!board.is_full());
        assert!(board.is_entry_empty(1, 4));
        assert_eq!(board.get(1, 4), Some(EMPTY_SYMBOL));
    }

    #[test]
    fn board_get_is_none_outside_the_board() {
        let board = Board::new(3, 3);
        assert_eq!(board.get(3, 0), None);
        assert_eq!(board.get(0, 3), None);
        assert!(!board.contains(3, 0));
        // `is_entry_empty` must not report out-of-bounds cells as playable.
        assert!(!board.is_entry_empty(3, 0));
    }

    #[test]
    fn display_matches_board_string() {
        let mut game = game();
        play(&mut game, &[(0, 0)]);
        assert_eq!(game.board().to_string(), "X  \n   \n   \n");
        assert_eq!(game.board_string(), game.board().to_string());
    }

    #[test]
    fn pretty_board_has_separators() {
        let board = Board::new(2, 2);
        assert_eq!(board.to_pretty_string(), " | \n-+-\n | \n");
    }

    #[test]
    fn players_alternate_by_turn_count() {
        let mut game = game();
        assert_eq!(game.current_symbol(), 'X');
        play(&mut game, &[(0, 0)]);
        assert_eq!(game.current_symbol(), 'O');
        play(&mut game, &[(0, 1)]);
        assert_eq!(game.current_symbol(), 'X');
        assert_eq!(game.current_player().name(), "Xavier");
    }

    #[test]
    fn turn_history_records_player_and_placement() {
        let mut game = game();
        play(&mut game, &[(1, 2)]);
        assert_eq!(game.turn_count(), 1);
        let turn = &game.turn_history()[0];
        assert_eq!(turn.player().symbol(), 'X');
        assert_eq!(turn.placement().row(), 1);
        assert_eq!(turn.placement().column(), 2);
    }

    #[test]
    fn occupied_cell_is_rejected_and_board_unchanged() {
        let mut game = game();
        play(&mut game, &[(1, 1)]);
        let before = game.board().clone();

        let err = game.take_turn(1, 1).unwrap_err();
        assert_eq!(err, MoveError::CellOccupied { row: 1, column: 1 });
        // A rejected move must not consume the turn or touch the board.
        assert_eq!(game.turn_count(), 1);
        assert_eq!(game.current_symbol(), 'O');
        assert_eq!(game.board(), &before);
    }

    #[test]
    fn out_of_bounds_move_is_rejected() {
        let mut game = game();
        assert_eq!(
            game.take_turn(3, 0).unwrap_err(),
            MoveError::OutOfBounds { row: 3, column: 0 }
        );
        assert_eq!(
            game.take_turn(0, 9).unwrap_err(),
            MoveError::OutOfBounds { row: 0, column: 9 }
        );
        assert_eq!(game.turn_count(), 0);
    }

    #[test]
    fn moves_after_the_game_ends_are_rejected() {
        let mut game = game();
        // X takes the top row, O answers in the middle row.
        play(&mut game, &[(0, 0), (1, 0), (0, 1), (1, 1), (0, 2)]);
        assert!(game.is_game_over());
        assert_eq!(game.take_turn(2, 2).unwrap_err(), MoveError::GameOver);
    }

    #[test]
    fn detects_a_row_win() {
        let mut game = game();
        play(&mut game, &[(0, 0), (1, 0), (0, 1), (1, 1), (0, 2)]);
        assert_eq!(game.winner().map(Player::name), Some("Xavier"));
        assert!(matches!(game.status(), GameStatus::Won(p) if p.symbol() == 'X'));
        assert!(!game.is_draw());
    }

    #[test]
    fn detects_a_column_win() {
        let mut game = game();
        play(&mut game, &[(0, 0), (0, 1), (1, 0), (1, 1), (2, 0)]);
        assert_eq!(game.winner().map(Player::symbol), Some('X'));
    }

    #[test]
    fn detects_both_diagonals() {
        let mut down = game();
        play(&mut down, &[(0, 0), (0, 1), (1, 1), (0, 2), (2, 2)]);
        assert_eq!(down.winner().map(Player::symbol), Some('X'));

        let mut up = game();
        play(&mut up, &[(0, 2), (0, 1), (1, 1), (0, 0), (2, 0)]);
        assert_eq!(up.winner().map(Player::symbol), Some('X'));
    }

    #[test]
    fn detects_a_draw_on_a_full_board() {
        let mut game = game();
        // X O X
        // X O O
        // O X X  — full, no three in a row.
        play(
            &mut game,
            &[
                (0, 0), // X
                (0, 1), // O
                (0, 2), // X
                (1, 1), // O
                (1, 0), // X
                (1, 2), // O
                (2, 1), // X
                (2, 0), // O
                (2, 2), // X
            ],
        );
        assert!(game.board().is_full());
        assert_eq!(game.winner(), None);
        assert!(game.is_draw());
        assert!(game.is_game_over());
        assert_eq!(game.status(), GameStatus::Draw);
    }

    #[test]
    fn game_in_progress_has_no_outcome() {
        let mut game = game();
        play(&mut game, &[(0, 0), (1, 1)]);
        assert_eq!(game.status(), GameStatus::InProgress);
        assert!(!game.is_game_over());
        assert!(!game.is_draw());
        assert_eq!(game.winner(), None);
    }

    #[test]
    fn supports_four_in_a_row_on_a_five_by_five_board() {
        let mut game = TicTacToeGame::new(
            Board::new(5, 5),
            vec![
                Player::new("Xavier".to_string(), 'X'),
                Player::new("Olive".to_string(), 'O'),
            ],
            4,
        );
        // X builds a run of four along row 0; three is not yet enough.
        play(&mut game, &[(0, 0), (4, 0), (0, 1), (4, 1), (0, 2), (4, 2)]);
        assert_eq!(game.status(), GameStatus::InProgress);
        play(&mut game, &[(0, 3)]);
        assert_eq!(game.winner().map(Player::symbol), Some('X'));
    }

    #[test]
    fn supports_more_than_two_players() {
        let mut game = TicTacToeGame::new(
            Board::new(3, 3),
            vec![
                Player::new("One".to_string(), 'X'),
                Player::new("Two".to_string(), 'O'),
                Player::new("Three".to_string(), 'Z'),
            ],
            3,
        );
        assert_eq!(game.number_of_players(), 3);
        assert_eq!(game.current_symbol(), 'X');
        play(&mut game, &[(0, 0), (0, 1), (0, 2)]);
        assert_eq!(game.current_symbol(), 'X');
        assert_eq!(game.turn_history()[2].player().name(), "Three");
    }

    #[test]
    fn reset_clears_board_and_history_but_keeps_rules() {
        let mut game = game();
        play(&mut game, &[(0, 0), (1, 1)]);
        game.reset();
        assert_eq!(game.turn_count(), 0);
        assert!(game.turn_history().is_empty());
        assert!(!game.board().is_full());
        assert_eq!(game.board().get(0, 0), Some(EMPTY_SYMBOL));
        assert_eq!(game.current_symbol(), 'X');
        assert_eq!(game.number_of_players(), 2);
        assert_eq!(game.how_many_to_win(), 3);
        assert_eq!(game.status(), GameStatus::InProgress);
    }

    #[test]
    fn check_for_win_ignores_empty_and_out_of_bounds_cells() {
        let game = game();
        assert!(!game.check_for_win(0, 0));
        assert!(!game.check_for_win(99, 99));
    }
}
