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

#![cfg_attr(not(test), no_std)]

// These rules need no engine, no clock and no operating system: the crate
// builds for a bare-metal Cortex-M4, which CI checks. `alloc` is still
// required, because a board is a `Vec` of rows and a player has a `String`
// name — refusing allocation too would mean a fixed-size board, which is a
// worse teaching example than a heap allocation is a cost.
//
// `breakout-lib` deliberately does *not* do this. It needs `f32::sqrt` to keep
// the ball's speed constant, and square root is a libm intrinsic that `core`
// does not provide, so `no_std` there would mean taking a dependency to gain
// nothing a browser or a desktop cares about. Discrete games get this for free;
// continuous physics does not.
extern crate alloc;

mod board;
mod error;
mod game;
mod player;

pub use board::Board;
pub use error::MoveError;
pub use game::{GameStatus, TicTacToeGame};
pub use player::{Placement, Player, Turn};

/// The character stored in a cell that has not been played yet.
pub const EMPTY_SYMBOL: char = ' ';

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
