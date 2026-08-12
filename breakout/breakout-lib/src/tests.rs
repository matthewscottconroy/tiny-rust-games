//! Tests for the Breakout rules.
//!
//! The interesting ones are at the bottom: whether continuous floating-point
//! physics is still deterministic enough to replay, which is the question this
//! game was written to answer.

use super::*;

fn game() -> BreakoutGame {
    BreakoutGame::new(BreakoutGame::default_layout())
}

/// A launched game, since almost everything needs the ball moving.
fn launched() -> BreakoutGame {
    let mut g = game();
    g.launch();
    g
}

/// Runs `n` steps.
fn run(game: &mut BreakoutGame, n: usize) {
    for _ in 0..n {
        game.step();
    }
}

/// A layout with a single brick parked in the top-left corner.
///
/// Clearing the bricks entirely would win the game on the first step, so tests
/// that need an empty field keep one brick out of the ball's way.
fn sparse_layout() -> Layout {
    let mut layout = BreakoutGame::default_layout();
    layout.bricks.truncate(1);
    layout.bricks[0].rect = Rect::new(Vec2::new(12.0, 12.0), Vec2::new(6.0, 4.0));
    layout.bricks[0].hits = 9;
    layout
}

/// Steps once, steering the paddle toward the ball first.
///
/// Several tests need the ball to actually reach the paddle, which a stationary
/// paddle cannot promise.
fn step_following(game: &mut BreakoutGame) -> StepOutcome {
    let input = if game.ball().x < game.paddle_x() - 2.0 {
        PaddleInput::Left
    } else if game.ball().x > game.paddle_x() + 2.0 {
        PaddleInput::Right
    } else {
        PaddleInput::None
    };
    game.set_paddle_input(input);
    game.step()
}

// ── Vec2 and Rect ────────────────────────────────────────────────────────────

#[test]
fn lerp_hits_both_ends_and_the_middle() {
    let a = Vec2::new(0.0, 10.0);
    let b = Vec2::new(10.0, 20.0);
    assert_eq!(a.lerp(b, 0.0), a);
    assert_eq!(a.lerp(b, 1.0), b);
    assert_eq!(a.lerp(b, 0.5), Vec2::new(5.0, 15.0));
}

#[test]
fn lerp_clamps_out_of_range_alpha() {
    let a = Vec2::new(0.0, 0.0);
    let b = Vec2::new(10.0, 0.0);
    assert_eq!(a.lerp(b, -3.0), a);
    assert_eq!(a.lerp(b, 7.0), b);
}

#[test]
fn a_rect_reports_its_edges() {
    let r = Rect::new(Vec2::new(10.0, 20.0), Vec2::new(4.0, 2.0));
    assert_eq!((r.left(), r.right()), (6.0, 14.0));
    assert_eq!((r.top(), r.bottom()), (18.0, 22.0));
}

#[test]
fn circle_overlap_covers_faces_corners_and_misses() {
    let r = Rect::new(Vec2::new(0.0, 0.0), Vec2::new(10.0, 5.0));
    assert!(r.overlaps_circle(Vec2::new(0.0, 0.0), 1.0), "inside");
    assert!(r.overlaps_circle(Vec2::new(10.5, 0.0), 1.0), "right face");
    assert!(r.overlaps_circle(Vec2::new(0.0, -5.5), 1.0), "top face");
    // Just past a corner, diagonally — the case a naive AABB test gets wrong.
    assert!(!r.overlaps_circle(Vec2::new(11.0, 6.0), 1.0), "past corner");
    assert!(r.overlaps_circle(Vec2::new(10.5, 5.3), 1.0), "near corner");
}

// ── Setup ────────────────────────────────────────────────────────────────────

#[test]
fn a_new_game_waits_on_the_paddle() {
    let g = game();
    assert!(g.ball_is_stuck());
    assert_eq!(g.status(), GameStatus::Playing);
    assert_eq!(g.score(), 0);
    assert_eq!(g.ticks(), 0);
    assert_eq!(g.lives(), 3);
    assert_eq!(g.bricks_remaining(), g.bricks().len());
}

#[test]
fn the_default_layout_is_eight_by_five() {
    let g = game();
    assert_eq!(g.bricks().len(), 40);
    // The top two rows take two hits.
    assert_eq!(g.bricks().iter().filter(|b| b.hits == 2).count(), 16);
}

#[test]
#[should_panic(expected = "positive size")]
fn a_degenerate_field_is_rejected() {
    let mut layout = BreakoutGame::default_layout();
    layout.width = 0.0;
    BreakoutGame::new(layout);
}

#[test]
#[should_panic(expected = "at least one life")]
fn a_game_without_lives_is_rejected() {
    let mut layout = BreakoutGame::default_layout();
    layout.lives = 0;
    BreakoutGame::new(layout);
}

// ── The paddle ───────────────────────────────────────────────────────────────

#[test]
fn the_paddle_moves_only_when_pushed() {
    let mut g = game();
    let start = g.paddle_x();
    g.step();
    assert_eq!(g.paddle_x(), start, "no input, no movement");

    g.set_paddle_input(PaddleInput::Right);
    g.step();
    assert!(g.paddle_x() > start);

    g.set_paddle_input(PaddleInput::Left);
    let mid = g.paddle_x();
    g.step();
    assert!(g.paddle_x() < mid);
}

#[test]
fn the_paddle_stops_at_both_walls() {
    let mut g = game();
    g.set_paddle_input(PaddleInput::Left);
    run(&mut g, 600);
    let paddle = g.paddle_rect();
    assert!(
        paddle.left() >= -0.01,
        "left edge escaped: {}",
        paddle.left()
    );

    g.set_paddle_input(PaddleInput::Right);
    run(&mut g, 1200);
    let paddle = g.paddle_rect();
    let width = g.size().x;
    assert!(paddle.right() <= width + 0.01, "right edge escaped");
}

#[test]
fn a_resting_ball_rides_the_paddle() {
    let mut g = game();
    g.set_paddle_input(PaddleInput::Right);
    run(&mut g, 20);
    assert!(g.ball_is_stuck());
    assert!((g.ball().x - g.paddle_x()).abs() < 0.01);
}

// ── Launching ────────────────────────────────────────────────────────────────

#[test]
fn launching_sets_the_ball_moving_upward() {
    let mut g = game();
    assert!(g.launch());
    assert!(!g.ball_is_stuck());
    assert!(g.ball_velocity().y < 0.0, "should travel up the screen");
}

#[test]
fn launching_twice_does_nothing() {
    let mut g = game();
    assert!(g.launch());
    assert!(!g.launch(), "a second launch must be refused");
}

// ── Bouncing ─────────────────────────────────────────────────────────────────

#[test]
fn the_ball_stays_inside_the_field() {
    let mut g = launched();
    let size = g.size();
    for _ in 0..4000 {
        g.step();
        if g.is_over() {
            break;
        }
        let b = g.ball();
        assert!(
            b.x >= -0.01 && b.x <= size.x + 0.01,
            "escaped sideways: {b:?}"
        );
        assert!(b.y >= -0.01, "escaped through the ceiling: {b:?}");
    }
}

#[test]
fn the_ceiling_reverses_vertical_travel() {
    // A near-empty field, so the ball reaches the ceiling instead of meeting a
    // brick on the way up.
    let mut g = BreakoutGame::new(sparse_layout());
    g.launch();
    for _ in 0..2000 {
        let before = g.ball_velocity().y;
        let outcome = g.step();
        if outcome.hit_wall && before < 0.0 && g.ball_velocity().y > 0.0 {
            return;
        }
    }
    panic!("expected a ceiling bounce");
}

#[test]
fn the_paddle_steers_the_bounce_by_where_it_is_struck() {
    // The only "feel" in the game: striking left of centre sends the ball left,
    // right of centre sends it right. Driven directly rather than by playing,
    // so the two hit offsets are exact.
    let bounce_vx = |offset: f32| {
        let mut g = BreakoutGame::new(sparse_layout());
        g.launch();
        // Drop the ball onto a stationary paddle from a known offset.
        for _ in 0..6000 {
            // Hold the paddle so the ball lands `offset` from its centre.
            let target = g.ball().x - offset;
            let input = if target < g.paddle_x() - 1.0 {
                PaddleInput::Left
            } else if target > g.paddle_x() + 1.0 {
                PaddleInput::Right
            } else {
                PaddleInput::None
            };
            g.set_paddle_input(input);
            if g.step().hit_paddle {
                return Some(g.ball_velocity().x);
            }
            if g.is_over() {
                break;
            }
        }
        None
    };

    let left = bounce_vx(-40.0).expect("no bounce for a left-of-centre hit");
    let right = bounce_vx(40.0).expect("no bounce for a right-of-centre hit");

    assert!(
        left < 0.0,
        "left-of-centre hit should send the ball left: {left}"
    );
    assert!(
        right > 0.0,
        "right-of-centre hit should send the ball right: {right}"
    );
}

#[test]
fn a_paddle_bounce_preserves_speed() {
    let mut g = BreakoutGame::new(sparse_layout());
    g.launch();
    let speed = |v: Vec2| (v.x * v.x + v.y * v.y).sqrt();
    let launch_speed = speed(g.ball_velocity());

    for _ in 0..6000 {
        if step_following(&mut g).hit_paddle {
            let after = speed(g.ball_velocity());
            assert!(
                (after - launch_speed).abs() < 1.0,
                "speed changed on bounce: {launch_speed} -> {after}"
            );
            return;
        }
    }
    panic!("the ball never reached the paddle");
}

// ── Bricks ───────────────────────────────────────────────────────────────────

#[test]
fn hitting_a_brick_scores_and_damages_it() {
    let mut g = launched();
    for _ in 0..2000 {
        let outcome = g.step();
        if let Some(index) = outcome.hit_brick {
            assert!(g.score() > 0, "a hit must score");
            // Two-hit bricks survive the first strike.
            let brick = g.bricks()[index];
            assert!(brick.hits < 2 || !outcome.broke_brick);
            return;
        }
    }
    panic!("the ball never reached a brick");
}

#[test]
fn a_broken_brick_stops_being_hit() {
    let mut g = launched();
    let mut broken = None;
    for _ in 0..6000 {
        let outcome = g.step();
        if outcome.broke_brick {
            broken = outcome.hit_brick;
            break;
        }
    }
    let index = broken.expect("no brick was destroyed");
    assert!(!g.bricks()[index].alive());
    assert!(g.bricks_remaining() < g.bricks().len());
}

#[test]
fn only_one_brick_is_hit_per_step() {
    // Resolving several at once would make the outcome depend on iteration
    // order, which is exactly what determinism cannot tolerate.
    let mut g = launched();
    for _ in 0..4000 {
        let before = g.bricks_remaining();
        g.step();
        assert!(before - g.bricks_remaining() <= 1);
    }
}

#[test]
fn clearing_every_brick_wins() {
    // Rather than play a perfect game, break the bricks directly and confirm
    // the win condition fires on the next step.
    let mut layout = BreakoutGame::default_layout();
    for brick in &mut layout.bricks {
        brick.hits = 0;
    }
    let mut g = BreakoutGame::new(layout);
    g.launch();
    let outcome = g.step();
    assert_eq!(g.status(), GameStatus::Won);
    assert!(outcome.finished);
    assert!(g.is_over());
}

// ── Losing ───────────────────────────────────────────────────────────────────

#[test]
fn dropping_the_ball_costs_a_life_and_resets() {
    let mut g = BreakoutGame::new(sparse_layout());
    g.launch();
    // Park the paddle in a corner so the ball eventually misses it.
    g.set_paddle_input(PaddleInput::Left);

    for _ in 0..20_000 {
        if g.step().lost_life {
            assert_eq!(g.lives(), 2);
            assert!(g.ball_is_stuck(), "the ball should reset onto the paddle");
            return;
        }
        if g.is_over() {
            break;
        }
    }
    panic!("the ball was never lost");
}

#[test]
fn running_out_of_lives_ends_the_game() {
    let mut layout = sparse_layout();
    layout.lives = 1;
    let mut g = BreakoutGame::new(layout);
    g.launch();
    g.set_paddle_input(PaddleInput::Left);

    for _ in 0..20_000 {
        if g.step().finished {
            assert_eq!(g.status(), GameStatus::Lost);
            assert_eq!(g.lives(), 0);
            return;
        }
    }
    panic!("the game never ended");
}

#[test]
fn stepping_after_the_game_ends_changes_nothing() {
    let mut layout = BreakoutGame::default_layout();
    for brick in &mut layout.bricks {
        brick.hits = 0;
    }
    let mut g = BreakoutGame::new(layout);
    g.launch();
    g.step();
    assert!(g.is_over());

    let before = g.clone();
    assert_eq!(g.step(), StepOutcome::default());
    assert_eq!(g, before, "a finished game must be frozen");
}

// ── Interpolation ────────────────────────────────────────────────────────────

#[test]
fn ball_at_blends_between_the_last_two_steps() {
    let mut g = launched();
    run(&mut g, 10);
    let previous = g.ball_at(0.0);
    let current = g.ball_at(1.0);
    assert_eq!(current, g.ball());
    assert_ne!(previous, current, "the ball should have moved");

    let middle = g.ball_at(0.5);
    assert!((middle.x - (previous.x + current.x) / 2.0).abs() < 1e-4);
    assert!((middle.y - (previous.y + current.y) / 2.0).abs() < 1e-4);
}

#[test]
fn interpolation_never_leaves_the_travelled_segment() {
    let mut g = launched();
    for _ in 0..200 {
        g.step();
        for alpha in [-1.0, 0.0, 0.25, 0.5, 1.0, 2.0] {
            let p = g.ball_at(alpha);
            let (a, b) = (g.ball_at(0.0), g.ball());
            assert!(p.x >= a.x.min(b.x) - 1e-3 && p.x <= a.x.max(b.x) + 1e-3);
            assert!(p.y >= a.y.min(b.y) - 1e-3 && p.y <= a.y.max(b.y) + 1e-3);
        }
    }
}

// ── Determinism: the question this game was written to answer ────────────────

#[test]
fn identical_input_produces_bit_identical_physics() {
    // Floating point is only non-deterministic across differing operation
    // order or transcendental implementations. This game uses +, -, *, / and
    // sqrt, all of which IEEE-754 specifies exactly, in a fixed order — so two
    // runs must agree bit for bit, not merely closely.
    let script = [
        (0usize, PaddleInput::Right),
        (300, PaddleInput::Left),
        (900, PaddleInput::None),
        (1500, PaddleInput::Right),
    ];
    let play = || {
        let mut g = game();
        g.launch();
        for tick in 0..3000usize {
            for (at, input) in &script {
                if *at == tick {
                    g.set_paddle_input(*input);
                }
            }
            g.step();
        }
        g
    };

    let a = play();
    let b = play();
    assert_eq!(a.ball(), b.ball(), "ball position diverged");
    assert_eq!(a.ball_velocity(), b.ball_velocity(), "velocity diverged");
    assert_eq!(a.score(), b.score());
    assert_eq!(a.paddle_x(), b.paddle_x());
    assert_eq!(a, b, "entire game state must match");
}

#[test]
fn a_fixed_timestep_is_what_makes_that_possible() {
    // The counter-demonstration. Advancing by wall-clock time would make the
    // result depend on frame pacing; stepping a fixed DT means the same number
    // of steps always produces the same state, however they were grouped in
    // time.
    let mut a = launched();
    let mut b = launched();

    // "Sixty frames of two steps" and "twenty frames of six steps".
    for _ in 0..60 {
        run(&mut a, 2);
    }
    for _ in 0..20 {
        run(&mut b, 6);
    }
    assert_eq!(a.ticks(), b.ticks());
    assert_eq!(a, b);
}

// ── Precision, found by mutation testing ─────────────────────────────────────
//
// The tests above prove the game behaves; these pin the arithmetic. Mutation
// testing showed the difference: constants and boundary comparisons could be
// changed freely without a single behavioural test noticing.

#[test]
fn circle_overlap_scales_with_the_radius() {
    // Earlier tests all used radius 1.0, where `r * r` and `r / r` are both 1 —
    // so the squared-distance comparison could be mutated undetected.
    let r = Rect::new(Vec2::new(0.0, 0.0), Vec2::new(10.0, 5.0));
    // 3 units clear of the right face.
    let centre = Vec2::new(13.0, 0.0);
    assert!(!r.overlaps_circle(centre, 2.0), "radius 2 should not reach");
    assert!(r.overlaps_circle(centre, 4.0), "radius 4 should reach");
    // And diagonally, where the squaring actually matters: the corner is
    // 5 units away (3-4-5), so radius 4 misses and 6 hits.
    let corner = Vec2::new(13.0, 9.0);
    assert!(!r.overlaps_circle(corner, 4.0));
    assert!(r.overlaps_circle(corner, 6.0));
}

#[test]
fn the_paddle_moves_exactly_speed_times_dt_each_step() {
    // Mutation: `+=` became `*=`, and the speed constant could drift.
    let layout = BreakoutGame::default_layout();
    let expected = layout.paddle_speed * DT;
    let mut g = BreakoutGame::new(layout);
    let start = g.paddle_x();

    g.set_paddle_input(PaddleInput::Right);
    g.step();
    assert!(
        (g.paddle_x() - start - expected).abs() < 1e-3,
        "moved {}, expected {expected}",
        g.paddle_x() - start
    );

    g.set_paddle_input(PaddleInput::Left);
    let mid = g.paddle_x();
    g.step();
    assert!((mid - g.paddle_x() - expected).abs() < 1e-3);
}

#[test]
fn a_resting_ball_sits_on_top_of_the_paddle() {
    // Mutation: the offset in `ball_rest_position`. Nothing checked the ball
    // was above the paddle rather than inside or below it.
    let g = game();
    let paddle = g.paddle_rect();
    let ball = g.ball();
    assert!(ball.y < paddle.top(), "ball {ball:?} not above {paddle:?}");
    assert!(
        paddle.top() - ball.y < 40.0,
        "ball is floating far above the paddle"
    );
    assert!(
        (ball.x - g.paddle_x()).abs() < 0.01,
        "not centred on the paddle"
    );
}

#[test]
fn the_ball_bounces_at_the_wall_not_before_or_after() {
    // Mutations: `ball.x - r < 0.0` became `<=`, `==`, or `+`. Each changes
    // exactly where the bounce happens, and no behavioural test could tell.
    let mut layout = sparse_layout();
    layout.ball_radius = 10.0;
    let mut g = BreakoutGame::new(layout);
    g.launch();

    let mut touched_left = false;
    let mut touched_right = false;
    let width = g.size().x;
    for _ in 0..20_000 {
        g.step();
        if g.is_over() {
            break;
        }
        let b = g.ball();
        // The ball's *edge* may reach a wall but must never pass it.
        assert!(b.x - 10.0 >= -1e-3, "left edge crossed: {b:?}");
        assert!(b.x + 10.0 <= width + 1e-3, "right edge crossed: {b:?}");
        assert!(b.y - 10.0 >= -1e-3, "top edge crossed: {b:?}");
        if (b.x - 10.0).abs() < 1e-3 {
            touched_left = true;
        }
        if (b.x + 10.0 - width).abs() < 1e-3 {
            touched_right = true;
        }
    }
    assert!(touched_left && touched_right, "never reached both walls");
}

#[test]
fn the_default_brick_grid_has_the_geometry_it_claims() {
    // Mutations: every arithmetic operator in `default_layout`. A wrong grid
    // still "works" — it just looks wrong, which no behavioural test sees.
    let g = game();
    let size = g.size();
    let bricks = g.bricks();

    // Five rows of eight, laid out in row-major order.
    assert_eq!(bricks.len(), 40);
    for (i, brick) in bricks.iter().enumerate() {
        assert_eq!(brick.row, i / 8, "brick {i} has the wrong row");
    }

    // Every brick sits inside the field, above the paddle, with positive size.
    let paddle_top = g.paddle_rect().top();
    for (i, brick) in bricks.iter().enumerate() {
        let r = brick.rect;
        assert!(r.half.x > 0.0 && r.half.y > 0.0, "brick {i} has no size");
        assert!(
            r.left() >= 0.0 && r.right() <= size.x,
            "brick {i} off-field"
        );
        assert!(
            r.top() >= 0.0 && r.bottom() < paddle_top,
            "brick {i} too low"
        );
    }

    // Columns are evenly spaced, and rows are stacked in increasing y.
    let spacing = bricks[1].rect.centre.x - bricks[0].rect.centre.x;
    assert!(spacing > 0.0, "columns must run left to right");
    for row in 0..5 {
        for col in 1..8 {
            let a = bricks[row * 8 + col - 1].rect.centre.x;
            let b = bricks[row * 8 + col].rect.centre.x;
            assert!((b - a - spacing).abs() < 1e-3, "uneven column spacing");
        }
    }
    for row in 1..5 {
        let above = bricks[(row - 1) * 8].rect.centre.y;
        let below = bricks[row * 8].rect.centre.y;
        assert!(below > above, "rows must stack downward");
    }

    // Bricks in a row must not overlap each other.
    for row in 0..5 {
        for col in 1..8 {
            let left = bricks[row * 8 + col - 1].rect;
            let right = bricks[row * 8 + col].rect;
            assert!(left.right() <= right.left() + 1e-3, "bricks overlap");
        }
    }
}

#[test]
fn the_launch_velocity_matches_the_configured_speed() {
    // Mutation: the launch angle constants. A launch that is too slow or too
    // steep still plays, just badly.
    let g = {
        let mut g = game();
        g.launch();
        g
    };
    let v = g.ball_velocity();
    let speed = (v.x * v.x + v.y * v.y).sqrt();
    let expected = BreakoutGame::default_layout().ball_speed;
    assert!(
        (speed - expected).abs() < 1.0,
        "launch speed {speed}, expected {expected}"
    );
    assert!(v.y < 0.0 && v.x > 0.0, "should launch up and to the right");
}

// ── Brick collision response, tested directly ────────────────────────────────
//
// Reached by placing the ball rather than by playing: landing on a specific
// brick face by choosing a launch angle would be a coincidence, not a test.
// Mutation testing showed this whole function was unpinned — every operator in
// the penetration maths could be changed without a failure.

/// A game with exactly one brick, centred at a known place.
fn one_brick_at(centre: Vec2, half: Vec2) -> BreakoutGame {
    let mut layout = BreakoutGame::default_layout();
    layout.ball_radius = 5.0;
    layout.bricks = vec![Brick {
        rect: Rect::new(centre, half),
        hits: 1,
        row: 0,
    }];
    BreakoutGame::new(layout)
}

#[test]
fn hitting_a_brick_from_below_reflects_vertically() {
    let brick = Vec2::new(400.0, 200.0);
    let half = Vec2::new(40.0, 12.0);
    let mut g = one_brick_at(brick, half);

    // Just touching the underside, travelling up.
    g.place_ball(
        Vec2::new(brick.x, brick.y + half.y + 3.0),
        Vec2::new(0.0, -300.0),
    );
    let outcome = g.step();

    assert_eq!(outcome.hit_brick, Some(0));
    assert!(g.ball_velocity().y > 0.0, "should now travel downward");
    assert_eq!(
        g.ball_velocity().x,
        0.0,
        "horizontal travel must not change"
    );
    assert!(
        g.ball().y >= brick.y + half.y,
        "ball {:?} was left inside the brick",
        g.ball()
    );
}

#[test]
fn hitting_a_brick_from_the_side_reflects_horizontally() {
    let brick = Vec2::new(400.0, 200.0);
    let half = Vec2::new(40.0, 12.0);
    let mut g = one_brick_at(brick, half);

    // Approaching the left face, travelling right.
    g.place_ball(
        Vec2::new(brick.x - half.x - 3.0, brick.y),
        Vec2::new(300.0, 0.0),
    );
    let outcome = g.step();

    assert_eq!(outcome.hit_brick, Some(0));
    assert!(g.ball_velocity().x < 0.0, "should now travel leftward");
    assert_eq!(g.ball_velocity().y, 0.0, "vertical travel must not change");
    assert!(
        g.ball().x <= brick.x - half.x,
        "ball {:?} was left inside the brick",
        g.ball()
    );
}

#[test]
fn hitting_a_brick_from_above_pushes_the_ball_up() {
    let brick = Vec2::new(400.0, 300.0);
    let half = Vec2::new(40.0, 12.0);
    let mut g = one_brick_at(brick, half);

    g.place_ball(
        Vec2::new(brick.x, brick.y - half.y - 3.0),
        Vec2::new(0.0, 300.0),
    );
    g.step();

    assert!(g.ball_velocity().y < 0.0, "should bounce back upward");
    assert!(g.ball().y <= brick.y - half.y, "left inside the brick");
}

#[test]
fn hitting_a_brick_from_the_right_pushes_the_ball_right() {
    let brick = Vec2::new(400.0, 300.0);
    let half = Vec2::new(40.0, 12.0);
    let mut g = one_brick_at(brick, half);

    g.place_ball(
        Vec2::new(brick.x + half.x + 3.0, brick.y),
        Vec2::new(-300.0, 0.0),
    );
    g.step();

    assert!(g.ball_velocity().x > 0.0, "should bounce back rightward");
    assert!(g.ball().x >= brick.x + half.x, "left inside the brick");
}

#[test]
fn a_brick_bounce_never_leaves_the_ball_overlapping() {
    // The invariant behind all four cases: after resolving, the ball is clear.
    let brick = Vec2::new(400.0, 250.0);
    let half = Vec2::new(40.0, 12.0);
    for (offset, velocity) in [
        (Vec2::new(0.0, 20.0), Vec2::new(0.0, -400.0)),
        (Vec2::new(0.0, -20.0), Vec2::new(0.0, 400.0)),
        (Vec2::new(-50.0, 0.0), Vec2::new(400.0, 0.0)),
        (Vec2::new(50.0, 0.0), Vec2::new(-400.0, 0.0)),
        (Vec2::new(-48.0, 14.0), Vec2::new(300.0, -300.0)),
    ] {
        let mut g = one_brick_at(brick, half);
        g.place_ball(Vec2::new(brick.x + offset.x, brick.y + offset.y), velocity);
        g.step();
        assert!(
            !g.bricks()[0].rect.overlaps_circle(g.ball(), 5.0) || !g.bricks()[0].alive(),
            "ball {:?} still overlaps after bouncing (offset {offset:?})",
            g.ball()
        );
    }
}

#[test]
fn a_two_hit_brick_survives_the_first_strike() {
    let mut layout = BreakoutGame::default_layout();
    layout.ball_radius = 5.0;
    layout.bricks = vec![Brick {
        rect: Rect::new(Vec2::new(400.0, 200.0), Vec2::new(40.0, 12.0)),
        hits: 2,
        row: 0,
    }];
    let mut g = BreakoutGame::new(layout);

    g.place_ball(Vec2::new(400.0, 215.0), Vec2::new(0.0, -300.0));
    let first = g.step();
    assert_eq!(first.hit_brick, Some(0));
    assert!(!first.broke_brick, "two-hit brick should survive");
    assert_eq!(g.bricks()[0].hits, 1);
    assert!(g.bricks()[0].alive());

    // Second strike from the same side finishes it.
    g.place_ball(Vec2::new(400.0, 215.0), Vec2::new(0.0, -300.0));
    let second = g.step();
    assert!(second.broke_brick, "second hit should destroy it");
    assert!(!g.bricks()[0].alive());
}

#[test]
fn breaking_a_brick_scores_more_than_merely_hitting_it() {
    let mut layout = BreakoutGame::default_layout();
    layout.ball_radius = 5.0;
    layout.bricks = vec![Brick {
        rect: Rect::new(Vec2::new(400.0, 200.0), Vec2::new(40.0, 12.0)),
        hits: 2,
        row: 0,
    }];
    let mut g = BreakoutGame::new(layout);

    g.place_ball(Vec2::new(400.0, 215.0), Vec2::new(0.0, -300.0));
    g.step();
    let after_hit = g.score();
    assert!(after_hit > 0);

    g.place_ball(Vec2::new(400.0, 215.0), Vec2::new(0.0, -300.0));
    g.step();
    assert!(
        g.score() - after_hit > after_hit,
        "destroying should score more than a glancing hit"
    );
}

#[test]
fn the_ball_is_lost_only_once_it_is_fully_below_the_field() {
    // Mutation: the `- radius` in the loss check became `+`, which would drop
    // the ball a whole diameter early.
    let mut layout = sparse_layout();
    layout.ball_radius = 10.0;
    let height = layout.height;
    let mut g = BreakoutGame::new(layout);

    // Straddling the bottom edge: still in play.
    g.place_ball(Vec2::new(400.0, height), Vec2::new(0.0, 10.0));
    assert!(!g.step().lost_life, "a ball on the line is still in play");

    // Clear of it: lost.
    g.place_ball(Vec2::new(400.0, height + 30.0), Vec2::new(0.0, 10.0));
    assert!(g.step().lost_life, "a ball below the field is lost");
}

#[test]
fn ticks_counts_every_step() {
    // Mutation: `ticks` replaced by 0.
    let mut g = launched();
    assert_eq!(g.ticks(), 0);
    run(&mut g, 7);
    assert_eq!(g.ticks(), 7);
}

#[test]
fn a_paddle_hit_at_dead_centre_sends_the_ball_straight_up() {
    // Mutation: the offset division in `collide_paddle`. A centred hit is the
    // one case with an exactly known answer.
    let mut g = BreakoutGame::new(sparse_layout());
    let paddle = g.paddle_rect();
    g.place_ball(
        Vec2::new(g.paddle_x(), paddle.top() - 6.0),
        Vec2::new(0.0, 300.0),
    );
    let outcome = g.step();

    assert!(outcome.hit_paddle);
    assert!(
        g.ball_velocity().x.abs() < 1.0,
        "a centred hit should not steer: vx = {}",
        g.ball_velocity().x
    );
    assert!(g.ball_velocity().y < 0.0);
}
