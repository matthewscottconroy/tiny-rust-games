//! Two-player Snake over **lockstep** networking.
//!
//! This is the fourth architecture in the repository, after turn-based
//! (`tic-tac-toe/`), real-time tick (`snake/`) and continuous physics
//! (`breakout/`) — and it is the one the other three were quietly building
//! toward, because it is the one that cannot be faked.
//!
//! # What lockstep is
//!
//! Nothing about the game is ever sent over the network. Each peer runs the
//! whole simulation itself, and the peers exchange only *inputs*: "on tick 47 I
//! pressed left". A tick advances when both peers' inputs for it are known.
//!
//! Two players, two boards. Each peer simulates **both** of them — its own and
//! its opponent's — from the same seeds and the same inputs. Nobody is the
//! server, nobody is authoritative, and no positions cross the wire. A turn is
//! nine bytes.
//!
//! # Why it needs everything that came before
//!
//! Lockstep only works if two machines running the same inputs produce the same
//! state, *exactly*, forever. One divergent bit on tick 500 — a hash iteration
//! order, a float rounding differently, a clock read — and the two games drift
//! apart with neither peer aware of it.
//!
//! [`snake_lib`] was built for this without knowing it:
//!
//! - `SnakeGame::step()` advances exactly one tick and never reads a clock, so
//!   there is no wall-time input to diverge on;
//! - the RNG is seeded and part of the game state, so food lands identically;
//! - `Ticker` counts whole microseconds rather than accumulating `f32`, so two
//!   peers fed different frame rates still take the same number of steps;
//! - and CI compares a digest of a finished game across Linux, macOS and
//!   Windows, so "identical" is measured rather than assumed.
//!
//! Take away any one of those and this module cannot exist. That is the point
//! of it: determinism is not a nice property, it is a load-bearing one.
//!
//! # Input delay
//!
//! A peer cannot wait for a packet that has not arrived without freezing, so
//! input for tick `T` is sent at tick `T - delay`. The cost is that a keypress
//! takes `delay` ticks to appear — the input lag every lockstep game has,
//! traded directly against how much network jitter it can absorb.
//!
//! # Desync detection
//!
//! Peers periodically exchange a checksum of the whole world. If the checksums
//! for a tick disagree, the games have diverged and the match is over: there is
//! no recovery in lockstep, only detection. Silent divergence is far worse than
//! a reported one, because both players keep playing different games.

use std::collections::BTreeMap;

pub use snake_lib::Direction;
use snake_lib::{Coord, SnakeGame};

/// Which of the two players a peer is.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Seat {
    /// The player who created the match.
    Host,
    /// The player who joined it.
    Guest,
}

impl Seat {
    /// The other seat.
    pub fn other(self) -> Self {
        match self {
            Self::Host => Self::Guest,
            Self::Guest => Self::Host,
        }
    }

    /// Index into the two-element arrays used throughout.
    pub fn index(self) -> usize {
        match self {
            Self::Host => 0,
            Self::Guest => 1,
        }
    }
}

/// One player's input for one tick: a steering choice, or nothing.
///
/// "Nothing" has to be sent explicitly. A missing message is indistinguishable
/// from a slow one, and a peer that guessed would diverge the moment it guessed
/// wrong.
pub type Turn = Option<Direction>;

/// A message between peers. The entire protocol.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Message {
    /// This seat's input for a tick.
    Input {
        /// The tick the input applies to.
        tick: u64,
        /// What was pressed, if anything.
        turn: Turn,
    },
    /// A checksum of the sender's whole world at the end of a tick.
    Checksum {
        /// The tick the checksum describes.
        tick: u64,
        /// The digest itself.
        digest: u64,
    },
}

/// Why a match stopped.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Desync {
    /// The peers' checksums for a tick disagreed: the games have diverged.
    ChecksumMismatch {
        /// The tick at which they first disagreed.
        tick: u64,
        /// What this peer computed.
        local: u64,
        /// What the other peer reported.
        remote: u64,
    },
}

/// How often peers compare checksums, in ticks.
///
/// Every tick would be wasteful and never would be negligent; the cost of a
/// larger number is only how long a divergence goes unnoticed.
pub const CHECKSUM_INTERVAL: u64 = 30;

/// Both games, and the lockstep bookkeeping around them.
///
/// Each peer owns one of these and they are expected to stay identical.
pub struct Lockstep {
    seat: Seat,
    games: [SnakeGame; 2],
    /// Inputs known so far, per tick, per seat.
    inputs: BTreeMap<u64, [Option<Turn>; 2]>,
    /// Checksums received from the other peer but not yet compared.
    remote_checksums: BTreeMap<u64, u64>,
    /// Our own checksums, kept until the other peer's arrives.
    local_checksums: BTreeMap<u64, u64>,
    tick: u64,
    delay: u64,
    /// The tick the next locally-sent input will apply to.
    ///
    /// Deliberately a counter rather than `tick + delay`. A peer can advance
    /// several ticks in one pass — after a burst of buffered messages arrives,
    /// say — and deriving the target from the current tick then skips the ticks
    /// in between, leaving holes no input ever fills. The match stalls forever
    /// on the first hole, which is exactly what the demo did before this was a
    /// counter.
    next_input_tick: u64,
    outbox: Vec<Message>,
}

impl Lockstep {
    /// Starts a match.
    ///
    /// Both peers must pass the same seeds and the same `delay`, which is the
    /// usual lockstep handshake: everything that shapes the simulation is
    /// agreed before it starts, and nothing that shapes it is sent afterwards.
    ///
    /// # Panics
    /// Panics if `delay` is zero. A delay of zero means a peer would need the
    /// other's input for the tick it is about to run, which cannot arrive in
    /// time on any real network.
    pub fn new(seat: Seat, width: i32, height: i32, seeds: [u64; 2], delay: u64) -> Self {
        assert!(delay > 0, "input delay must be at least 1 tick");
        let mut this = Self {
            seat,
            games: [
                SnakeGame::new(width, height, seeds[0]),
                SnakeGame::new(width, height, seeds[1]),
            ],
            inputs: BTreeMap::new(),
            remote_checksums: BTreeMap::new(),
            local_checksums: BTreeMap::new(),
            tick: 0,
            delay,
            next_input_tick: delay,
            outbox: Vec::new(),
        };
        // The first `delay` ticks have no input from anyone — there was no
        // earlier tick to have sent it. Filling them in is what lets the match
        // start rather than deadlock on tick zero.
        for tick in 0..delay {
            for seat in [Seat::Host, Seat::Guest] {
                this.record(tick, seat, None);
            }
        }
        this
    }

    /// This peer's seat.
    pub fn seat(&self) -> Seat {
        self.seat
    }

    /// The next tick that has not yet run.
    pub fn tick(&self) -> u64 {
        self.tick
    }

    /// A game, by seat.
    pub fn game(&self, seat: Seat) -> &SnakeGame {
        &self.games[seat.index()]
    }

    /// Whether either snake has died, which ends the match.
    pub fn is_over(&self) -> bool {
        self.games.iter().any(SnakeGame::is_over)
    }

    /// The winner, once the match is over.
    ///
    /// Returns `None` while the match is running or if both died on the same
    /// tick, which is a draw — and is reachable, because both games advance on
    /// the same tick.
    pub fn winner(&self) -> Option<Seat> {
        match (self.games[0].is_over(), self.games[1].is_over()) {
            (true, false) => Some(Seat::Guest),
            (false, true) => Some(Seat::Host),
            _ => None,
        }
    }

    /// Queues this peer's input for the next unscheduled tick, and returns the
    /// message to send.
    ///
    /// Call exactly once per tick you intend to run, including when nothing was
    /// pressed — "nothing" is a value here, not a silence. Successive calls
    /// schedule successive ticks, starting `delay` ahead of the first, so the
    /// stream of inputs has no gaps regardless of how many ticks the peer
    /// happens to run between calls.
    pub fn send_input(&mut self, turn: Turn) -> Message {
        let tick = self.next_input_tick;
        self.next_input_tick += 1;
        let seat = self.seat;
        self.record(tick, seat, turn);
        Message::Input { tick, turn }
    }

    /// The tick the next [`send_input`](Self::send_input) will apply to.
    pub fn next_input_tick(&self) -> u64 {
        self.next_input_tick
    }

    /// How many ticks ahead this match schedules input.
    ///
    /// Both peers agreed this before the match started and neither can change
    /// it: it is how much network jitter the match can absorb, paid for with
    /// exactly that much input lag.
    pub fn delay(&self) -> u64 {
        self.delay
    }

    /// Accepts a message from the other peer.
    pub fn receive(&mut self, message: Message) {
        match message {
            Message::Input { tick, turn } => {
                let seat = self.seat.other();
                self.record(tick, seat, turn);
            }
            Message::Checksum { tick, digest } => {
                self.remote_checksums.insert(tick, digest);
            }
        }
    }

    /// Messages this peer has produced and not yet handed to a transport.
    pub fn drain_outbox(&mut self) -> Vec<Message> {
        std::mem::take(&mut self.outbox)
    }

    /// Whether every input needed for the next tick has arrived.
    pub fn ready(&self) -> bool {
        self.inputs
            .get(&self.tick)
            .is_some_and(|slots| slots.iter().all(Option::is_some))
    }

    /// Runs one tick if both peers' inputs for it are known.
    ///
    /// Returns `Ok(true)` if a tick ran, `Ok(false)` if it is still waiting on
    /// the other peer, and `Err` if the peers have diverged.
    pub fn try_step(&mut self) -> Result<bool, Desync> {
        if self.is_over() || !self.ready() {
            return Ok(false);
        }

        let slots = self
            .inputs
            .remove(&self.tick)
            .expect("ready() checked this");
        for (index, slot) in slots.iter().enumerate() {
            if let Some(direction) = slot.expect("ready() checked this") {
                self.games[index].queue_turn(direction);
            }
        }
        for game in &mut self.games {
            if !game.is_over() {
                game.step();
            }
        }

        let completed = self.tick;
        self.tick += 1;

        if completed.is_multiple_of(CHECKSUM_INTERVAL) {
            let digest = self.checksum();
            self.local_checksums.insert(completed, digest);
            self.outbox.push(Message::Checksum {
                tick: completed,
                digest,
            });
        }
        self.compare_checksums()?;
        Ok(true)
    }

    /// A digest of both games: the state that must not diverge.
    ///
    /// FNV-1a over everything that feeds the next tick. Score is included even
    /// though it is derivable, because a mismatch there localises the bug
    /// faster than one in the body alone.
    pub fn checksum(&self) -> u64 {
        let mut hash = 0xcbf2_9ce4_8422_2325u64;
        let mut write = |bytes: &[u8]| {
            for byte in bytes {
                hash ^= u64::from(*byte);
                hash = hash.wrapping_mul(0x1000_0000_01b3);
            }
        };
        for game in &self.games {
            write(&game.ticks().to_le_bytes());
            write(&game.score().to_le_bytes());
            write(&[u8::from(game.is_over())]);
            for Coord { x, y } in game.body() {
                write(&x.to_le_bytes());
                write(&y.to_le_bytes());
            }
            if let Some(food) = game.food() {
                write(&food.x.to_le_bytes());
                write(&food.y.to_le_bytes());
            }
        }
        hash
    }

    /// Compares any checksum pairs that have both halves.
    fn compare_checksums(&mut self) -> Result<(), Desync> {
        let ticks: Vec<u64> = self
            .remote_checksums
            .keys()
            .filter(|tick| self.local_checksums.contains_key(tick))
            .copied()
            .collect();
        for tick in ticks {
            let remote = self.remote_checksums.remove(&tick).expect("just listed");
            let local = self.local_checksums.remove(&tick).expect("just filtered");
            if local != remote {
                return Err(Desync::ChecksumMismatch {
                    tick,
                    local,
                    remote,
                });
            }
        }
        Ok(())
    }

    /// Files an input, ignoring a duplicate.
    fn record(&mut self, tick: u64, seat: Seat, turn: Turn) {
        let slots = self.inputs.entry(tick).or_insert([None, None]);
        let slot = &mut slots[seat.index()];
        if slot.is_none() {
            *slot = Some(turn);
        }
    }
}

#[cfg(test)]
mod tests;
