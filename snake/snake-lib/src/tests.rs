//! Unit tests for the Snake rules.
//!
//! Split into its own file because the rules are subtle enough that the tests
//! are longer than the implementation, and burying them under it made `lib.rs`
//! hard to read.

use super::*;

/// A game with the snake at a known place, travelling right.
fn game() -> SnakeGame {
    SnakeGame::new(10, 10, 1)
}

/// Runs the snake into the right-hand wall and returns the fatal outcome.
///
/// Bounded, though a 10x10 board kills it within a handful of steps. Mutation
/// testing is why: a mutant that makes `contains` always true produces an
/// immortal snake, and an unbounded loop turns that into a hung test rather
/// than a failed one. A hang is a far worse diagnosis — it names no line.
fn run_into_wall(game: &mut SnakeGame) -> StepOutcome {
    for _ in 0..1_000 {
        let outcome = game.step();
        if matches!(outcome, StepOutcome::Died(_) | StepOutcome::Ended) {
            return outcome;
        }
    }
    panic!("the snake never hit the wall, so it cannot leave the board");
}

// ── Ticker ───────────────────────────────────────────────────────────────────

#[test]
fn ticker_yields_no_steps_before_the_interval_elapses() {
    let mut t = Ticker::new(10.0); // 100 ms per step
    assert_eq!(t.accumulate(0.05), 0);
    assert_eq!(t.accumulate(0.04), 0);
}

#[test]
fn ticker_yields_a_step_once_the_interval_elapses() {
    let mut t = Ticker::new(10.0);
    assert_eq!(t.accumulate(0.10), 1);
}

#[test]
fn ticker_yields_several_steps_for_a_long_frame() {
    let mut t = Ticker::new(8.0); // 125 ms per step
    assert_eq!(t.accumulate(0.25), 2);
}

#[test]
fn ticker_keeps_the_remainder_so_frame_rate_does_not_change_speed() {
    // 60 frames of 1/60 s at 10 steps/s is 10 steps — not 6 (which is what
    // discarding the remainder each frame would give) and not 11.
    let mut t = Ticker::new(10.0);
    let steps: u32 = (0..60).map(|_| t.accumulate(1.0 / 60.0)).sum();
    assert_eq!(steps, 10);
}

#[test]
fn ticker_agrees_across_frame_rates_exactly() {
    // This used to allow a one-step discrepancy, because a float accumulator
    // drifted: 100 frames of 0.01 summed to 0.99999998 and lost a step against
    // four frames of 0.25. Counting whole microseconds instead makes the two
    // agree exactly, which is what replays and lockstep need.
    let mut many = Ticker::new(10.0);
    let mut few = Ticker::new(10.0);
    let mut mixed = Ticker::new(10.0);

    let many_steps: u32 = (0..100).map(|_| many.accumulate(0.01)).sum();
    let few_steps: u32 = (0..4).map(|_| few.accumulate(0.25)).sum();
    let mixed_steps: u32 = [0.004, 0.030, 0.016, 0.100, 0.050, 0.300, 0.200, 0.300]
        .iter()
        .map(|d| mixed.accumulate(*d))
        .sum();

    assert_eq!(many_steps, 10, "100 frames of 10 ms");
    assert_eq!(few_steps, 10, "4 frames of 250 ms");
    assert_eq!(mixed_steps, 10, "8 uneven frames");
}

#[test]
fn seconds_to_micros_rounds_rather_than_truncating() {
    // Truncation is what lost the step: 0.01f32 is really 0.00999999977, so
    // truncating gives 9_999 and a hundred of them fall short of a second.
    assert_eq!(seconds_to_micros(0.01), 10_000);
    assert_eq!(seconds_to_micros(0.25), 250_000);
    assert_eq!(seconds_to_micros(1.0), MICROS_PER_SECOND);
}

#[test]
fn seconds_to_micros_collapses_nonsense_to_zero() {
    assert_eq!(seconds_to_micros(-1.0), 0);
    assert_eq!(seconds_to_micros(0.0), 0);
    assert_eq!(seconds_to_micros(f32::NAN), 0);
    assert_eq!(seconds_to_micros(f32::NEG_INFINITY), 0);
}

#[test]
fn ticker_accepts_exact_microsecond_input() {
    // The path a replay uses: no float conversion anywhere.
    let mut t = Ticker::new(8.0);
    assert_eq!(t.micros_per_step(), 125_000);
    assert_eq!(t.accumulate_micros(124_999), 0);
    assert_eq!(t.accumulate_micros(1), 1);
    assert_eq!(t.accumulate_micros(250_000), 2);
}

#[test]
fn two_tickers_fed_the_same_time_in_different_groupings_agree() {
    // The property that makes deterministic replay possible.
    let mut a = Ticker::new(9.0);
    let mut b = Ticker::new(9.0);
    let mut a_steps = 0;
    let mut b_steps = 0;

    for _ in 0..600 {
        a_steps += a.accumulate_micros(16_667); // ~60 fps
    }
    for _ in 0..200 {
        b_steps += b.accumulate_micros(50_001); // ~20 fps, same total
    }
    assert_eq!(a_steps, b_steps);
    assert_eq!(a, b, "including the leftover remainder");
}

#[test]
fn ticker_caps_a_long_stall_instead_of_teleporting() {
    let mut t = Ticker::new(10.0);
    // Ten seconds would be 100 steps; the cap prevents that.
    assert_eq!(t.accumulate(10.0), Ticker::MAX_STEPS_PER_CALL);
    // The surplus is dropped rather than paid out on the next call.
    assert_eq!(t.accumulate(0.0), 0);
}

#[test]
fn ticker_ignores_negative_and_zero_deltas() {
    let mut t = Ticker::new(10.0);
    assert_eq!(t.accumulate(-1.0), 0);
    assert_eq!(t.accumulate(0.0), 0);
    assert_eq!(t.accumulate(0.10), 1);
}

#[test]
fn ticker_alpha_reports_progress_toward_the_next_step() {
    let mut t = Ticker::new(10.0);
    assert_eq!(t.alpha(), 0.0);
    t.accumulate(0.05);
    assert!((t.alpha() - 0.5).abs() < 1e-5, "got {}", t.alpha());
    t.accumulate(0.05); // completes a step; remainder returns to zero
    assert!(t.alpha() < 1e-5, "got {}", t.alpha());
}

#[test]
fn ticker_reports_its_rate() {
    assert!((Ticker::new(8.0).steps_per_second() - 8.0).abs() < 1e-5);
}

#[test]
#[should_panic(expected = "steps_per_second must be positive")]
fn ticker_rejects_a_non_positive_rate() {
    Ticker::new(0.0);
}

#[test]
#[should_panic(expected = "positive and finite")]
fn ticker_rejects_a_non_finite_rate() {
    Ticker::new(f32::NAN);
}

#[test]
#[should_panic(expected = "faster than one step per microsecond")]
fn ticker_rejects_an_absurdly_fast_rate() {
    Ticker::new(10_000_000.0);
}

// ── Direction ────────────────────────────────────────────────────────────────

#[test]
fn opposite_is_its_own_inverse() {
    for d in [
        Direction::Up,
        Direction::Down,
        Direction::Left,
        Direction::Right,
    ] {
        assert_eq!(d.opposite().opposite(), d);
        assert_ne!(d.opposite(), d);
    }
}

#[test]
fn deltas_are_unit_steps_on_one_axis() {
    for d in [
        Direction::Up,
        Direction::Down,
        Direction::Left,
        Direction::Right,
    ] {
        let c = d.delta();
        assert_eq!(c.x.abs() + c.y.abs(), 1, "{d:?} is not a unit step");
    }
    assert_eq!(Direction::Up.delta(), Coord::new(0, -1));
    assert_eq!(Direction::Down.delta(), Coord::new(0, 1));
}

#[test]
fn opposite_directions_cancel() {
    for d in [Direction::Up, Direction::Left] {
        let a = d.delta();
        let b = d.opposite().delta();
        assert_eq!((a.x + b.x, a.y + b.y), (0, 0));
    }
}

// ── Setup ────────────────────────────────────────────────────────────────────

#[test]
fn a_new_game_starts_running_with_one_segment_and_food() {
    let g = game();
    assert_eq!(g.status(), GameStatus::Running);
    assert_eq!(g.len(), 1);
    assert_eq!(g.score(), 0);
    assert_eq!(g.ticks(), 0);
    assert!(g.food().is_some());
    assert!(!g.is_over());
}

#[test]
fn food_never_starts_under_the_snake() {
    for seed in 0..64 {
        let g = SnakeGame::new(5, 5, seed);
        assert_ne!(g.food(), Some(g.head()), "seed {seed}");
    }
}

#[test]
#[should_panic(expected = "board must be at least 2x2")]
fn a_degenerate_board_is_rejected() {
    SnakeGame::new(1, 10, 0);
}

#[test]
fn the_same_seed_produces_the_same_game() {
    let mut a = SnakeGame::new(8, 8, 99);
    let mut b = SnakeGame::new(8, 8, 99);
    for _ in 0..20 {
        assert_eq!(a.step(), b.step());
        assert_eq!(a.food(), b.food());
        assert_eq!(a.body().collect::<Vec<_>>(), b.body().collect::<Vec<_>>());
    }
}

#[test]
fn different_seeds_generally_diverge() {
    let a = SnakeGame::new(20, 20, 1);
    let b = SnakeGame::new(20, 20, 2);
    assert_ne!(a.food(), b.food());
}

// ── Movement ─────────────────────────────────────────────────────────────────

#[test]
fn stepping_moves_the_head_one_cell_in_the_current_direction() {
    let mut g = game();
    let before = g.head();
    assert_eq!(g.step(), StepOutcome::Moved);
    assert_eq!(g.head(), Coord::new(before.x + 1, before.y));
    assert_eq!(g.ticks(), 1);
}

#[test]
fn a_length_one_snake_does_not_grow_by_moving() {
    let mut g = game();
    g.step();
    assert_eq!(g.len(), 1);
}

#[test]
fn queued_turns_apply_on_the_next_step_not_immediately() {
    let mut g = game();
    assert!(g.queue_turn(Direction::Down));
    // Still travelling right until a step commits the turn.
    assert_eq!(g.direction(), Direction::Right);
    g.step();
    assert_eq!(g.direction(), Direction::Down);
}

#[test]
fn the_last_queued_turn_within_a_tick_is_the_one_that_applies() {
    let mut g = game();
    g.queue_turn(Direction::Up);
    g.queue_turn(Direction::Down);
    g.step();
    assert_eq!(g.direction(), Direction::Down);
}

// ── The reversal rule ────────────────────────────────────────────────────────

#[test]
fn a_length_one_snake_may_reverse() {
    let mut g = game();
    assert_eq!(g.len(), 1);
    assert!(g.queue_turn(Direction::Left));
}

#[test]
fn a_longer_snake_may_not_reverse_into_its_own_neck() {
    let mut g = SnakeGame::new(10, 10, 7);
    grow_to(&mut g, 3);
    let travelling = g.direction();
    assert!(!g.queue_turn(travelling.opposite()));
}

#[test]
fn two_turns_in_one_tick_cannot_smuggle_in_a_reversal() {
    // The bug this guards: if turns applied immediately, a player pressing two
    // keys inside one tick could chain two individually legal turns into a
    // reversal — e.g. travelling Right, press Up (legal), then Left (legal
    // relative to Up) and drive straight back into the neck.
    let mut g = SnakeGame::new(10, 10, 7);
    grow_to(&mut g, 3);

    let travelling = g.direction();
    let perpendicular = match travelling {
        Direction::Left | Direction::Right => Direction::Up,
        Direction::Up | Direction::Down => Direction::Left,
    };

    assert!(g.queue_turn(perpendicular), "a perpendicular turn is legal");
    // The second turn is validated against the direction actually travelled,
    // not against the one sitting in the queue, so the reversal is refused.
    assert!(
        !g.queue_turn(travelling.opposite()),
        "reversing must be refused even after another turn is queued"
    );

    g.step();
    assert_eq!(g.direction(), perpendicular);
    assert_ne!(g.direction(), travelling.opposite());
    assert_eq!(g.status(), GameStatus::Running);
}

#[test]
fn turning_is_rejected_once_the_game_is_over() {
    let mut g = game();
    run_into_wall(&mut g);
    assert!(!g.queue_turn(Direction::Up));
}

// ── Eating and growth ────────────────────────────────────────────────────────

/// Whether stepping `dir` would stay on the board and off the body.
///
/// Conservative: it counts the tail cell as occupied even though the tail
/// vacates as the head arrives. That costs the helper nothing and keeps it from
/// depending on the very rule some of these tests are checking.
fn is_safe(game: &SnakeGame, dir: Direction) -> bool {
    let head = game.head();
    let d = dir.delta();
    let next = Coord::new(head.x + d.x, head.y + d.y);
    game.contains(next) && !game.body().any(|c| c == next)
}

/// Picks a direction that closes the gap to `target`, if one is safe.
fn steer_toward(game: &SnakeGame, target: Coord) -> Option<Direction> {
    let head = game.head();
    let mut wanted = Vec::new();
    if target.x > head.x {
        wanted.push(Direction::Right);
    }
    if target.x < head.x {
        wanted.push(Direction::Left);
    }
    if target.y > head.y {
        wanted.push(Direction::Down);
    }
    if target.y < head.y {
        wanted.push(Direction::Up);
    }
    wanted.into_iter().find(|d| is_safe(game, *d))
}

/// Grows the snake to `target` segments by chasing the food greedily.
///
/// Several tests need a snake with an actual neck; this is the cheapest honest
/// way to get one without exposing the body for tests to construct directly.
fn grow_to(game: &mut SnakeGame, target: usize) {
    let mut guard = 0;
    while game.len() < target {
        guard += 1;
        assert!(guard < 10_000, "failed to grow the snake to {target}");

        let heading = game
            .food()
            .and_then(|food| steer_toward(game, food))
            // No safe move toward the food: take any safe move at all.
            .or_else(|| {
                [
                    Direction::Right,
                    Direction::Down,
                    Direction::Left,
                    Direction::Up,
                ]
                .into_iter()
                .find(|d| is_safe(game, *d))
            });
        if let Some(dir) = heading {
            game.queue_turn(dir);
        }

        match game.step() {
            StepOutcome::Died(cause) => panic!("snake died while growing: {cause:?}"),
            StepOutcome::Ended | StepOutcome::Won => panic!("game ended while growing"),
            _ => {}
        }
    }
}

#[test]
fn eating_grows_the_snake_and_scores() {
    // Search seeds for one whose first food is directly ahead of the snake.
    let mut g = None;
    for seed in 0..500u64 {
        let candidate = SnakeGame::new(10, 10, seed);
        let head = candidate.head();
        if candidate.food() == Some(Coord::new(head.x + 1, head.y)) {
            g = Some(candidate);
            break;
        }
    }
    let mut g = g.expect("some seed places food directly right of the head");

    let outcome = g.step();
    assert!(
        matches!(outcome, StepOutcome::Ate { score: 1, .. }),
        "expected to eat, got {outcome:?}"
    );
    assert_eq!(g.len(), 2);
    assert_eq!(g.score(), 1);
    assert!(g.food().is_some(), "new food must be placed after eating");
}

#[test]
fn new_food_never_lands_on_the_snake() {
    let mut g = SnakeGame::new(6, 6, 3);
    for _ in 0..400 {
        if g.is_over() {
            break;
        }
        g.step();
        if let Some(food) = g.food() {
            assert!(
                !g.body().any(|c| c == food),
                "food landed on the snake at {food:?}"
            );
        }
    }
}

// ── Death ────────────────────────────────────────────────────────────────────

#[test]
fn leaving_the_board_is_fatal() {
    let mut g = game();
    assert_eq!(
        run_into_wall(&mut g),
        StepOutcome::Died(DeathCause::HitWall)
    );
    assert_eq!(g.status(), GameStatus::Dead(DeathCause::HitWall));
    assert!(g.is_over());
}

#[test]
fn every_wall_is_fatal() {
    for dir in [
        Direction::Up,
        Direction::Down,
        Direction::Left,
        Direction::Right,
    ] {
        let mut g = SnakeGame::new(6, 6, 11);
        g.queue_turn(dir);
        let outcome = run_into_wall(&mut g);
        assert_eq!(outcome, StepOutcome::Died(DeathCause::HitWall), "{dir:?}");
    }
}

#[test]
fn stepping_after_death_changes_nothing() {
    let mut g = game();
    run_into_wall(&mut g);
    let body: Vec<_> = g.body().collect();
    let (score, ticks) = (g.score(), g.ticks());

    assert_eq!(g.step(), StepOutcome::Ended);
    assert_eq!(g.body().collect::<Vec<_>>(), body);
    assert_eq!(g.score(), score);
    assert_eq!(
        g.ticks(),
        ticks,
        "a finished game must not advance its clock"
    );
}

#[test]
fn the_snake_may_follow_its_own_vacating_tail() {
    // A snake in a tight loop chases the cell its tail is leaving this tick.
    // That is legal, and the classic off-by-one is to call it a collision.
    let mut g = SnakeGame::new(10, 10, 5);
    grow_to(&mut g, 4);
    let mut survived = 0;
    for dir in [
        Direction::Down,
        Direction::Left,
        Direction::Up,
        Direction::Right,
    ] {
        g.queue_turn(dir);
        if matches!(g.step(), StepOutcome::Died(DeathCause::HitSelf)) {
            panic!("following the vacating tail was treated as a collision");
        }
        survived += 1;
    }
    assert_eq!(survived, 4);
}

// ── Winning ──────────────────────────────────────────────────────────────────

#[test]
fn filling_the_smallest_board_wins() {
    // On a 2x2 board the snake needs only four segments.
    let mut g = SnakeGame::new(2, 2, 0);
    let mut outcome = StepOutcome::Moved;
    for _ in 0..64 {
        if g.is_over() {
            break;
        }
        // Walk the perimeter: right, down, left, up.
        let next = match g.direction() {
            Direction::Right => Direction::Down,
            Direction::Down => Direction::Left,
            Direction::Left => Direction::Up,
            Direction::Up => Direction::Right,
        };
        g.queue_turn(next);
        outcome = g.step();
    }
    // Either it filled the board or it died; both are legitimate on 2x2, but
    // if it won the bookkeeping must be consistent.
    if g.status() == GameStatus::Won {
        assert_eq!(outcome, StepOutcome::Won);
        assert_eq!(g.len(), 4);
        assert_eq!(g.food(), None, "a full board has nowhere for food");
    }
}

// ── Board queries ────────────────────────────────────────────────────────────

#[test]
fn contains_accepts_the_board_and_rejects_everything_else() {
    let g = SnakeGame::new(4, 3, 0);
    assert!(g.contains(Coord::new(0, 0)));
    assert!(g.contains(Coord::new(3, 2)));
    assert!(!g.contains(Coord::new(4, 0)));
    assert!(!g.contains(Coord::new(0, 3)));
    assert!(!g.contains(Coord::new(-1, 0)));
    assert!(!g.contains(Coord::new(0, -1)));
}

#[test]
fn dimensions_are_reported_back() {
    let g = SnakeGame::new(7, 5, 0);
    assert_eq!((g.width(), g.height()), (7, 5));
}

#[test]
fn the_snake_is_never_empty() {
    let mut g = game();
    assert!(!g.is_empty());
    run_into_wall(&mut g);
    assert!(!g.is_empty(), "even a dead snake still occupies cells");
}

#[test]
fn body_yields_the_head_first() {
    let mut g = SnakeGame::new(10, 10, 5);
    grow_to(&mut g, 3);
    assert_eq!(g.body().next(), Some(g.head()));
}

#[test]
fn reset_restores_a_fresh_game() {
    let mut g = game();
    run_into_wall(&mut g);
    g.reset(123);
    assert_eq!(g.status(), GameStatus::Running);
    assert_eq!(g.len(), 1);
    assert_eq!(g.score(), 0);
    assert_eq!(g.ticks(), 0);
    assert_eq!((g.width(), g.height()), (10, 10));
    assert!(g.food().is_some());
}

// ── Regressions found by mutation testing ────────────────────────────────────
//
// `cargo mutants` reported these four mutations as surviving the suite above.
// Each test below kills one; the comments say what the mutation was, because
// that is the part a future reader cannot reconstruct.

#[test]
fn a_length_one_snake_actually_reverses_when_stepped() {
    // Mutation: `self.body.len() == 1` -> `!= 1` in the turn-commit condition.
    // The old suite checked that `queue_turn` *accepted* the reversal but never
    // that `step` applied it, so flipping the comparison changed nothing.
    let mut g = game();
    assert_eq!(g.len(), 1);
    let start = g.direction();

    assert!(g.queue_turn(start.opposite()));
    g.step();
    assert_eq!(
        g.direction(),
        start.opposite(),
        "a length-one snake must be able to turn around"
    );
}

#[test]
fn a_fatal_self_collision_leaves_the_body_intact() {
    // Mutation: `if !eating` -> `if eating` on the path that restores the tail
    // after a self-collision. Nothing asserted the body was whole afterwards,
    // so the snake could lose a segment on death unnoticed.
    let mut g = SnakeGame::new(12, 12, 4);
    grow_to(&mut g, 5);

    // A snake of five segments driving round a single cell must catch itself.
    let mut length_before = g.len();
    let mut died = false;
    for dir in [
        Direction::Right,
        Direction::Down,
        Direction::Left,
        Direction::Up,
    ]
    .into_iter()
    .cycle()
    .take(24)
    {
        if g.is_over() {
            break;
        }
        length_before = g.len();
        g.queue_turn(dir);
        if g.step() == StepOutcome::Died(DeathCause::HitSelf) {
            died = true;
            break;
        }
    }

    assert!(died, "expected the snake to run into itself");
    assert_eq!(
        g.len(),
        length_before,
        "the body lost a segment when it died"
    );
}

#[test]
fn a_partly_filled_board_is_not_a_win() {
    // Mutation: `self.width * self.height` -> `+` in the win condition. The
    // only win test used a 2x2 board, where 2*2 and 2+2 are both 4 — so the
    // mutation was invisible. On 10x10 the two differ (100 vs 20).
    let mut g = SnakeGame::new(10, 10, 8);
    grow_to(&mut g, 20);

    assert_eq!(g.len(), 20);
    assert_eq!(
        g.status(),
        GameStatus::Running,
        "a snake of width+height segments must not count as filling the board"
    );
    assert!(g.food().is_some(), "an unfinished board still has food");
}

#[test]
fn a_negative_delta_never_rewinds_the_ticker() {
    // Mutation: `delta > 0.0` -> `>= 0.0`, which the old code could not
    // distinguish. The branch is gone now; this pins the behaviour that
    // replaced it.
    let mut t = Ticker::new(10.0);
    t.accumulate(0.05);
    let progress = t.alpha();

    t.accumulate(-10.0);
    assert!(
        (t.alpha() - progress).abs() < 1e-6,
        "a negative delta moved the accumulator"
    );
    // And the ticker still fires on schedule afterwards.
    assert_eq!(t.accumulate(0.05), 1);
}

#[test]
fn ticker_keeps_the_remainder_when_exactly_at_the_cap() {
    // Mutation: `due > MAX` -> `>=`, which would discard the remainder on a
    // frame that lands exactly on the cap rather than one that exceeds it.
    let mut t = Ticker::new(10.0); // 100_000 us per step
    let exactly_max = Ticker::MAX_STEPS_PER_CALL as u64 * 100_000;

    assert_eq!(t.accumulate_micros(exactly_max + 40_000), 8);
    // Not over the cap, so the leftover 40 ms is still owed.
    assert!((t.alpha() - 0.4).abs() < 1e-4, "alpha was {}", t.alpha());
}

#[test]
fn ticker_discards_the_backlog_only_when_over_the_cap() {
    let mut t = Ticker::new(10.0);
    // Twenty steps' worth: over the cap, so the surplus is dropped entirely.
    assert_eq!(t.accumulate_micros(2_000_000), Ticker::MAX_STEPS_PER_CALL);
    assert_eq!(t.alpha(), 0.0);
    assert_eq!(t.accumulate_micros(0), 0);
}

#[test]
fn an_infinite_delta_is_absorbed_rather_than_exploding() {
    // `seconds_to_micros` saturates +inf to u64::MAX; the cap must contain it.
    let mut t = Ticker::new(10.0);
    assert_eq!(t.accumulate(f32::INFINITY), Ticker::MAX_STEPS_PER_CALL);
    assert_eq!(t.alpha(), 0.0);
}

// ── no_std ───────────────────────────────────────────────────────────────────

#[test]
fn rounding_matches_the_std_implementation_it_replaced() {
    // `f32::round` is a libm intrinsic and so unavailable under no_std. The
    // replacement must be exactly equivalent over the domain callers use, or
    // the ticker's timing changes on a target nobody tests interactively.
    //
    // Tests build with std, so the original is right here to compare against.
    let cases = [
        0.0f32,
        0.4,
        0.5,
        0.6,
        1.0,
        1.4999,
        1.5,
        2.5,
        999.49,
        999.5,
        16_666.6,
        1_000_000.0,
        33_333.33,
        8_333.0,
    ];
    for value in cases {
        assert_eq!(
            round_to_u64(value),
            value.round() as u64,
            "disagreed on {value}"
        );
    }
    // And the saturating behaviour the callers document.
    assert_eq!(round_to_u64(f32::NAN), 0);
    assert_eq!(round_to_u64(-1.0), 0);
    assert_eq!(round_to_u64(f32::NEG_INFINITY), 0);
    assert_eq!(round_to_u64(f32::INFINITY), u64::MAX);
}
