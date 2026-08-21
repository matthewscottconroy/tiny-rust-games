//! Tests for the searching opponent.
//!
//! The interesting ones are at the bottom. "Plays well" is not a property you
//! can check with examples, so the last two play every game that can be played
//! against it and assert it never loses one — which for 3x3 is the definition
//! of perfect play, and is cheap enough to do exhaustively.

use super::*;
use crate::{Board, Player};
use alloc::string::ToString as _;
use alloc::vec;

fn standard() -> TicTacToeGame {
    TicTacToeGame::new(
        Board::new(3, 3),
        vec![
            Player::new("X".to_string(), 'X'),
            Player::new("O".to_string(), 'O'),
        ],
        3,
    )
}

/// Plays a sequence of `(row, column)` moves, asserting each is legal.
fn play(game: &mut TicTacToeGame, moves: &[(usize, usize)]) {
    for &(row, column) in moves {
        game.take_turn(row, column).expect("scripted move is legal");
    }
}

// ── Refusals ─────────────────────────────────────────────────────────────────

#[test]
fn a_finished_game_has_no_best_move() {
    let mut game = standard();
    play(&mut game, &[(0, 0), (1, 0), (0, 1), (1, 1), (0, 2)]);
    assert!(matches!(game.status(), GameStatus::Won(_)));
    assert_eq!(best_move(&game), None);
}

#[test]
fn a_three_player_game_has_no_best_move() {
    // Negamax assumes one player's gain is exactly the other's loss, which is
    // false with three players — so it declines rather than answering wrongly.
    let game = TicTacToeGame::new(
        Board::new(3, 3),
        vec![
            Player::new("X".to_string(), 'X'),
            Player::new("O".to_string(), 'O'),
            Player::new("Z".to_string(), 'Z'),
        ],
        3,
    );
    assert_eq!(best_move(&game), None);
}

// ── Tactics ──────────────────────────────────────────────────────────────────

#[test]
fn it_takes_an_immediate_win() {
    let mut game = standard();
    play(&mut game, &[(0, 0), (1, 0), (0, 1), (1, 1)]);
    let mv = best_move(&game).expect("X has a move");
    assert_eq!((mv.row(), mv.column()), (0, 2), "should complete the row");
}

#[test]
fn it_blocks_an_immediate_loss() {
    let mut game = standard();
    play(&mut game, &[(0, 0), (2, 2), (0, 1)]);
    let mv = best_move(&game).expect("O has a move");
    assert_eq!((mv.row(), mv.column()), (0, 2), "should block the row");
}

#[test]
fn it_finishes_a_won_game_at_once() {
    // Without the depth term in the score every winning line rates the same and
    // the search has no reason to prefer the quick one.
    let mut game = standard();
    play(&mut game, &[(0, 0), (1, 1), (0, 1), (2, 2)]);
    let mv = best_move(&game).expect("X has a move");
    let mut finished = game.clone();
    finished.take_turn(mv.row(), mv.column()).expect("legal");
    assert!(
        matches!(finished.status(), GameStatus::Won(_)),
        "should win immediately, played {:?}",
        (mv.row(), mv.column())
    );
}

#[test]
fn alpha_beta_does_not_change_the_move_chosen() {
    // Pruning is an optimisation; if it ever changes an answer it is a bug. So
    // compare it against a full-width search on the same positions.
    fn unpruned(game: &TicTacToeGame, depth: usize, max_depth: usize) -> i32 {
        match game.status() {
            GameStatus::Won(_) => return -(WIN - depth as i32),
            GameStatus::Draw => return 0,
            GameStatus::InProgress => {}
        }
        if depth >= max_depth {
            return 0;
        }
        let mut best = -WIN * 2;
        for (row, column) in empty_cells(game) {
            let mut next = game.clone();
            if next.take_turn(row, column).is_err() {
                continue;
            }
            let score = -unpruned(&next, depth + 1, max_depth);
            if score > best {
                best = score;
            }
        }
        best
    }

    for opening in [(0usize, 0usize), (1, 1), (0, 1), (2, 0)] {
        let mut game = standard();
        play(&mut game, &[opening]);
        let pruned = best_move(&game).expect("a move exists");

        let mut best: Option<(i32, (usize, usize))> = None;
        for (row, column) in empty_cells(&game) {
            let mut next = game.clone();
            next.take_turn(row, column).expect("legal");
            let score = -unpruned(&next, 1, DEFAULT_MAX_DEPTH);
            if best.is_none_or(|(top, _)| score > top) {
                best = Some((score, (row, column)));
            }
        }
        let (_, expected) = best.expect("a move exists");
        assert_eq!(
            (pruned.row(), pruned.column()),
            expected,
            "pruned search disagreed after {opening:?}"
        );
    }
}

// ── Depth ────────────────────────────────────────────────────────────────────

#[test]
fn a_depth_of_zero_still_returns_a_legal_move() {
    let game = standard();
    let mv = best_move_to_depth(&game, 0).expect("a move exists");
    assert!(game.board().is_entry_empty(mv.row(), mv.column()));
    // With no search every move scores the same, so the documented row-major
    // tie-break decides — and that is what makes the opponent reproducible.
    // Mutation testing found nothing pinned this: flipping the comparison to
    // keep the *last* equal move instead of the first passed every test.
    assert_eq!((mv.row(), mv.column()), (0, 0), "ties go to the first cell");
}

#[test]
fn the_depth_limit_actually_limits() {
    // A search that ignored `max_depth` would still play well — better, even —
    // so no test of move quality can catch it. What proves the limit works is
    // that a shallow search and a deep one ever *disagree*.
    let disagreement = [(0usize, 0usize), (1, 1), (0, 1), (2, 0), (1, 0)]
        .into_iter()
        .any(|opening| {
            let mut game = standard();
            play(&mut game, &[opening]);
            let shallow = best_move_to_depth(&game, 2);
            let deep = best_move(&game);
            match (shallow, deep) {
                (Some(a), Some(b)) => (a.row(), a.column()) != (b.row(), b.column()),
                _ => false,
            }
        });
    assert!(
        disagreement,
        "a two-ply search agreed with a nine-ply one everywhere, so the depth \
         limit is not being applied"
    );
}

#[test]
fn a_shallow_search_still_takes_a_win_in_one() {
    let mut game = standard();
    play(&mut game, &[(0, 0), (1, 0), (0, 1), (1, 1)]);
    let mv = best_move_to_depth(&game, 1).expect("X has a move");
    assert_eq!((mv.row(), mv.column()), (0, 2));
}

// ── Perfect play, proved exhaustively ────────────────────────────────────────

/// Plays out every game the opponent can force from `game`.
///
/// The AI moves for `ai_symbol`; the other side tries *everything*. Returns
/// false as soon as any line ends with the AI having lost.
fn ai_never_loses(game: &TicTacToeGame, ai_symbol: char) -> bool {
    match game.status() {
        GameStatus::Won(player) => return player.symbol() == ai_symbol,
        GameStatus::Draw => return true,
        GameStatus::InProgress => {}
    }

    if game.current_symbol() == ai_symbol {
        let Some(mv) = best_move(game) else {
            return true;
        };
        let mut next = game.clone();
        next.take_turn(mv.row(), mv.column())
            .expect("AI move is legal");
        return ai_never_loses(&next, ai_symbol);
    }

    empty_cells(game).into_iter().all(|(row, column)| {
        let mut next = game.clone();
        next.take_turn(row, column).expect("legal");
        ai_never_loses(&next, ai_symbol)
    })
}

#[test]
fn a_perfect_player_never_loses_as_x() {
    // The whole claim, checked rather than asserted: tic-tac-toe is a draw
    // under perfect play, so the search must not lose a single game against any
    // opponent, however adversarial.
    assert!(ai_never_loses(&standard(), 'X'));
}

#[test]
fn a_perfect_player_never_loses_as_o() {
    // The harder seat: moving second, O can only ever hold the draw. Every
    // distinct opening is covered — corner, edge and centre.
    for opening in [(0usize, 0usize), (0, 1), (1, 1)] {
        let mut line = standard();
        line.take_turn(opening.0, opening.1).expect("legal");
        assert!(
            ai_never_loses(&line, 'O'),
            "lost a game after X opened {opening:?}"
        );
    }
}
