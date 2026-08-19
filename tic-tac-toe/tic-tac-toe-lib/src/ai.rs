//! An opponent that plays perfectly, by searching the game tree.
//!
//! This lives in the rules crate rather than in a frontend, for the same reason
//! the rules do: an opponent is not a property of Bevy or of a terminal. Put it
//! in one frontend and the others either go without or grow their own, at which
//! point two versions of "the computer" disagree about how hard the game is.
//!
//! # Negamax
//!
//! Minimax with the observation that one player's gain is exactly the other's
//! loss, so a single function suffices if each level negates the level below.
//! [`best_move`] returns the move with the best score for whoever is to play.
//!
//! Alpha-beta pruning cuts branches that cannot affect the result: once a reply
//! is found that refutes a move, the rest of that move's replies do not matter,
//! because the opponent only needs one refutation. It changes nothing about the
//! move chosen — only how long it takes to choose it, which for an empty 3x3
//! board is the difference between a few hundred thousand positions and a few
//! thousand.
//!
//! # Depth, and why there is a limit
//!
//! Tic-tac-toe on 3x3 is small enough to search exhaustively, and the default
//! depth does. But [`TicTacToeGame`] permits any board size and any run length,
//! and the tree grows as the factorial of the empty cells — a 5x5 board is
//! about 15 *trillion* positions. So the search takes a depth limit and scores
//! anything beyond it as a draw. The result is still a strong player and no
//! longer a perfect one; on the boards this game is actually played on, the
//! limit is never reached.
//!
//! ```
//! use tic_tac_toe_lib::{Board, Player, TicTacToeGame, ai};
//!
//! let mut game = TicTacToeGame::new(
//!     Board::new(3, 3),
//!     vec![Player::new("X".into(), 'X'), Player::new("O".into(), 'O')],
//!     3,
//! );
//! // X takes two of a row; O must block or lose.
//! game.take_turn(0, 0).unwrap();
//! game.take_turn(2, 2).unwrap();
//! game.take_turn(0, 1).unwrap();
//!
//! let block = ai::best_move(&game).expect("O has a move");
//! assert_eq!((block.row(), block.column()), (0, 2));
//! ```

use alloc::vec::Vec;

use crate::{GameStatus, Placement, TicTacToeGame};

/// Score of a win, less the number of plies taken to reach it.
///
/// Subtracting the depth is what makes the opponent finish a game it has
/// already won instead of wandering: every winning line scores the same
/// otherwise, so it has no reason to prefer the quick one. It also makes it
/// choose the *slowest* loss, which looks like resistance rather than
/// resignation.
const WIN: i32 = 1_000;

/// How deep the search goes unless told otherwise.
///
/// Nine plies is a full 3x3 board, so the default is exhaustive there.
pub const DEFAULT_MAX_DEPTH: usize = 9;

/// The best move for the player to play, or `None` if there is not one.
///
/// Returns `None` when the game has already ended, when the board is full, or
/// when the game does not have exactly two players — negamax assumes that one
/// player's gain is the other's loss, which stops being true with three.
pub fn best_move(game: &TicTacToeGame) -> Option<Placement> {
    best_move_to_depth(game, DEFAULT_MAX_DEPTH)
}

/// [`best_move`], with an explicit search depth in plies.
///
/// Use this on boards larger than 3x3, where an exhaustive search is not
/// affordable. A depth of 0 searches nothing and returns the first legal move.
pub fn best_move_to_depth(game: &TicTacToeGame, max_depth: usize) -> Option<Placement> {
    if game.number_of_players() != 2 || game.status() != GameStatus::InProgress {
        return None;
    }

    let mut best: Option<(i32, Placement)> = None;
    for (row, column) in empty_cells(game) {
        let mut next = game.clone();
        if next.take_turn(row, column).is_err() {
            continue;
        }
        // The child is scored from the opponent's point of view, so negate it.
        let score = -negamax(&next, 1, max_depth, -WIN * 2, WIN * 2);
        if best.is_none_or(|(top, _)| score > top) {
            best = Some((score, Placement::new(row, column)));
        }
    }
    best.map(|(_, placement)| placement)
}

/// Scores a position for whoever is to play, in `alpha..beta`.
fn negamax(game: &TicTacToeGame, depth: usize, max_depth: usize, alpha: i32, beta: i32) -> i32 {
    match game.status() {
        // A won position is only ever reached after the winner has moved, and
        // the turn has already advanced — so the player to move here is the one
        // who just lost.
        GameStatus::Won(_) => return -(WIN - depth as i32),
        GameStatus::Draw => return 0,
        GameStatus::InProgress => {}
    }
    if depth >= max_depth {
        // Beyond the horizon, claim nothing rather than guess.
        return 0;
    }

    let mut alpha = alpha;
    let mut best = -WIN * 2;
    for (row, column) in empty_cells(game) {
        let mut next = game.clone();
        if next.take_turn(row, column).is_err() {
            continue;
        }
        let score = -negamax(&next, depth + 1, max_depth, -beta, -alpha);
        if score > best {
            best = score;
        }
        if best > alpha {
            alpha = best;
        }
        if alpha >= beta {
            // The opponent has a refutation already; the rest cannot matter.
            break;
        }
    }
    best
}

/// Every empty cell, in row-major order.
///
/// Row-major rather than "interesting first": the order decides which of two
/// equally good moves is chosen, and a fixed order makes the opponent
/// reproducible, which is what lets the tests below assert exact squares.
fn empty_cells(game: &TicTacToeGame) -> Vec<(usize, usize)> {
    let mut cells = Vec::new();
    for row in 0..game.height() {
        for column in 0..game.width() {
            if game.board().is_entry_empty(row, column) {
                cells.push((row, column));
            }
        }
    }
    cells
}

#[cfg(test)]
mod tests;
