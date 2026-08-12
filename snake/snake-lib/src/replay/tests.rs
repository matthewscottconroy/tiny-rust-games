//! Tests for replay recording and playback.

use super::*;
use crate::{Direction, GameStatus};

/// Plays a game while recording it, returning both so they can be compared.
///
/// This mirrors what a frontend does: record the turn, then queue it.
fn play_and_record(
    width: i32,
    height: i32,
    seed: u64,
    script: &[(u64, Direction)],
    ticks: u64,
) -> (SnakeGame, Replay) {
    let mut game = SnakeGame::new(width, height, seed);
    let mut replay = Replay::recording(width, height, seed);
    for _ in 0..ticks {
        if game.is_over() {
            break;
        }
        for (tick, direction) in script {
            if *tick == game.ticks() {
                replay.record_turn(game.ticks(), *direction);
                game.queue_turn(*direction);
            }
        }
        game.step();
    }
    (game, replay)
}

/// Asserts two games are indistinguishable.
fn assert_same(a: &SnakeGame, b: &SnakeGame) {
    assert_eq!(a.ticks(), b.ticks(), "tick count");
    assert_eq!(a.score(), b.score(), "score");
    assert_eq!(a.status(), b.status(), "status");
    assert_eq!(a.food(), b.food(), "food");
    assert_eq!(a.direction(), b.direction(), "direction");
    assert_eq!(
        a.body().collect::<Vec<_>>(),
        b.body().collect::<Vec<_>>(),
        "body"
    );
}

#[test]
fn a_replay_reproduces_the_game_it_recorded() {
    let script = [
        (3, Direction::Down),
        (7, Direction::Left),
        (11, Direction::Up),
        (18, Direction::Right),
    ];
    let (live, replay) = play_and_record(20, 15, 7, &script, 40);
    assert_same(&live, &replay.play(40));
}

#[test]
fn a_replay_reproduces_a_death_exactly() {
    // The case that matters for bug reports: the recording must reach the same
    // end state, by the same cause, on the same tick.
    let script = [(2, Direction::Up)];
    let (live, replay) = play_and_record(12, 9, 3, &script, 200);
    assert!(live.is_over(), "expected the scripted game to end");
    let replayed = replay.play(200);
    assert_same(&live, &replayed);
    assert!(matches!(replayed.status(), GameStatus::Dead(_)));
}

#[test]
fn replays_are_reproducible_across_many_seeds() {
    let script = [(4, Direction::Down), (9, Direction::Right)];
    for seed in 0..40u64 {
        let (live, replay) = play_and_record(14, 11, seed, &script, 60);
        assert_same(&live, &replay.play(60));
    }
}

#[test]
fn play_out_runs_to_the_end_without_being_told_the_length() {
    let script = [(2, Direction::Down), (6, Direction::Left)];
    let (_, replay) = play_and_record(10, 10, 5, &script, 300);
    let played = replay.play_out(300);
    assert!(played.ticks() > 0);
}

#[test]
fn an_empty_recording_still_replays_the_default_run() {
    let replay = Replay::recording(20, 15, 42);
    let played = replay.play(10);
    let mut direct = SnakeGame::new(20, 15, 42);
    for _ in 0..10 {
        direct.step();
    }
    assert_same(&direct, &played);
}

// ── Text format ──────────────────────────────────────────────────────────────

#[test]
fn text_round_trips() {
    let mut replay = Replay::recording(20, 15, 99);
    replay.record_turn(1, Direction::Up);
    replay.record_turn(4, Direction::Left);
    replay.record_turn(9, Direction::Down);

    let text = replay.to_text();
    assert_eq!(Replay::from_text(&text).unwrap(), replay);
}

#[test]
fn the_text_format_is_readable() {
    let mut replay = Replay::recording(20, 15, 7);
    replay.record_turn(5, Direction::Down);
    let text = replay.to_text();

    assert!(text.starts_with("snake-replay 1\n"), "{text}");
    assert!(text.contains("board 20 15\n"), "{text}");
    assert!(text.contains("seed 7\n"), "{text}");
    assert!(text.contains("turn 5 down\n"), "{text}");
}

#[test]
fn a_parsed_replay_reproduces_the_original_game() {
    // The end-to-end promise: record, serialise, parse, replay, identical.
    let script = [(3, Direction::Down), (8, Direction::Left)];
    let (live, replay) = play_and_record(18, 12, 21, &script, 50);

    let parsed = Replay::from_text(&replay.to_text()).unwrap();
    assert_same(&live, &parsed.play(50));
}

#[test]
fn comments_and_blank_lines_are_ignored() {
    let text = "snake-replay 1\n\
                # recorded from a bug report\n\
                board 20 15\n\
                \n\
                seed 7\n\
                turn 5 down\n";
    let replay = Replay::from_text(text).unwrap();
    assert_eq!(replay.seed(), 7);
    assert_eq!(replay.turns(), [(5, Direction::Down)]);
}

#[test]
fn every_direction_survives_the_round_trip() {
    let mut replay = Replay::recording(10, 10, 0);
    for (tick, dir) in [
        Direction::Up,
        Direction::Down,
        Direction::Left,
        Direction::Right,
    ]
    .into_iter()
    .enumerate()
    {
        replay.record_turn(tick as u64, dir);
    }
    assert_eq!(Replay::from_text(&replay.to_text()).unwrap(), replay);
}

// ── Rejections ───────────────────────────────────────────────────────────────

#[test]
fn a_missing_header_is_rejected() {
    assert_eq!(
        Replay::from_text("board 20 15\n"),
        Err(ReplayError::MissingHeader)
    );
    assert_eq!(Replay::from_text(""), Err(ReplayError::MissingHeader));
}

#[test]
fn a_future_version_is_rejected_rather_than_guessed_at() {
    assert_eq!(
        Replay::from_text("snake-replay 99\nboard 4 4\n"),
        Err(ReplayError::UnsupportedVersion(99))
    );
}

#[test]
fn a_replay_without_a_board_is_rejected() {
    assert_eq!(
        Replay::from_text("snake-replay 1\nseed 3\n"),
        Err(ReplayError::MissingBoard)
    );
}

#[test]
fn a_malformed_line_reports_where() {
    let err = Replay::from_text("snake-replay 1\nboard 20 15\nturn five down\n").unwrap_err();
    match err {
        ReplayError::BadLine { line, ref text } => {
            assert_eq!(line, 3);
            assert!(text.contains("five"));
        }
        other => panic!("expected BadLine, got {other:?}"),
    }
}

#[test]
fn an_unknown_direction_is_rejected() {
    assert!(matches!(
        Replay::from_text("snake-replay 1\nboard 4 4\nturn 1 sideways\n"),
        Err(ReplayError::BadLine { .. })
    ));
}

#[test]
fn turns_recorded_out_of_order_are_rejected_on_parse() {
    assert_eq!(
        Replay::from_text("snake-replay 1\nboard 4 4\nturn 9 up\nturn 2 down\n"),
        Err(ReplayError::OutOfOrderTurn { tick: 2 })
    );
}

#[test]
#[should_panic(expected = "recorded after tick")]
fn recording_a_turn_in_the_past_panics() {
    let mut replay = Replay::recording(10, 10, 0);
    replay.record_turn(5, Direction::Up);
    replay.record_turn(2, Direction::Down);
}

#[test]
fn two_turns_on_one_tick_keep_the_last() {
    // Matches the live game, where the last request before a step wins.
    let mut replay = Replay::recording(10, 10, 0);
    replay.record_turn(3, Direction::Up);
    replay.record_turn(3, Direction::Left);
    assert_eq!(replay.turns(), [(3, Direction::Left)]);
}

#[test]
fn errors_describe_themselves() {
    // These surface in a CLI, so they must read as sentences.
    assert!(ReplayError::MissingHeader.to_string().contains("header"));
    assert!(
        ReplayError::UnsupportedVersion(9)
            .to_string()
            .contains("version 9")
    );
    assert!(
        ReplayError::OutOfOrderTurn { tick: 4 }
            .to_string()
            .contains("tick 4")
    );
}

// ── Regressions found by mutation testing ────────────────────────────────────

#[test]
fn a_replay_reports_the_shape_it_was_created_with() {
    // Mutation: `width`/`height` replaced by constants. Nothing read them back.
    let replay = Replay::recording(23, 17, 5);
    assert_eq!(replay.width(), 23);
    assert_eq!(replay.height(), 17);
    assert_eq!(replay.seed(), 5);
    // And the replayed game must actually use them.
    let game = replay.play(1);
    assert_eq!((game.width(), game.height()), (23, 17));
}

#[test]
fn last_turn_tick_reports_the_final_entry() {
    // Mutation: `last_turn_tick` replaced by None / Some(0) / Some(1).
    let mut replay = Replay::recording(10, 10, 0);
    assert_eq!(replay.last_turn_tick(), None);
    replay.record_turn(3, Direction::Up);
    assert_eq!(replay.last_turn_tick(), Some(3));
    replay.record_turn(11, Direction::Left);
    assert_eq!(replay.last_turn_tick(), Some(11));
}

#[test]
fn play_out_runs_past_the_last_recorded_turn() {
    // Mutation: the `last + 1 + extra` arithmetic in `play_out`. Nothing pinned
    // how far it actually runs.
    let mut replay = Replay::recording(30, 30, 1);
    replay.record_turn(4, Direction::Down);

    // Last turn at tick 4 means 5 ticks to reach it, plus the extra requested.
    assert_eq!(replay.play_out(0).ticks(), 5);
    assert_eq!(replay.play_out(3).ticks(), 8);
    // With no turns at all there is nothing to play out but the extra.
    assert_eq!(Replay::recording(30, 30, 1).play_out(6).ticks(), 6);
}

#[test]
fn a_hand_written_file_may_repeat_a_tick_and_the_last_wins() {
    // `record_turn` merges same-tick entries, so `to_text` never emits a
    // duplicate — but a human editing a bug report might. Parsing accepts it,
    // and playback applies them in order, matching the live game.
    let text = "snake-replay 1\nboard 10 10\nseed 0\nturn 2 down\nturn 2 left\n";
    let replay = Replay::from_text(text).expect("duplicate ticks are legal");
    assert_eq!(replay.turns().len(), 2);

    let played = replay.play(4);
    let mut direct = crate::SnakeGame::new(10, 10, 0);
    for _ in 0..4 {
        if direct.ticks() == 2 {
            direct.queue_turn(Direction::Down);
            direct.queue_turn(Direction::Left);
        }
        direct.step();
    }
    assert_eq!(played.direction(), direct.direction());
}
