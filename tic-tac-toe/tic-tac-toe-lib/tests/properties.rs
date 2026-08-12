//! Property-based tests for the tic-tac-toe rules.
//!
//! The library generalises well past 3x3 — arbitrary board sizes, any number of
//! players, any winning run length — and that is exactly the surface where
//! hand-written examples stop covering much. These assert invariants that must
//! hold across the whole space.

use proptest::prelude::*;
use tic_tac_toe_lib::{Board, EMPTY_SYMBOL, GameStatus, MoveError, Player, TicTacToeGame};

/// Builds a game with `n` players on a `rows` x `cols` board.
fn game(rows: usize, cols: usize, players: usize, win: usize) -> TicTacToeGame {
    let symbols = ['X', 'O', 'Z', 'W', 'V', 'U'];
    let roster = (0..players)
        .map(|i| Player::new(format!("P{i}"), symbols[i % symbols.len()]))
        .collect();
    TicTacToeGame::new(Board::new(rows, cols), roster, win)
}

proptest! {
    /// No board shape, player count, or move sequence may panic.
    #[test]
    fn playing_never_panics(
        rows in 1usize..7, cols in 1usize..7,
        players in 1usize..5, win in 1usize..6,
        moves in prop::collection::vec((0usize..8, 0usize..8), 0..60),
    ) {
        let mut g = game(rows, cols, players, win);
        for (r, c) in moves {
            let _ = g.take_turn(r, c);
            let _ = g.status();
        }
    }

    /// `turn_count` counts exactly the accepted moves.
    #[test]
    fn turn_count_tracks_accepted_moves(
        rows in 1usize..6, cols in 1usize..6,
        moves in prop::collection::vec((0usize..6, 0usize..6), 0..50),
    ) {
        let mut g = game(rows, cols, 2, 3);
        let mut accepted = 0;
        for (r, c) in moves {
            if g.take_turn(r, c).is_ok() {
                accepted += 1;
            }
        }
        prop_assert_eq!(g.turn_count(), accepted);
        prop_assert_eq!(g.turn_history().len(), accepted);
    }

    /// A rejected move never changes the game.
    #[test]
    fn rejected_moves_are_inert(
        rows in 1usize..6, cols in 1usize..6,
        moves in prop::collection::vec((0usize..8, 0usize..8), 0..40),
    ) {
        let mut g = game(rows, cols, 2, 3);
        for (r, c) in moves {
            // `GameStatus` borrows the game, so compare owned projections of
            // it rather than holding the value across the mutation.
            let board_before = g.board().clone();
            let turns_before = g.turn_count();
            let winner_before = g.winner().map(|p| p.symbol());
            let over_before = g.is_game_over();

            if g.take_turn(r, c).is_err() {
                prop_assert_eq!(g.board(), &board_before);
                prop_assert_eq!(g.turn_count(), turns_before);
                prop_assert_eq!(g.winner().map(|p| p.symbol()), winner_before);
                prop_assert_eq!(g.is_game_over(), over_before);
            }
        }
    }

    /// Out-of-bounds moves are always rejected as such.
    #[test]
    fn out_of_bounds_is_always_rejected(
        rows in 1usize..6, cols in 1usize..6,
        r in 0usize..20, c in 0usize..20,
    ) {
        let mut g = game(rows, cols, 2, 3);
        if r >= rows || c >= cols {
            prop_assert_eq!(
                g.take_turn(r, c),
                Err(MoveError::OutOfBounds { row: r, column: c })
            );
        }
    }

    /// The three outcomes are mutually exclusive and match the board.
    #[test]
    fn status_agrees_with_the_board(
        rows in 1usize..6, cols in 1usize..6,
        players in 1usize..4, win in 1usize..5,
        moves in prop::collection::vec((0usize..6, 0usize..6), 0..50),
    ) {
        let mut g = game(rows, cols, players, win);
        for (r, c) in moves {
            let _ = g.take_turn(r, c);
        }
        match g.status() {
            GameStatus::Won(p) => {
                prop_assert!(g.has_winner());
                prop_assert!(!g.is_draw());
                prop_assert!(g.is_game_over());
                // The winner is one of the players in the game.
                prop_assert!(g.players().iter().any(|q| q.symbol() == p.symbol()));
            }
            GameStatus::Draw => {
                prop_assert!(g.board().is_full());
                prop_assert!(g.winner().is_none());
                prop_assert!(g.is_draw());
                prop_assert!(g.is_game_over());
            }
            GameStatus::InProgress => {
                prop_assert!(!g.board().is_full());
                prop_assert!(g.winner().is_none());
                prop_assert!(!g.is_game_over());
            }
        }
    }

    /// No move is accepted after the game ends.
    #[test]
    fn a_finished_game_accepts_nothing(
        rows in 1usize..5, cols in 1usize..5,
        moves in prop::collection::vec((0usize..5, 0usize..5), 0..60),
    ) {
        let mut g = game(rows, cols, 2, 2);
        for (r, c) in moves {
            let over = g.is_game_over();
            let result = g.take_turn(r, c);
            if over {
                prop_assert_eq!(result, Err(MoveError::GameOver));
            }
        }
    }

    /// Every played cell holds the symbol of the player who played it, and
    /// every unplayed cell is empty.
    #[test]
    fn the_board_reflects_the_turn_history(
        rows in 1usize..6, cols in 1usize..6,
        players in 1usize..4,
        moves in prop::collection::vec((0usize..6, 0usize..6), 0..40),
    ) {
        let mut g = game(rows, cols, players, 3);
        for (r, c) in moves {
            let _ = g.take_turn(r, c);
        }
        let mut expected = vec![vec![EMPTY_SYMBOL; cols]; rows];
        for turn in g.turn_history() {
            let p = turn.placement();
            expected[p.row()][p.column()] = turn.player().symbol();
        }
        for (r, row) in expected.iter().enumerate() {
            for (c, want) in row.iter().enumerate() {
                prop_assert_eq!(g.get(r, c), Some(*want));
            }
        }
    }

    /// Reset always returns a game to its starting state.
    #[test]
    fn reset_restores_the_start(
        rows in 1usize..6, cols in 1usize..6,
        players in 1usize..4, win in 1usize..5,
        moves in prop::collection::vec((0usize..6, 0usize..6), 0..40),
    ) {
        let mut g = game(rows, cols, players, win);
        for (r, c) in moves {
            let _ = g.take_turn(r, c);
        }
        g.reset();

        prop_assert_eq!(g.turn_count(), 0);
        prop_assert!(g.turn_history().is_empty());
        prop_assert_eq!(g.width(), cols);
        prop_assert_eq!(g.height(), rows);
        prop_assert_eq!(g.how_many_to_win(), win);
        prop_assert_eq!(g.number_of_players(), players);
        for r in 0..rows {
            for c in 0..cols {
                prop_assert_eq!(g.get(r, c), Some(EMPTY_SYMBOL));
            }
        }
    }
}
