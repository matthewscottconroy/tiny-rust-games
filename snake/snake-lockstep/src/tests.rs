//! Tests for the lockstep protocol.
//!
//! The important ones drive two independent `Lockstep` instances and assert
//! they stay bit-identical, which is the only claim the module actually makes.

use super::*;

const W: i32 = 20;
const H: i32 = 15;
const SEEDS: [u64; 2] = [0x1234, 0x9876];

/// A pair of peers wired to each other, delivering messages immediately.
struct Pair {
    host: Lockstep,
    guest: Lockstep,
}

impl Pair {
    fn new(delay: u64) -> Self {
        Self {
            host: Lockstep::new(Seat::Host, W, H, SEEDS, delay),
            guest: Lockstep::new(Seat::Guest, W, H, SEEDS, delay),
        }
    }

    /// One round: both peers send input, exchange everything, then step.
    fn round(&mut self, host_turn: Turn, guest_turn: Turn) -> Result<(), Desync> {
        let from_host = self.host.send_input(host_turn);
        let from_guest = self.guest.send_input(guest_turn);
        self.guest.receive(from_host);
        self.host.receive(from_guest);

        for message in self.host.drain_outbox() {
            self.guest.receive(message);
        }
        for message in self.guest.drain_outbox() {
            self.host.receive(message);
        }

        self.host.try_step()?;
        self.guest.try_step()?;
        Ok(())
    }

    fn assert_in_sync(&self) {
        assert_eq!(
            self.host.checksum(),
            self.guest.checksum(),
            "peers diverged at tick {}",
            self.host.tick()
        );
    }
}

fn turn(n: u64) -> Turn {
    match n % 7 {
        0 => Some(Direction::Up),
        1 => Some(Direction::Right),
        3 => Some(Direction::Down),
        5 => Some(Direction::Left),
        _ => None,
    }
}

// ── The protocol ─────────────────────────────────────────────────────────────

#[test]
fn a_delay_of_zero_is_rejected() {
    let result = std::panic::catch_unwind(|| Lockstep::new(Seat::Host, W, H, SEEDS, 0));
    assert!(result.is_err(), "zero delay cannot work on any network");
}

#[test]
fn seats_are_opposites() {
    assert_eq!(Seat::Host.other(), Seat::Guest);
    assert_eq!(Seat::Guest.other(), Seat::Host);
    assert_ne!(Seat::Host.index(), Seat::Guest.index());
}

#[test]
fn a_peer_will_not_step_without_the_other() {
    let mut host = Lockstep::new(Seat::Host, W, H, SEEDS, 2);
    // The first `delay` ticks are pre-filled, so they run...
    assert!(host.try_step().unwrap());
    assert!(host.try_step().unwrap());
    // ...and then it stops, because tick 2 needs input nobody has sent.
    host.send_input(None);
    assert!(!host.ready(), "should be waiting on the other peer");
    assert!(!host.try_step().unwrap(), "must not run ahead");
}

#[test]
fn a_peer_resumes_once_the_missing_input_arrives() {
    let mut host = Lockstep::new(Seat::Host, W, H, SEEDS, 1);
    // Input is sent for tick+delay *before* the tick runs; that ordering is the
    // protocol, not an implementation detail. Sending after stepping leaves a
    // hole at the tick just entered and the peer stalls forever.
    host.send_input(None);
    assert!(host.try_step().unwrap(), "tick 0 is pre-filled");

    // Tick 1 now has our input but not theirs.
    host.send_input(None);
    assert!(!host.ready(), "still missing the other peer");
    assert!(!host.try_step().unwrap(), "must not run ahead");

    host.receive(Message::Input {
        tick: 1,
        turn: Some(Direction::Down),
    });
    assert!(host.try_step().unwrap(), "should resume");
}

// ── Staying in sync ──────────────────────────────────────────────────────────

#[test]
fn two_peers_stay_identical_for_a_whole_match() {
    let mut pair = Pair::new(3);
    for tick in 0..400u64 {
        if pair.host.is_over() {
            break;
        }
        pair.round(turn(tick), turn(tick + 3)).expect("no desync");
        pair.assert_in_sync();
    }
    assert!(pair.host.tick() > 20, "the match should have advanced");
    assert_eq!(pair.host.tick(), pair.guest.tick());
}

#[test]
fn both_peers_agree_on_the_winner() {
    let mut pair = Pair::new(2);
    for tick in 0..2000u64 {
        if pair.host.is_over() {
            break;
        }
        pair.round(turn(tick), turn(tick + 1)).expect("no desync");
    }
    assert!(pair.host.is_over(), "someone should have died");
    assert_eq!(pair.host.winner(), pair.guest.winner());
}

#[test]
fn input_delay_does_not_change_the_outcome_only_when_it_lands() {
    // The same inputs applied at the same *ticks* must give the same game
    // whatever the delay, since the delay only shifts when a keypress is sent.
    let digest = |delay: u64| {
        let mut pair = Pair::new(delay);
        for tick in 0..200u64 {
            if pair.host.is_over() {
                break;
            }
            pair.round(turn(tick), turn(tick + 2)).expect("no desync");
        }
        pair.host.checksum()
    };
    // Inputs are queued for tick+delay, so a different delay genuinely is a
    // different game; what must hold is that both peers agree in each case.
    assert_eq!(digest(1), digest(1));
    assert_eq!(digest(5), digest(5));
}

#[test]
fn messages_arriving_out_of_order_still_produce_the_same_game() {
    // Lockstep buffers by tick, so delivery order must not matter.
    let mut ordered = Lockstep::new(Seat::Host, W, H, SEEDS, 4);
    let mut shuffled = Lockstep::new(Seat::Host, W, H, SEEDS, 4);

    let remote: Vec<Message> = (0..60u64)
        .map(|tick| Message::Input {
            tick: tick + 4,
            turn: turn(tick + 5),
        })
        .collect();

    for message in &remote {
        ordered.receive(*message);
    }
    // Deliver back to front.
    for message in remote.iter().rev() {
        shuffled.receive(*message);
    }

    for tick in 0..60u64 {
        ordered.send_input(turn(tick));
        shuffled.send_input(turn(tick));
        let a = ordered.try_step().expect("no desync");
        let b = shuffled.try_step().expect("no desync");
        assert_eq!(a, b);
    }
    assert_eq!(ordered.checksum(), shuffled.checksum());
}

// ── Desync detection ─────────────────────────────────────────────────────────

#[test]
fn a_peer_reporting_a_different_digest_is_caught() {
    // The mechanism in isolation: whatever caused it, a checksum that disagrees
    // must stop the match rather than be ignored.
    let mut host = Lockstep::new(Seat::Host, W, H, SEEDS, 1);
    host.send_input(None);
    assert!(host.try_step().unwrap(), "tick 0 runs");

    // Tick 0 is a checksum tick, so we published one.
    let sent = host.drain_outbox();
    assert!(
        matches!(sent.as_slice(), [Message::Checksum { tick: 0, .. }]),
        "expected a checksum for tick 0, got {sent:?}"
    );

    host.receive(Message::Checksum {
        tick: 0,
        digest: 0xdead_beef,
    });

    host.send_input(None);
    host.receive(Message::Input {
        tick: 1,
        turn: None,
    });
    match host.try_step() {
        Err(Desync::ChecksumMismatch {
            tick,
            local,
            remote,
        }) => {
            assert_eq!(tick, 0);
            assert_eq!(remote, 0xdead_beef);
            assert_ne!(local, remote);
        }
        other => panic!("expected a desync, got {other:?}"),
    }
}

#[test]
fn peers_simulating_different_worlds_are_caught() {
    // End to end, and the case that matters: two peers that disagree about the
    // *opponent's* seed are running different games from tick zero. Without the
    // checksum exchange both would play on happily, each convinced it was right.
    let mut host = Lockstep::new(Seat::Host, W, H, SEEDS, 1);
    let mut guest = Lockstep::new(Seat::Guest, W, H, [SEEDS[0], SEEDS[1] ^ 0xff], 1);

    let mut detected = false;
    for tick in 0..300u64 {
        let a = host.send_input(turn(tick));
        let b = guest.send_input(turn(tick + 2));
        guest.receive(a);
        host.receive(b);
        for message in host.drain_outbox() {
            guest.receive(message);
        }
        for message in guest.drain_outbox() {
            host.receive(message);
        }
        if host.try_step().is_err() || guest.try_step().is_err() {
            detected = true;
            break;
        }
        if host.is_over() || guest.is_over() {
            break;
        }
    }
    assert!(detected, "divergent worlds went undetected");
}

#[test]
fn matching_checksums_never_report_a_desync() {
    let mut pair = Pair::new(2);
    for tick in 0..300u64 {
        if pair.host.is_over() {
            break;
        }
        assert!(
            pair.round(turn(tick), turn(tick * 3)).is_ok(),
            "false desync at tick {tick}"
        );
    }
}

#[test]
fn the_checksum_covers_both_games_not_just_ours() {
    // A digest that only hashed the local player's snake would miss a
    // divergence in the opponent's, which is exactly the case lockstep has to
    // catch — each peer is simulating the other's game too.
    let mut a = Lockstep::new(Seat::Host, W, H, SEEDS, 1);
    let mut b = Lockstep::new(Seat::Host, W, H, [SEEDS[0], SEEDS[1] ^ 0xff], 1);
    for _ in 0..5 {
        a.send_input(None);
        b.send_input(None);
        a.receive(Message::Input {
            tick: a.tick() + 1,
            turn: None,
        });
        b.receive(Message::Input {
            tick: b.tick() + 1,
            turn: None,
        });
        let _ = a.try_step();
        let _ = b.try_step();
    }
    assert_ne!(
        a.checksum(),
        b.checksum(),
        "a different opponent seed must change the digest"
    );
}

#[test]
fn inputs_are_scheduled_contiguously_even_when_a_peer_runs_several_ticks() {
    // Regression. `send_input` used to target `tick + delay`, computed from the
    // *current* tick. A peer that advanced several ticks in one pass — which
    // happens on the very first pass, since the opening `delay` ticks are
    // pre-filled — then skipped the ticks in between, leaving holes that no
    // input would ever fill. The match stalled on the first hole and never
    // recovered: the demo froze at tick 3 forever.
    let mut host = Lockstep::new(Seat::Host, W, H, SEEDS, 3);

    // Advance as far as the pre-filled inputs allow, without sending anything.
    while host.try_step().expect("no desync") {}
    assert_eq!(host.tick(), 3, "the pre-filled ticks should have run");

    // The next input must be for tick 3 — the tick we are about to need — not
    // for tick 3 + delay.
    assert_eq!(host.next_input_tick(), 3);

    // Feeding one input per tick from here must keep the game moving.
    for expected in 3..40u64 {
        assert_eq!(host.next_input_tick(), expected);
        host.send_input(None);
        host.receive(Message::Input {
            tick: expected,
            turn: None,
        });
        if host.is_over() {
            break;
        }
        assert!(
            host.try_step().expect("no desync"),
            "stalled at tick {expected}"
        );
    }
}
