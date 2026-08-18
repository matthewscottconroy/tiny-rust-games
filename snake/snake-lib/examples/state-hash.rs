//! Prints a digest of a replayed game's final state.
//!
//! The companion to `breakout-lib`'s probe of the same name, and the easier
//! half of the question. Snake's state is integers on a grid, so it should
//! reproduce anywhere; Breakout's is `f32`, where that is genuinely uncertain.
//! Running both tells you whether a disagreement is a property of floating
//! point or of something more embarrassing.
//!
//! This deliberately goes through [`Replay`] rather than driving the game
//! directly, so it exercises the path the replay feature actually promises:
//! that a recorded file reproduces a game exactly, on any machine.
//!
//! ```text
//! cargo run --example state-hash -p snake-lib
//! ```

use snake_lib::{Direction, Replay, SnakeGame};

/// FNV-1a, written out rather than pulled in — the crate has no dependencies.
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

    fn write_i32(&mut self, value: i32) {
        self.write(&value.to_le_bytes());
    }

    fn write_u64(&mut self, value: u64) {
        self.write(&value.to_le_bytes());
    }
}

fn main() {
    // Drive a live game with a fixed policy and record what it does, rather
    // than hand-writing a turn list. A hand-written script walks the snake into
    // a wall within a few seconds, and a game that ends at tick 43 with a score
    // of zero hashes almost no state — a probe that would agree across
    // platforms even if the rules were badly broken.
    let (width, height, seed) = (31, 23, 0x5EED_C0FF_EE00_1234);
    let mut game = SnakeGame::new(width, height, seed);
    let mut replay = Replay::recording(width, height, seed);

    for _ in 0..4_000 {
        if game.is_over() {
            break;
        }
        if let Some(direction) = seek(&game) {
            // Record and queue the same turn: the recording has to mirror the
            // inputs the game actually received, or the replay is fiction.
            replay.record_turn(game.ticks(), direction);
            game.queue_turn(direction);
        }
        game.step();
    }

    // Replaying must reproduce the live game exactly. Checking it here means
    // the digest below describes a replay that is known to match the original,
    // so a cross-platform difference cannot be blamed on the replay layer.
    let replayed = replay.play_out(0);
    assert_eq!(
        replayed.ticks(),
        game.ticks(),
        "replay diverged from the game"
    );
    assert_eq!(
        replayed.score(),
        game.score(),
        "replay diverged from the game"
    );
    assert_eq!(
        replayed.body().collect::<Vec<_>>(),
        game.body().collect::<Vec<_>>(),
        "replay diverged from the game"
    );

    let mut hash = Fnv::new();
    hash.write_u64(replayed.ticks());
    hash.write_u64(u64::from(replayed.score()));
    for cell in replayed.body() {
        hash.write_i32(cell.x);
        hash.write_i32(cell.y);
    }
    if let Some(food) = replayed.food() {
        hash.write_i32(food.x);
        hash.write_i32(food.y);
    }

    println!("snake {:016x}", hash.0);
    println!("  ticks  {}", replayed.ticks());
    println!("  score  {}", replayed.score());
    println!("  len    {}", replayed.len());
    println!("  status {:?}", replayed.status());
}

/// Steers greedily toward the food, preferring the axis it is furthest along.
///
/// Deliberately simple and deliberately not perfect — it eventually traps
/// itself, which is fine. What matters is that it is a pure function of the
/// game state, so every platform makes the same choices.
fn seek(game: &SnakeGame) -> Option<Direction> {
    let food = game.food()?;
    let head = game.head();
    let (dx, dy) = (food.x - head.x, food.y - head.y);

    let horizontal = if dx > 0 {
        Some(Direction::Right)
    } else if dx < 0 {
        Some(Direction::Left)
    } else {
        None
    };
    let vertical = if dy > 0 {
        Some(Direction::Down)
    } else if dy < 0 {
        Some(Direction::Up)
    } else {
        None
    };

    // Try the longer axis first, then the other; skip a direction that would
    // reverse into the neck, which the library refuses anyway.
    let (first, second) = if dx.abs() >= dy.abs() {
        (horizontal, vertical)
    } else {
        (vertical, horizontal)
    };
    let reverse = game.direction().opposite();
    [first, second]
        .into_iter()
        .flatten()
        .find(|candidate| *candidate != reverse)
}
