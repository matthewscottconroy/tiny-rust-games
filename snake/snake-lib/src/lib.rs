//! Engine-agnostic Snake rules.
//!
//! This is the repository's second demonstration of goal #4, and it exists to
//! answer a question `tic-tac-toe-lib` could not. Tic-tac-toe is turn-based: a
//! frontend calls into the library when the *player* acts. Snake is real-time —
//! the world moves whether or not anyone touches the keyboard — so something
//! has to own the clock.
//!
//! # The rule: the library never owns time
//!
//! [`SnakeGame`] exposes [`step`](SnakeGame::step), which advances the world by
//! exactly one tick. It never sleeps, never reads a clock, and never asks how
//! long a frame took. That is what keeps it engine-agnostic *and* what makes it
//! deterministic: the same seed and the same sequence of steps always produce
//! the same game, in a test or in a window.
//!
//! Turning real elapsed time into a whole number of steps is the frontend's
//! job — but it is the same job in every frontend, so [`Ticker`] does it here.
//! Give it a delta in seconds and it tells you how many steps to run:
//!
//! ```
//! use snake_lib::{SnakeGame, Ticker};
//!
//! let mut game = SnakeGame::new(20, 15, 42);
//! let mut ticker = Ticker::new(8.0); // eight steps per second
//!
//! // A frame that took 250 ms is worth two steps at 8 Hz.
//! for _ in 0..ticker.accumulate(0.25) {
//!     game.step();
//! }
//! assert_eq!(game.ticks(), 2);
//! ```
//!
//! This split is the whole lesson. A terminal frontend blocks on input and
//! calls `step` on a timer; Bevy calls it from a system with `Time::delta`;
//! Godot calls it from `process(delta)`. None of them contains a rule, and none
//! of them can disagree about how fast the snake moves.
//!
//! # Determinism buys replays
//!
//! Because nothing enters a game except its board size, its seed and the turns
//! queued on each tick, recording those three reproduces it exactly. See
//! [`Replay`]: a bug report becomes a few hundred bytes of readable text that
//! reproduces the death in CI, rather than a description of what someone
//! thought they pressed.
//!
//! # Input arrives faster than ticks
//!
//! At 8 steps per second a player can easily press two keys inside one tick.
//! Applying each immediately would let *up* then *left* — both individually
//! legal — turn the snake back into its own neck. So
//! [`queue_turn`](SnakeGame::queue_turn) records the intended direction and
//! [`step`](SnakeGame::step) commits it, validating against the direction
//! actually travelled rather than the last one requested.

#![cfg_attr(not(test), no_std)]

// The rules depend on nothing — not an engine, not a clock, and now not an
// operating system either. `Cargo.toml` has advertised `no-std-compatible`
// since this crate was written, which was simply untrue: it used
// `std::collections::VecDeque` and `std::error::Error`. Making the claim true
// is the smallest honest fix, and it is a real one — the same rules that drive
// Bevy, Godot and a browser now build for a bare-metal Cortex-M4, which CI
// checks on every push.
//
// `alloc` is still required. A snake grows, so its body is a heap-allocated
// deque; refusing allocation as well would mean a fixed-capacity board and a
// worse teaching example. `core::error::Error` (stable since Rust 1.81) is what
// lets `ReplayError` keep its trait impl without `std`.
extern crate alloc;

mod replay;
pub use replay::{REPLAY_VERSION, Replay, ReplayError};

use alloc::collections::VecDeque;

/// A cell on the board, `(0, 0)` at the top-left.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Coord {
    /// Column, increasing rightwards.
    pub x: i32,
    /// Row, increasing downwards.
    pub y: i32,
}

impl Coord {
    /// Creates a coordinate.
    pub fn new(x: i32, y: i32) -> Self {
        Self { x, y }
    }
}

/// One of the four directions the snake can travel.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Direction {
    /// Decreasing `y`.
    Up,
    /// Increasing `y`.
    Down,
    /// Decreasing `x`.
    Left,
    /// Increasing `x`.
    Right,
}

impl Direction {
    /// The unit step this direction moves the head by.
    pub fn delta(self) -> Coord {
        match self {
            Direction::Up => Coord::new(0, -1),
            Direction::Down => Coord::new(0, 1),
            Direction::Left => Coord::new(-1, 0),
            Direction::Right => Coord::new(1, 0),
        }
    }

    /// The direction facing the other way.
    pub fn opposite(self) -> Self {
        match self {
            Direction::Up => Direction::Down,
            Direction::Down => Direction::Up,
            Direction::Left => Direction::Right,
            Direction::Right => Direction::Left,
        }
    }
}

/// Why a game ended.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DeathCause {
    /// The head left the board.
    HitWall,
    /// The head entered a cell the body occupies.
    HitSelf,
}

/// Where a game stands.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GameStatus {
    /// The snake is still moving.
    Running,
    /// The snake died.
    Dead(DeathCause),
    /// The snake fills the board; there is nowhere left to grow.
    Won,
}

/// What a single [`step`](SnakeGame::step) did.
///
/// Returned so a frontend can react — play a sound, flash the screen — without
/// diffing the game state itself to work out what changed.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StepOutcome {
    /// The snake advanced one cell.
    Moved,
    /// The snake ate, grew, and new food was placed.
    Ate {
        /// Where the eaten food had been.
        at: Coord,
        /// Score after eating.
        score: u32,
    },
    /// The snake died this step.
    Died(DeathCause),
    /// The board is full; the game is won.
    Won,
    /// The game had already ended, so nothing happened.
    Ended,
}

/// Microseconds in one second, the unit [`Ticker`] counts in.
pub const MICROS_PER_SECOND: u64 = 1_000_000;

/// Rounds a non-negative `f32` to the nearest integer, half away from zero.
///
/// `f32::round` lives in `std`: float rounding is a libm intrinsic, and a
/// `no_std` build has no libm unless it takes a dependency for one. This crate
/// has no dependencies and is not about to gain one for a single addition.
///
/// For non-negative inputs `floor(x + 0.5)` *is* round-half-away-from-zero, and
/// the `as` cast truncates toward zero, which is `floor` here. The saturating
/// behaviour the callers rely on survives unchanged: NaN and negatives still
/// become 0, and `+inf` still becomes `u64::MAX`.
fn round_to_u64(value: f32) -> u64 {
    (value + 0.5) as u64
}

/// Converts a duration in seconds to whole microseconds.
///
/// Rounds rather than truncates, which matters more than it looks: `0.01f32` is
/// really `0.00999999977`, so truncating gives 9,999 µs and a hundred of them
/// fall 100 µs short of a second — one step lost. Rounding gives exactly
/// 10,000. Negatives, NaN and infinities all collapse to zero, so a clock
/// adjustment cannot rewind a ticker.
pub fn seconds_to_micros(seconds: f32) -> u64 {
    // No guard against negatives or NaN: Rust's float-to-int casts saturate, so
    // NaN, any negative and -inf already become 0, and +inf becomes `u64::MAX`
    // which the ticker's step cap absorbs. An explicit check here would be a
    // branch whose two arms behave identically — mutation testing found exactly
    // that and it was right.
    round_to_u64(seconds * MICROS_PER_SECOND as f32)
}

/// Converts elapsed real time into whole simulation steps.
///
/// Every frontend needs this and none of them should write it twice. Carrying
/// the remainder forward rather than discarding it each frame is what keeps the
/// snake's speed independent of frame rate: sixty short frames and six long
/// ones covering the same second produce the same number of steps.
///
/// # The accumulator is an integer on purpose
///
/// It counts whole microseconds, not seconds in `f32`. An earlier version
/// accumulated floats and drifted — a hundred frames of `0.01` summed to
/// `0.99999998` and silently lost a step against four frames of `0.25`. Its
/// documentation had to warn against building anything that needs exact
/// agreement, which is a bad thing for a timing primitive to have to say.
///
/// With integer microseconds the accumulator is exact: the only rounding
/// happens once, converting each incoming delta (see [`seconds_to_micros`]),
/// and it never compounds. Two tickers fed the same deltas in any grouping
/// agree exactly, on any machine, which is what makes replays and lockstep
/// simulation possible.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Ticker {
    micros_per_step: u64,
    accumulated: u64,
}

impl Ticker {
    /// Creates a ticker running at `steps_per_second`.
    ///
    /// # Panics
    /// Panics if `steps_per_second` is not positive and finite, or is so large
    /// that a step would take less than a microsecond.
    pub fn new(steps_per_second: f32) -> Self {
        assert!(
            steps_per_second > 0.0 && steps_per_second.is_finite(),
            "steps_per_second must be positive and finite, got {steps_per_second}"
        );
        let micros_per_step = round_to_u64(MICROS_PER_SECOND as f32 / steps_per_second);
        assert!(
            micros_per_step > 0,
            "steps_per_second {steps_per_second} is faster than one step per microsecond"
        );
        Self {
            micros_per_step,
            accumulated: 0,
        }
    }

    /// Adds `delta` seconds and returns how many steps are now due.
    ///
    /// A long stall (a breakpoint, a dragged window) would otherwise bank
    /// hundreds of steps and teleport the snake, so the return value is capped
    /// at [`Ticker::MAX_STEPS_PER_CALL`] and the surplus is discarded.
    pub fn accumulate(&mut self, delta: f32) -> u32 {
        self.accumulate_micros(seconds_to_micros(delta))
    }

    /// Adds `micros` microseconds and returns how many steps are now due.
    ///
    /// The exact path, with no float conversion at all. A replay feeds this
    /// directly so playback cannot diverge from the recording by even one step.
    pub fn accumulate_micros(&mut self, micros: u64) -> u32 {
        self.accumulated = self.accumulated.saturating_add(micros);

        let due = self.accumulated / self.micros_per_step;
        let steps = due.min(Self::MAX_STEPS_PER_CALL as u64) as u32;
        if due > Self::MAX_STEPS_PER_CALL as u64 {
            // Dropped the backlog, so drop the remainder with it rather than
            // paying it out on the next call.
            self.accumulated = 0;
        } else {
            self.accumulated -= steps as u64 * self.micros_per_step;
        }
        steps
    }

    /// Upper bound on steps returned from one [`accumulate`](Ticker::accumulate).
    pub const MAX_STEPS_PER_CALL: u32 = 8;

    /// Steps per second this ticker runs at.
    pub fn steps_per_second(&self) -> f32 {
        MICROS_PER_SECOND as f32 / self.micros_per_step as f32
    }

    /// Microseconds one step takes.
    pub fn micros_per_step(&self) -> u64 {
        self.micros_per_step
    }

    /// Fraction of the way to the next step, in `0.0..1.0`.
    ///
    /// Frontends use this to interpolate the snake between cells so movement
    /// looks smooth at 60 fps while the simulation runs at 8 Hz.
    pub fn alpha(&self) -> f32 {
        (self.accumulated as f32 / self.micros_per_step as f32).clamp(0.0, 1.0)
    }
}

/// A deterministic pseudo-random source.
///
/// Hand-rolled so the crate has no dependencies at all — the point of this
/// library is that it drops into any engine.
#[derive(Debug, Clone)]
struct Rng(u64);

impl Rng {
    fn new(seed: u64) -> Self {
        // Any odd constant works; this one is the usual LCG multiplier pair.
        Self(seed.wrapping_mul(6364136223846793005).wrapping_add(1))
    }

    fn next_u32(&mut self) -> u32 {
        self.0 = self
            .0
            .wrapping_mul(6364136223846793005)
            .wrapping_add(1442695040888963407);
        (self.0 >> 33) as u32
    }

    /// Uniform in `0..bound`.
    fn below(&mut self, bound: usize) -> usize {
        (self.next_u32() as usize) % bound.max(1)
    }
}

/// A game of Snake: the board, the snake, the food, and the rules.
#[derive(Debug, Clone)]
pub struct SnakeGame {
    width: i32,
    height: i32,
    /// Head first, tail last.
    body: VecDeque<Coord>,
    direction: Direction,
    queued: Option<Direction>,
    food: Option<Coord>,
    status: GameStatus,
    score: u32,
    ticks: u64,
    rng: Rng,
}

/// The smallest playable board edge, in cells.
///
/// A 1-wide board has no room for the snake to turn, and a 0-wide one has no
/// cells at all. [`Replay::from_text`](crate::Replay::from_text) checks incoming
/// files against this rather than letting [`SnakeGame::new`] panic on them.
pub const MIN_BOARD_SIZE: i32 = 2;

/// The largest board edge, in cells.
///
/// Nothing about Snake needs an upper bound, but food placement counts free
/// cells as `width * height` in `i32`. A 46_341-square board overflows that and
/// panics, so an unbounded edge turns a plausible-looking number in a replay
/// file into a crash. The cap keeps the product comfortably inside `i32` and
/// keeps the O(board) food scan quick; it is some three orders of magnitude
/// larger than any board here (the demos use 24x18).
pub const MAX_BOARD_SIZE: i32 = 4096;

impl SnakeGame {
    /// Starts a game on a `width`x`height` board.
    ///
    /// The snake begins length 1 at the centre travelling right, and `seed`
    /// fixes every food placement, so two games with the same seed and the same
    /// inputs are identical.
    ///
    /// # Panics
    /// Panics if either dimension is less than 2.
    pub fn new(width: i32, height: i32, seed: u64) -> Self {
        assert!(
            width >= MIN_BOARD_SIZE && height >= MIN_BOARD_SIZE,
            "board must be at least {MIN_BOARD_SIZE}x{MIN_BOARD_SIZE}, got {width}x{height}"
        );
        assert!(
            width <= MAX_BOARD_SIZE && height <= MAX_BOARD_SIZE,
            "board must be at most {MAX_BOARD_SIZE}x{MAX_BOARD_SIZE}, got {width}x{height}"
        );
        let mut body = VecDeque::new();
        body.push_back(Coord::new(width / 2, height / 2));

        let mut game = Self {
            width,
            height,
            body,
            direction: Direction::Right,
            queued: None,
            food: None,
            status: GameStatus::Running,
            score: 0,
            ticks: 0,
            rng: Rng::new(seed),
        };
        game.place_food();
        game
    }

    /// Requests a direction change, applied by the next
    /// [`step`](SnakeGame::step).
    ///
    /// Reversing straight into the snake's own neck is rejected, and so is a
    /// change once the game has ended. Queuing is what makes this safe when
    /// several keys are pressed within one tick — see the module docs.
    ///
    /// Returns whether the request was accepted.
    pub fn queue_turn(&mut self, direction: Direction) -> bool {
        if self.status != GameStatus::Running {
            return false;
        }
        // A length-1 snake has no neck, so any direction is legal.
        if self.body.len() > 1 && direction == self.direction.opposite() {
            return false;
        }
        self.queued = Some(direction);
        true
    }

    /// Advances the world by exactly one tick.
    ///
    /// This is the only thing that mutates the game. It does not know how much
    /// real time has passed and does not care — see [`Ticker`].
    pub fn step(&mut self) -> StepOutcome {
        if self.status != GameStatus::Running {
            return StepOutcome::Ended;
        }
        self.ticks += 1;

        // Commit the queued turn against the direction actually travelled.
        if let Some(next) = self.queued.take()
            && (self.body.len() == 1 || next != self.direction.opposite())
        {
            self.direction = next;
        }

        let head = *self.body.front().expect("snake is never empty");
        let delta = self.direction.delta();
        let next_head = Coord::new(head.x + delta.x, head.y + delta.y);

        if !self.contains(next_head) {
            self.status = GameStatus::Dead(DeathCause::HitWall);
            return StepOutcome::Died(DeathCause::HitWall);
        }

        let eating = self.food == Some(next_head);

        // The tail vacates as the head arrives, so the cell it leaves is free
        // this tick — unless the snake is growing, in which case it stays put.
        if !eating {
            self.body.pop_back();
        }
        if self.body.contains(&next_head) {
            // Put the tail back so the death state shows the real snake.
            if !eating {
                self.body.push_back(self.last_tail());
            }
            self.status = GameStatus::Dead(DeathCause::HitSelf);
            return StepOutcome::Died(DeathCause::HitSelf);
        }

        self.body.push_front(next_head);

        if eating {
            self.score += 1;
            self.food = None;
            if self.body.len() as i32 == self.width * self.height {
                self.status = GameStatus::Won;
                return StepOutcome::Won;
            }
            self.place_food();
            return StepOutcome::Ate {
                at: next_head,
                score: self.score,
            };
        }
        StepOutcome::Moved
    }

    /// Restarts with a fresh board, keeping the dimensions and reseeding.
    pub fn reset(&mut self, seed: u64) {
        *self = Self::new(self.width, self.height, seed);
    }

    /// Board width in cells.
    pub fn width(&self) -> i32 {
        self.width
    }

    /// Board height in cells.
    pub fn height(&self) -> i32 {
        self.height
    }

    /// Whether a coordinate lies on the board.
    pub fn contains(&self, c: Coord) -> bool {
        c.x >= 0 && c.y >= 0 && c.x < self.width && c.y < self.height
    }

    /// The snake, head first.
    pub fn body(&self) -> impl Iterator<Item = Coord> + '_ {
        self.body.iter().copied()
    }

    /// The snake's head.
    pub fn head(&self) -> Coord {
        *self.body.front().expect("snake is never empty")
    }

    /// How many cells the snake occupies.
    pub fn len(&self) -> usize {
        self.body.len()
    }

    /// Always `false` — the snake always occupies at least one cell.
    ///
    /// Present because clippy expects it beside [`len`](SnakeGame::len), and it
    /// documents the invariant.
    pub fn is_empty(&self) -> bool {
        false
    }

    /// The current food, or `None` on a full board.
    pub fn food(&self) -> Option<Coord> {
        self.food
    }

    /// The direction the snake is travelling.
    pub fn direction(&self) -> Direction {
        self.direction
    }

    /// Food eaten so far.
    pub fn score(&self) -> u32 {
        self.score
    }

    /// How many steps have been taken.
    pub fn ticks(&self) -> u64 {
        self.ticks
    }

    /// Where the game stands.
    pub fn status(&self) -> GameStatus {
        self.status
    }

    /// Whether the game has ended, by death or by filling the board.
    pub fn is_over(&self) -> bool {
        self.status != GameStatus::Running
    }

    /// The tail cell, used to undo a speculative `pop_back`.
    fn last_tail(&self) -> Coord {
        *self.body.back().expect("snake is never empty")
    }

    /// Places food on a random free cell, or clears it if the board is full.
    ///
    /// Picks the nth *free* cell rather than retrying random cells, so it stays
    /// O(board) even when the snake covers almost everything.
    fn place_food(&mut self) {
        let free = (self.width * self.height) as usize - self.body.len();
        if free == 0 {
            self.food = None;
            return;
        }
        let mut nth = self.rng.below(free);
        for y in 0..self.height {
            for x in 0..self.width {
                let c = Coord::new(x, y);
                if self.body.contains(&c) {
                    continue;
                }
                if nth == 0 {
                    self.food = Some(c);
                    return;
                }
                nth -= 1;
            }
        }
        unreachable!("free cell count and scan must agree");
    }
}

#[cfg(test)]
mod tests;
