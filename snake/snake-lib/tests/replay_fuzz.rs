//! Robustness tests for the replay parser.
//!
//! [`Replay::from_text`] is the one function in this crate that reads input the
//! program did not produce: a file attached to a bug report, pasted into an
//! issue, or recovered from a crashed session. Everything else is called with
//! values the caller already had.
//!
//! That makes it the only place where malformed input must produce an `Err`
//! rather than a panic, and the only place worth fuzzing. These are property
//! tests rather than a `cargo-fuzz` target so they run on stable in ordinary
//! CI; the generators below are shaped to spend most of their time near the
//! grammar's edges, where a purely random byte stream would almost never land.
//!
//! This is not hypothetical. The `InvalidBoard` variant exists because these
//! tests found that `from_text` accepted `board 0 0`, which parsed cleanly and
//! then panicked inside `SnakeGame::new` the first time the replay was played.

use proptest::prelude::*;
use snake_lib::{Direction, MAX_BOARD_SIZE, MIN_BOARD_SIZE, Replay, SnakeGame};

/// Tokens that appear in the format, so mutations stay near-valid.
fn any_token() -> impl Strategy<Value = String> {
    prop_oneof![
        Just("snake-replay".to_string()),
        Just("board".to_string()),
        Just("seed".to_string()),
        Just("turn".to_string()),
        Just("up".to_string()),
        Just("down".to_string()),
        Just("left".to_string()),
        Just("right".to_string()),
        Just("#".to_string()),
        Just("1".to_string()),
        Just("0".to_string()),
        Just("-1".to_string()),
        // Values chosen to probe the integer parsers' limits.
        Just("2147483648".to_string()),
        Just("18446744073709551616".to_string()),
        Just("".to_string()),
        "[a-z0-9-]{0,8}",
    ]
}

/// A line assembled from those tokens.
fn any_line() -> impl Strategy<Value = String> {
    prop::collection::vec(any_token(), 0..5).prop_map(|parts| parts.join(" "))
}

/// A whole file of such lines.
fn near_valid_text() -> impl Strategy<Value = String> {
    prop::collection::vec(any_line(), 0..12).prop_map(|lines| lines.join("\n"))
}

/// Board edges worth testing: every boundary, and a sample either side.
fn board_edge() -> impl Strategy<Value = i32> {
    prop_oneof![
        Just(i32::MIN),
        Just(-1),
        Just(0),
        Just(MIN_BOARD_SIZE - 1),
        Just(MIN_BOARD_SIZE),
        Just(MIN_BOARD_SIZE + 1),
        2..64i32,
        Just(MAX_BOARD_SIZE - 1),
        Just(MAX_BOARD_SIZE),
        Just(MAX_BOARD_SIZE + 1),
        Just(i32::MAX),
    ]
}

/// An arbitrary direction.
fn any_direction() -> impl Strategy<Value = Direction> {
    prop_oneof![
        Just(Direction::Up),
        Just(Direction::Down),
        Just(Direction::Left),
        Just(Direction::Right),
    ]
}

/// A valid replay, built through the recording API.
fn any_replay() -> impl Strategy<Value = Replay> {
    (
        MIN_BOARD_SIZE..40i32,
        MIN_BOARD_SIZE..40i32,
        any::<u64>(),
        prop::collection::vec((0u64..500, any_direction()), 0..40),
    )
        .prop_map(|(w, h, seed, mut turns)| {
            // `record_turn` requires ascending ticks, which is the recorder's
            // contract rather than something to fuzz here.
            turns.sort_by_key(|(tick, _)| *tick);
            let mut replay = Replay::recording(w, h, seed);
            for (tick, direction) in turns {
                replay.record_turn(tick, direction);
            }
            replay
        })
}

proptest! {
    /// Arbitrary text must never panic the parser.
    #[test]
    fn parsing_arbitrary_text_never_panics(text in ".*") {
        let _ = Replay::from_text(&text);
    }

    /// Neither may text built from the format's own vocabulary, which gets far
    /// closer to the parser's branches than random bytes ever would.
    #[test]
    fn parsing_near_valid_text_never_panics(text in near_valid_text()) {
        let _ = Replay::from_text(&text);
    }

    /// Corrupting one line of a real replay must not panic either.
    #[test]
    fn parsing_a_corrupted_replay_never_panics(
        replay in any_replay(),
        index: prop::sample::Index,
        replacement in any_line(),
    ) {
        let text = replay.to_text();
        let mut lines: Vec<&str> = text.lines().collect();
        if !lines.is_empty() {
            let i = index.index(lines.len());
            lines[i] = &replacement;
        }
        let _ = Replay::from_text(&lines.join("\n"));
    }

    /// **The invariant that matters.** Anything the parser accepts must be
    /// playable — accepting a replay and then panicking when it is played is
    /// the bug this file was written to prevent, and the reason
    /// `ReplayError::InvalidBoard` exists.
    #[test]
    fn every_accepted_replay_can_be_played(text in near_valid_text()) {
        if let Ok(replay) = Replay::from_text(&text) {
            let game = replay.play_out(20);
            // Not merely "it did not panic": the replayed game must also be
            // internally consistent, which is the same invariant the property
            // tests assert for a game played live.
            prop_assert_eq!(game.len() as u32, game.score() + 1);
        }
    }

    /// The same, for deliberately hostile board sizes.
    ///
    /// The edges are sampled from a curated set rather than all of `i32`.
    /// Sampling uniformly would spend nearly every case on a board of hundreds
    /// of millions of cells, where the O(board) food scan takes minutes — the
    /// first version of this test simply hung. The interesting values are the
    /// boundaries, and those are enumerated here.
    #[test]
    fn every_accepted_board_size_is_playable_or_refused(
        w in board_edge(),
        h in board_edge(),
    ) {
        let text = format!("snake-replay 1\nboard {w} {h}\nseed 3\n");
        match Replay::from_text(&text) {
            Ok(replay) => {
                prop_assert!((MIN_BOARD_SIZE..=MAX_BOARD_SIZE).contains(&w));
                prop_assert!((MIN_BOARD_SIZE..=MAX_BOARD_SIZE).contains(&h));
                // Only actually play the small ones; a legal 4096-square board
                // is playable but slow, and that is not what is under test.
                if w <= 64 && h <= 64 {
                    let _ = replay.play(10);
                }
            }
            Err(_) => {
                prop_assert!(
                    !(MIN_BOARD_SIZE..=MAX_BOARD_SIZE).contains(&w)
                        || !(MIN_BOARD_SIZE..=MAX_BOARD_SIZE).contains(&h),
                    "a legal {}x{} board was refused",
                    w,
                    h
                );
            }
        }
    }

    /// Writing a replay and reading it back returns the same replay.
    #[test]
    fn text_round_trips(replay in any_replay()) {
        let parsed = Replay::from_text(&replay.to_text()).expect("own output must parse");
        prop_assert_eq!(parsed, replay);
    }

    /// And the round trip preserves behaviour, not merely the fields.
    #[test]
    fn a_round_tripped_replay_plays_identically(replay in any_replay()) {
        let parsed = Replay::from_text(&replay.to_text()).expect("own output must parse");
        let a: SnakeGame = replay.play_out(30);
        let b: SnakeGame = parsed.play_out(30);
        prop_assert_eq!(a.score(), b.score());
        prop_assert_eq!(a.ticks(), b.ticks());
        prop_assert_eq!(a.body().collect::<Vec<_>>(), b.body().collect::<Vec<_>>());
    }
}
