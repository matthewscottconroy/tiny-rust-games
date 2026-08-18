//! Prints a digest of a scripted game's final state.
//!
//! Run on several platforms, the output answers a question the rest of the
//! test suite cannot: `breakout-lib` claims its physics are reproducible, but
//! every test asserting that runs on one machine, where reproducibility is
//! nearly free. The claim worth checking is reproducibility *across* machines,
//! and this game is `f32` throughout — a type whose results are famously
//! allowed to differ between targets once a compiler starts contracting
//! multiply-adds or choosing different instruction sequences.
//!
//! So this is a probe, not an assertion. CI runs it on Linux, macOS and
//! Windows and compares the three lines. If they agree, the claim is on much
//! firmer ground than a single-platform test could put it. If they disagree,
//! that is worth knowing and worth documenting — the same way measuring
//! `spatial-partitioning` was worth it precisely because it falsified the
//! claim in its own docs.
//!
//! ```text
//! cargo run --example state-hash -p breakout-lib
//! ```

use breakout_lib::{BreakoutGame, PaddleInput};

/// FNV-1a, written out rather than pulled in.
///
/// The crate has no dependencies and this is not the place to give it one; the
/// digest only has to be sensitive to every byte, not cryptographic.
struct Fnv(u64);

impl Fnv {
    fn new() -> Self {
        Self(0xcbf2_9ce4_8422_2325)
    }

    fn write(&mut self, bytes: &[u8]) {
        for b in bytes {
            self.0 ^= u64::from(*b);
            self.0 = self.0.wrapping_mul(0x1000_0000_01b3);
        }
    }

    /// Hashes the exact bit pattern, not the printed value.
    ///
    /// Two `f32`s that differ in the last bit print the same at most precisions
    /// but are different numbers, and that difference is the entire point here.
    fn write_f32(&mut self, value: f32) {
        self.write(&value.to_bits().to_le_bytes());
    }

    fn write_u64(&mut self, value: u64) {
        self.write(&value.to_le_bytes());
    }
}

fn main() {
    let mut game = BreakoutGame::new(BreakoutGame::default_layout());
    game.launch();

    // A fixed policy rather than a fixed input list: following the ball keeps
    // it in play for thousands of steps, so the run accumulates far more
    // floating-point history — bounces off walls, bricks and the paddle — than
    // a scripted sequence that loses the ball early would.
    for _ in 0..20_000 {
        let input = if game.ball().x < game.paddle_x() - 2.0 {
            PaddleInput::Left
        } else if game.ball().x > game.paddle_x() + 2.0 {
            PaddleInput::Right
        } else {
            PaddleInput::None
        };
        game.set_paddle_input(input);
        game.step();
        if game.is_over() {
            break;
        }
        if game.ball_is_stuck() {
            game.launch();
        }
    }

    let mut hash = Fnv::new();
    hash.write_u64(game.ticks());
    hash.write_u64(u64::from(game.score()));
    hash.write_u64(u64::from(game.lives()));
    hash.write_f32(game.ball().x);
    hash.write_f32(game.ball().y);
    hash.write_f32(game.ball_velocity().x);
    hash.write_f32(game.ball_velocity().y);
    hash.write_f32(game.paddle_x());
    for brick in game.bricks() {
        hash.write(&[brick.hits]);
    }

    println!("breakout {:016x}", hash.0);
    println!("  ticks  {}", game.ticks());
    println!("  score  {}", game.score());
    println!("  ball   {:?}", game.ball());
    println!("  vel    {:?}", game.ball_velocity());
}
