//! Property-based tests for the Breakout rules.
//!
//! The unit tests in `src/tests.rs` pin specific scenarios. These assert
//! invariants that must hold for *every* starting layout and input sequence,
//! which is where continuous physics differs in kind from a grid game: Snake
//! can only be in one of finitely many states, so examples cover it well, while
//! a ball has a real-valued position and the interesting failures live in the
//! gaps between the cases anyone thinks to write down.

use breakout_lib::{BreakoutGame, GameStatus, Layout, PaddleInput, Vec2};
use proptest::prelude::*;

/// An arbitrary paddle input.
fn any_input() -> impl Strategy<Value = PaddleInput> {
    prop_oneof![
        Just(PaddleInput::Left),
        Just(PaddleInput::Right),
        Just(PaddleInput::None),
    ]
}

/// The default layout with the ball speed and paddle geometry varied.
///
/// The speed range stops below the tunnelling threshold that
/// `the_ball_cannot_cross_a_brick_in_one_step` pins down; above it the ball is
/// *supposed* to pass through bricks, so sampling there would test nothing.
fn any_layout() -> impl Strategy<Value = Layout> {
    (60.0f32..1200.0, 20.0f32..120.0, 3.0f32..12.0).prop_map(|(speed, half_w, radius)| {
        let mut layout = BreakoutGame::default_layout();
        layout.ball_speed = speed;
        layout.paddle_half_width = half_w;
        layout.ball_radius = radius;
        layout
    })
}

/// Plays `inputs`, applying each and stepping once.
fn play(game: &mut BreakoutGame, inputs: &[PaddleInput]) {
    for input in inputs {
        game.set_paddle_input(*input);
        game.step();
    }
}

proptest! {
    /// No layout or input sequence may panic.
    #[test]
    fn stepping_never_panics(
        layout in any_layout(),
        launch: bool,
        inputs in prop::collection::vec(any_input(), 0..400),
    ) {
        let mut game = BreakoutGame::new(layout);
        if launch {
            game.launch();
        }
        play(&mut game, &inputs);
    }

    /// The paddle never leaves the field, however long it is pushed.
    #[test]
    fn the_paddle_stays_within_the_field(
        layout in any_layout(),
        inputs in prop::collection::vec(any_input(), 0..400),
    ) {
        let width = layout.width;
        let mut game = BreakoutGame::new(layout);
        game.launch();
        for input in inputs {
            game.set_paddle_input(input);
            game.step();
            let paddle = game.paddle_rect();
            prop_assert!(paddle.left() >= -0.01, "left edge escaped: {}", paddle.left());
            prop_assert!(paddle.right() <= width + 0.01, "right edge escaped: {}", paddle.right());
        }
    }

    /// A ball still in play stays inside the walls and under the ceiling.
    ///
    /// The floor is deliberately excluded: falling past it is how a life is
    /// lost, and the game handles that rather than preventing it.
    #[test]
    fn a_live_ball_never_escapes_sideways_or_upward(
        layout in any_layout(),
        inputs in prop::collection::vec(any_input(), 0..400),
    ) {
        let size = Vec2::new(layout.width, layout.height);
        let mut game = BreakoutGame::new(layout);
        game.launch();
        for input in inputs {
            game.set_paddle_input(input);
            game.step();
            if game.is_over() {
                break;
            }
            let ball = game.ball();
            prop_assert!(ball.x >= -0.01 && ball.x <= size.x + 0.01, "escaped sideways: {ball:?}");
            prop_assert!(ball.y >= -0.01, "escaped through the ceiling: {ball:?}");
        }
    }

    /// A moving ball keeps the speed its layout gave it.
    ///
    /// Every bounce rewrites the velocity, so a sign error or a missing
    /// normalisation shows up here as a ball that accelerates away or grinds to
    /// a halt — the classic Breakout bug, and invisible in a single-step test.
    ///
    /// The paddle deliberately *follows* the ball rather than taking random
    /// input. With random input the ball is usually lost within a few seconds,
    /// the loop ends before it is ever returned, and the paddle-bounce branch —
    /// the one that rewrites both velocity components — never runs at all. The
    /// `paddle_bounces` assertion at the end is what keeps that honest: it
    /// fails if this test ever stops exercising the code it claims to cover.
    #[test]
    fn bouncing_preserves_the_ball_speed(layout in any_layout()) {
        let speed = layout.ball_speed;
        let mut game = BreakoutGame::new(layout);
        game.launch();
        let mut paddle_bounces = 0u32;

        for _ in 0..3000 {
            let input = if game.ball().x < game.paddle_x() - 2.0 {
                PaddleInput::Left
            } else if game.ball().x > game.paddle_x() + 2.0 {
                PaddleInput::Right
            } else {
                PaddleInput::None
            };
            game.set_paddle_input(input);
            let outcome = game.step();
            if outcome.hit_paddle {
                paddle_bounces += 1;
            }
            if game.is_over() {
                break;
            }
            if game.ball_is_stuck() {
                // A life was lost anyway; put the ball back in play.
                game.launch();
                continue;
            }
            let v = game.ball_velocity();
            let actual = (v.x * v.x + v.y * v.y).sqrt();
            prop_assert!(
                (actual - speed).abs() < speed * 0.02,
                "speed drifted to {} from {}",
                actual,
                speed
            );
        }

        prop_assert!(
            paddle_bounces > 0,
            "the ball never reached the paddle, so this test proved nothing"
        );
    }

    /// Score never decreases, and lives never increase.
    #[test]
    fn score_and_lives_only_move_one_way(
        layout in any_layout(),
        inputs in prop::collection::vec(any_input(), 0..400),
    ) {
        let mut game = BreakoutGame::new(layout);
        game.launch();
        let (mut score, mut lives) = (game.score(), game.lives());
        for input in inputs {
            game.set_paddle_input(input);
            game.step();
            prop_assert!(game.score() >= score, "score fell");
            prop_assert!(game.lives() <= lives, "lives rose");
            score = game.score();
            lives = game.lives();
        }
    }

    /// Bricks are only ever destroyed, never resurrected, and the count agrees
    /// with the bricks themselves.
    #[test]
    fn bricks_remaining_tracks_the_brick_list(
        layout in any_layout(),
        inputs in prop::collection::vec(any_input(), 0..400),
    ) {
        let mut game = BreakoutGame::new(layout);
        game.launch();
        let mut remaining = game.bricks_remaining();
        for input in inputs {
            game.set_paddle_input(input);
            game.step();
            let counted = game.bricks().iter().filter(|b| b.alive()).count();
            prop_assert_eq!(counted, game.bricks_remaining());
            prop_assert!(game.bricks_remaining() <= remaining, "a brick came back");
            remaining = game.bricks_remaining();
        }
    }

    /// A finished game stays finished, and stepping it changes nothing.
    #[test]
    fn a_finished_game_is_frozen(
        layout in any_layout(),
        inputs in prop::collection::vec(any_input(), 0..600),
    ) {
        let mut game = BreakoutGame::new(layout);
        game.launch();
        play(&mut game, &inputs);
        if game.is_over() {
            let (status, score, ball, ticks) =
                (game.status(), game.score(), game.ball(), game.ticks());
            game.set_paddle_input(PaddleInput::Left);
            game.step();
            prop_assert_eq!(game.status(), status);
            prop_assert_eq!(game.score(), score);
            prop_assert_eq!(game.ticks(), ticks);
            prop_assert_eq!(game.ball(), ball);
            prop_assert_ne!(status, GameStatus::Playing);
        }
    }

    /// Interpolation stays on the segment between the last two steps.
    #[test]
    fn ball_at_interpolates_between_the_two_most_recent_positions(
        layout in any_layout(),
        inputs in prop::collection::vec(any_input(), 1..200),
        alpha in 0.0f32..=1.0,
    ) {
        let mut game = BreakoutGame::new(layout);
        game.launch();
        play(&mut game, &inputs);
        let (a, b, mid) = (game.ball_at(0.0), game.ball_at(1.0), game.ball_at(alpha));
        prop_assert_eq!(b, game.ball());
        // The interpolated point lies within the bounding box of the segment.
        let (lo_x, hi_x) = (a.x.min(b.x), a.x.max(b.x));
        let (lo_y, hi_y) = (a.y.min(b.y), a.y.max(b.y));
        prop_assert!(mid.x >= lo_x - 0.001 && mid.x <= hi_x + 0.001);
        prop_assert!(mid.y >= lo_y - 0.001 && mid.y <= hi_y + 0.001);
    }
}
