//! The rules: turn order, move validation, and how a game ends.
//!
//! This is the module a frontend talks to. [`TicTacToeGame::take_turn`] is the
//! only way to mutate the board, and [`TicTacToeGame::status`] is the only
//! thing a frontend needs to render an outcome.

use crate::EMPTY_SYMBOL;
use crate::board::Board;
use crate::error::MoveError;
use crate::player::{Placement, Player, Turn};

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
        self.turn_history
            .push(Turn::new(player, Placement::new(row, column)));
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
        for (dr, dc) in [(0isize, 1isize), (1, 0), (1, 1), (1, -1)] {
            let mut count = 1;
            for sign in [-1isize, 1] {
                let mut r = row as isize + sign * dr;
                let mut c = column as isize + sign * dc;
                // `symbol_at` handles the board edges, so this loop does not
                // compare against width/height itself. It used to, and mutation
                // testing showed those comparisons could be broken without any
                // test noticing — because running off the edge already yields
                // `None`, which ends the walk regardless.
                while self.symbol_at(r, c) == Some(symbol) {
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

    /// The symbol at a possibly-off-board coordinate.
    ///
    /// Takes signed coordinates so a walk outward from a cell can step past the
    /// edge without the caller bounds-checking first; anything outside the
    /// board is `None`.
    fn symbol_at(&self, row: isize, column: isize) -> Option<char> {
        let row = usize::try_from(row).ok()?;
        let column = usize::try_from(column).ok()?;
        self.get(row, column)
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

    fn game() -> TicTacToeGame {
        TicTacToeGame::new(
            Board::new(2, 3),
            vec![
                Player::new("Xavier".to_string(), 'X'),
                Player::new("Olive".to_string(), 'O'),
            ],
            3,
        )
    }

    #[test]
    fn players_are_returned_in_turn_order() {
        let game = game();
        let names: Vec<&str> = game.players().iter().map(Player::name).collect();
        assert_eq!(names, ["Xavier", "Olive"]);
    }

    #[test]
    fn width_and_height_mirror_the_board() {
        let game = game();
        assert_eq!(game.width(), 3);
        assert_eq!(game.height(), 2);
        assert_eq!(
            (game.width(), game.height()),
            (game.board().width(), game.board().height())
        );
    }

    #[test]
    fn pretty_board_matches_the_boards_own_rendering() {
        let mut game = game();
        game.take_turn(0, 0).unwrap();
        assert_eq!(game.pretty_board(), game.board().to_pretty_string());
        assert!(game.pretty_board().contains('X'));
        // The pretty form separates cells; the plain form does not.
        assert!(game.pretty_board().contains('|'));
        assert!(!game.board_string().contains('|'));
    }
}
