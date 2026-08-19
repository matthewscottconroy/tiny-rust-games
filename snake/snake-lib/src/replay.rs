//! Recording and replaying a game exactly.
//!
//! A [`SnakeGame`] is fully determined by three things: the board size, the
//! seed, and which turns were queued on which tick. Nothing else enters — the
//! library reads no clock and no global state — so recording those three
//! reproduces the game frame for frame, forever.
//!
//! That is worth more than the ghost-replay feature it enables. A bug report
//! stops being "the snake died and I think I pressed left" and becomes a file
//! of a few hundred bytes that reproduces the death in CI.
//!
//! ```
//! use snake_lib::{Direction, Replay, SnakeGame};
//!
//! // Record: wrap the inputs you feed the game.
//! let mut recording = Replay::recording(20, 15, 7);
//! let mut game = SnakeGame::new(20, 15, 7);
//! for tick in 0..30 {
//!     if tick == 5 {
//!         recording.record_turn(game.ticks(), Direction::Down);
//!         game.queue_turn(Direction::Down);
//!     }
//!     game.step();
//! }
//!
//! // Replay: the result is identical, without needing the original game.
//! let replayed = recording.play(30);
//! assert_eq!(replayed.score(), game.score());
//! assert_eq!(replayed.body().collect::<Vec<_>>(), game.body().collect::<Vec<_>>());
//! ```
//!
//! # The file format
//!
//! Plain text, one directive per line, because a replay you can read in a diff
//! is far more useful in a bug report than an opaque blob — and because this
//! crate has no dependencies and is not about to gain a serialisation one for
//! four fields.
//!
//! ```text
//! snake-replay 1
//! board 20 15
//! seed 7
//! turn 5 down
//! turn 12 left
//! ```
//!
//! A replay is read from outside the program, so [`Replay::from_text`] treats
//! every field as untrusted: `board` must lie within
//! [`MIN_BOARD_SIZE`]..=[`MAX_BOARD_SIZE`], turns must be in ascending tick
//! order, and anything else is a [`ReplayError`] rather than a panic. The rule
//! is that whatever the parser accepts must be playable — see
//! `tests/replay_fuzz.rs`, which exists to keep that true.

use alloc::string::{String, ToString as _};
use alloc::vec::Vec;
use core::fmt::Write as _;

use crate::{Direction, MAX_BOARD_SIZE, MIN_BOARD_SIZE, SnakeGame};

/// Why a replay could not be parsed.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ReplayError {
    /// The first line was not a `snake-replay <version>` header.
    MissingHeader,
    /// The file declares a format version this build does not understand.
    UnsupportedVersion(u32),
    /// A line was not one of the known directives, or had the wrong shape.
    BadLine {
        /// One-based line number.
        line: usize,
        /// The offending text.
        text: String,
    },
    /// The file parsed but never declared a board size.
    MissingBoard,
    /// The declared board is outside [`MIN_BOARD_SIZE`]..=[`MAX_BOARD_SIZE`]
    /// and cannot be played.
    InvalidBoard {
        /// The declared width.
        width: i32,
        /// The declared height.
        height: i32,
    },
    /// Turns must be recorded in tick order; this one went backwards.
    OutOfOrderTurn {
        /// The tick that appeared after a later one.
        tick: u64,
    },
}

impl core::fmt::Display for ReplayError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Self::MissingHeader => write!(f, "not a snake replay: missing header"),
            Self::UnsupportedVersion(v) => write!(f, "unsupported replay version {v}"),
            Self::BadLine { line, text } => write!(f, "line {line}: cannot parse {text:?}"),
            Self::MissingBoard => write!(f, "replay declares no board size"),
            Self::InvalidBoard { width, height } => write!(
                f,
                "replay declares a {width}x{height} board, but the limits are \
                 {MIN_BOARD_SIZE}x{MIN_BOARD_SIZE} to {MAX_BOARD_SIZE}x{MAX_BOARD_SIZE}"
            ),
            Self::OutOfOrderTurn { tick } => {
                write!(f, "turn at tick {tick} is out of order")
            }
        }
    }
}

impl core::error::Error for ReplayError {}

/// The format version this build writes.
pub const REPLAY_VERSION: u32 = 1;

/// A recording of a game: everything needed to reproduce it exactly.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Replay {
    width: i32,
    height: i32,
    seed: u64,
    /// `(tick, direction)` pairs, in ascending tick order.
    turns: Vec<(u64, Direction)>,
}

impl Replay {
    /// Starts an empty recording for a game of this shape.
    ///
    /// # Panics
    /// Panics on a board smaller than [`MIN_BOARD_SIZE`] square, matching
    /// [`SnakeGame::new`]. These are your own numbers, so a mistake here is a
    /// bug rather than bad input; [`from_text`](Self::from_text) returns
    /// [`ReplayError::InvalidBoard`] instead, because its numbers come from a
    /// file someone else wrote.
    pub fn recording(width: i32, height: i32, seed: u64) -> Self {
        assert!(
            (MIN_BOARD_SIZE..=MAX_BOARD_SIZE).contains(&width)
                && (MIN_BOARD_SIZE..=MAX_BOARD_SIZE).contains(&height),
            "board must be {MIN_BOARD_SIZE}x{MIN_BOARD_SIZE} to \
             {MAX_BOARD_SIZE}x{MAX_BOARD_SIZE}, got {width}x{height}"
        );
        Self {
            width,
            height,
            seed,
            turns: Vec::new(),
        }
    }

    /// Records a turn queued at `tick`.
    ///
    /// Pass [`SnakeGame::ticks`] as `tick`, before stepping. A turn queued at
    /// the same tick as an earlier one replaces it, matching the live game,
    /// where the last request before a step is the one that counts.
    ///
    /// # Panics
    /// Panics if `tick` precedes the last recorded turn — a recording must be
    /// made in order, and silently accepting a rewind would produce a replay
    /// that does not match what happened.
    pub fn record_turn(&mut self, tick: u64, direction: Direction) {
        if let Some((last, dir)) = self.turns.last_mut() {
            assert!(
                tick >= *last,
                "turn at tick {tick} recorded after tick {last}"
            );
            if *last == tick {
                *dir = direction;
                return;
            }
        }
        self.turns.push((tick, direction));
    }

    /// Board width in cells.
    pub fn width(&self) -> i32 {
        self.width
    }

    /// Board height in cells.
    pub fn height(&self) -> i32 {
        self.height
    }

    /// The seed the recorded game used.
    pub fn seed(&self) -> u64 {
        self.seed
    }

    /// The recorded turns, in tick order.
    pub fn turns(&self) -> &[(u64, Direction)] {
        &self.turns
    }

    /// The tick of the last recorded turn, if any.
    pub fn last_turn_tick(&self) -> Option<u64> {
        self.turns.last().map(|(tick, _)| *tick)
    }

    /// Replays the recording for `ticks` steps and returns the resulting game.
    ///
    /// Stops early if the snake dies, exactly as the live game would.
    pub fn play(&self, ticks: u64) -> SnakeGame {
        let mut game = SnakeGame::new(self.width, self.height, self.seed);
        let mut next = 0;
        for _ in 0..ticks {
            if game.is_over() {
                break;
            }
            // Apply every turn recorded for this tick, in order.
            while let Some((tick, direction)) = self.turns.get(next) {
                if *tick != game.ticks() {
                    break;
                }
                game.queue_turn(*direction);
                next += 1;
            }
            game.step();
        }
        game
    }

    /// Replays far enough to reach the end of the recording, plus `extra` steps.
    ///
    /// Convenient for "play this back to the death" without the caller having to
    /// know how long the game ran.
    pub fn play_out(&self, extra: u64) -> SnakeGame {
        self.play(self.last_turn_tick().map_or(0, |t| t + 1) + extra)
    }

    /// Serialises to the text format described in the module documentation.
    pub fn to_text(&self) -> String {
        let mut out = String::new();
        writeln!(out, "snake-replay {REPLAY_VERSION}").expect("writing to a String cannot fail");
        writeln!(out, "board {} {}", self.width, self.height).expect("infallible");
        writeln!(out, "seed {}", self.seed).expect("infallible");
        for (tick, direction) in &self.turns {
            writeln!(out, "turn {tick} {}", direction_name(*direction)).expect("infallible");
        }
        out
    }

    /// Parses the text format.
    ///
    /// # Errors
    /// Returns [`ReplayError`] describing the first problem found.
    pub fn from_text(text: &str) -> Result<Self, ReplayError> {
        let mut lines = text.lines().enumerate();

        let (_, header) = lines.next().ok_or(ReplayError::MissingHeader)?;
        let version = header
            .strip_prefix("snake-replay ")
            .ok_or(ReplayError::MissingHeader)?
            .trim()
            .parse::<u32>()
            .map_err(|_| ReplayError::MissingHeader)?;
        if version != REPLAY_VERSION {
            return Err(ReplayError::UnsupportedVersion(version));
        }

        let mut board = None;
        let mut seed = 0u64;
        let mut turns: Vec<(u64, Direction)> = Vec::new();

        for (index, raw) in lines {
            let line = raw.trim();
            // Blank lines and `#` comments let a human annotate a bug report.
            if line.is_empty() || line.starts_with('#') {
                continue;
            }
            let number = index + 1;
            let bad = || ReplayError::BadLine {
                line: number,
                text: raw.to_string(),
            };
            let parts: Vec<&str> = line.split_whitespace().collect();

            match parts.as_slice() {
                ["board", w, h] => {
                    board = Some((
                        w.parse::<i32>().map_err(|_| bad())?,
                        h.parse::<i32>().map_err(|_| bad())?,
                    ));
                }
                ["seed", s] => seed = s.parse::<u64>().map_err(|_| bad())?,
                ["turn", tick, dir] => {
                    let tick = tick.parse::<u64>().map_err(|_| bad())?;
                    let direction = direction_from_name(dir).ok_or_else(bad)?;
                    if let Some((last, _)) = turns.last()
                        && *last > tick
                    {
                        return Err(ReplayError::OutOfOrderTurn { tick });
                    }
                    turns.push((tick, direction));
                }
                _ => return Err(bad()),
            }
        }

        let (width, height) = board.ok_or(ReplayError::MissingBoard)?;
        // A replay arrives from outside the program — a bug report, a file on
        // disk, a paste into an issue — so a board `SnakeGame::new` would
        // reject has to be an error here. Accepting it and panicking later, at
        // the first `play`, turns someone else's malformed file into a crash in
        // your process, a long way from the line that actually caused it.
        if !(MIN_BOARD_SIZE..=MAX_BOARD_SIZE).contains(&width)
            || !(MIN_BOARD_SIZE..=MAX_BOARD_SIZE).contains(&height)
        {
            return Err(ReplayError::InvalidBoard { width, height });
        }
        Ok(Self {
            width,
            height,
            seed,
            turns,
        })
    }
}

/// The name written for a direction in a replay file.
fn direction_name(direction: Direction) -> &'static str {
    match direction {
        Direction::Up => "up",
        Direction::Down => "down",
        Direction::Left => "left",
        Direction::Right => "right",
    }
}

/// The inverse of [`direction_name`].
fn direction_from_name(name: &str) -> Option<Direction> {
    match name {
        "up" => Some(Direction::Up),
        "down" => Some(Direction::Down),
        "left" => Some(Direction::Left),
        "right" => Some(Direction::Right),
        _ => None,
    }
}

#[cfg(test)]
mod tests;
