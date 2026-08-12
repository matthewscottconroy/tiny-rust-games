//! Property-based tests for the Snake rules.
//!
//! The unit tests in `src/tests.rs` pin specific scenarios. These assert
//! invariants that must hold for *every* board size, seed, and sequence of
//! inputs — the space where hand-written examples run out, and where the
//! subtle rules (tail-follow, reversal, food placement) actually live.

use proptest::prelude::*;
use snake_lib::{Coord, Direction, GameStatus, SnakeGame, StepOutcome};

/// An arbitrary steering direction.
fn any_direction() -> impl Strategy<Value = Direction> {
    prop_oneof![
        Just(Direction::Up),
        Just(Direction::Down),
        Just(Direction::Left),
        Just(Direction::Right),
    ]
}

/// Plays `inputs`, queuing each direction and stepping once.
fn play(game: &mut SnakeGame, inputs: &[Direction]) {
    for dir in inputs {
        game.queue_turn(*dir);
        game.step();
    }
}

proptest! {
    /// No board size, seed, or input sequence may panic.
    #[test]
    fn stepping_never_panics(
        w in 2i32..30, h in 2i32..30, seed: u64,
        inputs in prop::collection::vec(any_direction(), 0..300),
    ) {
        let mut game = SnakeGame::new(w, h, seed);
        play(&mut game, &inputs);
    }

    /// The snake occupies exactly `score + 1` cells: it starts at length 1 and
    /// grows by one per food eaten.
    #[test]
    fn length_always_equals_score_plus_one(
        w in 2i32..20, h in 2i32..20, seed: u64,
        inputs in prop::collection::vec(any_direction(), 0..300),
    ) {
        let mut game = SnakeGame::new(w, h, seed);
        play(&mut game, &inputs);
        prop_assert_eq!(game.len() as u32, game.score() + 1);
    }

    /// A living snake never overlaps itself.
    #[test]
    fn a_living_snake_never_self_intersects(
        w in 2i32..20, h in 2i32..20, seed: u64,
        inputs in prop::collection::vec(any_direction(), 0..300),
    ) {
        let mut game = SnakeGame::new(w, h, seed);
        for dir in inputs {
            game.queue_turn(dir);
            if game.step() == StepOutcome::Ended {
                break;
            }
            if game.status() == GameStatus::Running {
                let cells: Vec<Coord> = game.body().collect();
                let mut unique = cells.clone();
                unique.sort_by_key(|c| (c.x, c.y));
                unique.dedup();
                prop_assert_eq!(unique.len(), cells.len(), "snake overlaps itself");
            }
        }
    }

    /// A living snake is always entirely on the board.
    #[test]
    fn a_living_snake_stays_on_the_board(
        w in 2i32..20, h in 2i32..20, seed: u64,
        inputs in prop::collection::vec(any_direction(), 0..300),
    ) {
        let mut game = SnakeGame::new(w, h, seed);
        for dir in inputs {
            game.queue_turn(dir);
            game.step();
            if game.status() == GameStatus::Running {
                for cell in game.body() {
                    prop_assert!(game.contains(cell), "{:?} is off the board", cell);
                }
            }
        }
    }

    /// Food is never placed under the snake, and only vanishes on a full board.
    #[test]
    fn food_is_always_reachable(
        w in 2i32..15, h in 2i32..15, seed: u64,
        inputs in prop::collection::vec(any_direction(), 0..300),
    ) {
        let mut game = SnakeGame::new(w, h, seed);
        for dir in inputs {
            game.queue_turn(dir);
            game.step();
            match game.food() {
                Some(food) => {
                    prop_assert!(game.contains(food));
                    prop_assert!(!game.body().any(|c| c == food), "food under the snake");
                }
                None => {
                    // The only legitimate reason for no food is a full board.
                    prop_assert_eq!(game.len() as i32, w * h);
                }
            }
        }
    }

    /// The same seed and inputs always produce the same game.
    #[test]
    fn play_is_deterministic(
        w in 2i32..20, h in 2i32..20, seed: u64,
        inputs in prop::collection::vec(any_direction(), 0..200),
    ) {
        let mut a = SnakeGame::new(w, h, seed);
        let mut b = SnakeGame::new(w, h, seed);
        play(&mut a, &inputs);
        play(&mut b, &inputs);

        prop_assert_eq!(a.score(), b.score());
        prop_assert_eq!(a.ticks(), b.ticks());
        prop_assert_eq!(a.status(), b.status());
        prop_assert_eq!(a.food(), b.food());
        prop_assert_eq!(a.body().collect::<Vec<_>>(), b.body().collect::<Vec<_>>());
    }

    /// Once a game ends it stays ended, and stops changing.
    #[test]
    fn an_ended_game_is_frozen(
        w in 2i32..12, h in 2i32..12, seed: u64,
        inputs in prop::collection::vec(any_direction(), 0..400),
    ) {
        let mut game = SnakeGame::new(w, h, seed);
        play(&mut game, &inputs);
        if !game.is_over() {
            return Ok(());
        }

        let before = (game.score(), game.ticks(), game.status(), game.len());
        for dir in [Direction::Up, Direction::Left, Direction::Down] {
            prop_assert!(!game.queue_turn(dir), "a finished game accepted a turn");
            prop_assert_eq!(game.step(), StepOutcome::Ended);
        }
        prop_assert_eq!(
            (game.score(), game.ticks(), game.status(), game.len()),
            before
        );
    }

    /// A snake with a neck can never be steered into a reversal.
    #[test]
    fn a_reversal_is_never_accepted(
        w in 4i32..20, h in 4i32..20, seed: u64,
        inputs in prop::collection::vec(any_direction(), 0..200),
    ) {
        let mut game = SnakeGame::new(w, h, seed);
        for dir in inputs {
            if game.is_over() {
                break;
            }
            let travelling = game.direction();
            let accepted = game.queue_turn(dir);
            if game.len() > 1 && dir == travelling.opposite() {
                prop_assert!(!accepted, "reversal was accepted");
            }
            game.step();
        }
    }

    /// A win means the snake fills the board; a loss means it does not.
    #[test]
    fn the_end_state_is_consistent(
        w in 2i32..8, h in 2i32..8, seed: u64,
        inputs in prop::collection::vec(any_direction(), 0..600),
    ) {
        let mut game = SnakeGame::new(w, h, seed);
        play(&mut game, &inputs);
        match game.status() {
            GameStatus::Won => {
                prop_assert_eq!(game.len() as i32, w * h);
                prop_assert_eq!(game.food(), None);
            }
            GameStatus::Dead(_) => {
                prop_assert!((game.len() as i32) <= w * h);
            }
            GameStatus::Running => {
                prop_assert!(game.food().is_some());
            }
        }
    }
}
