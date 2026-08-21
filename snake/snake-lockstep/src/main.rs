//! Runs a lockstep match between two peers and reports whether they agreed.
//!
//! Both peers run in this one process, connected by a queue that can be given
//! latency. That is not a shortcut: the peers exchange nothing but [`Message`]
//! values, so a socket is a different `send` and nothing else. Keeping it
//! in-process is what makes the demo deterministic enough to assert on, and a
//! networking layer would only add ways for the demo to fail that have nothing
//! to do with lockstep.
//!
//! ```text
//! cargo run -p snake-lockstep                 # a clean match
//! cargo run -p snake-lockstep -- --latency 8  # eight ticks of one-way delay
//! cargo run -p snake-lockstep -- --desync     # corrupt a peer; watch it caught
//! ```

use std::collections::VecDeque;

use snake_lockstep::{Desync, Direction, Lockstep, Message, Seat, Turn};

/// A one-way link that holds each message for a fixed number of ticks.
struct Link {
    latency: u64,
    queue: VecDeque<(u64, Message)>,
}

impl Link {
    fn new(latency: u64) -> Self {
        Self {
            latency,
            queue: VecDeque::new(),
        }
    }

    fn send(&mut self, now: u64, message: Message) {
        self.queue.push_back((now + self.latency, message));
    }

    /// Everything due by `now`.
    fn deliver(&mut self, now: u64) -> Vec<Message> {
        let mut out = Vec::new();
        while let Some((due, _)) = self.queue.front() {
            if *due > now {
                break;
            }
            out.push(self.queue.pop_front().expect("just peeked").1);
        }
        out
    }
}

/// A deterministic stand-in for a player pressing keys.
fn scripted_turn(seed: u64, round: u64) -> Turn {
    match (seed.wrapping_mul(2_654_435_761).wrapping_add(round * 31)) % 11 {
        0 => Some(Direction::Up),
        2 => Some(Direction::Right),
        5 => Some(Direction::Down),
        8 => Some(Direction::Left),
        _ => None,
    }
}

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let desync = args.iter().any(|a| a == "--desync");
    let latency = args
        .windows(2)
        .find(|w| w[0] == "--latency")
        .and_then(|w| w[1].parse::<u64>().ok())
        .unwrap_or(0);

    // The input delay has to cover the round trip, or a peer stalls waiting for
    // a message still in flight. This is the whole tuning problem of lockstep in
    // one line: more delay absorbs more jitter and costs more input lag.
    let delay = (latency + 1).max(2);

    let seeds = [0xC0FF_EE00_1234_5678, 0x5EED_0000_9876_5432];
    let mut host = Lockstep::new(Seat::Host, 24, 18, seeds, delay);
    let mut guest = Lockstep::new(Seat::Guest, 24, 18, seeds, delay);

    let mut to_guest = Link::new(latency);
    let mut to_host = Link::new(latency);

    println!("lockstep snake: latency {latency} tick(s), input delay {delay} tick(s)");
    if desync {
        println!("--desync: the guest will be fed an input the host never sent");
    }
    println!();

    let mut round = 0u64;
    let outcome = loop {
        if round > 5_000 {
            break "stopped after 5000 rounds";
        }
        if host.is_over() || guest.is_over() {
            break "a snake died";
        }

        let now = round;
        to_guest.send(now, host.send_input(scripted_turn(1, round)));
        to_host.send(now, guest.send_input(scripted_turn(2, round)));

        for message in host.drain_outbox() {
            to_guest.send(now, message);
        }
        for message in guest.drain_outbox() {
            to_host.send(now, message);
        }

        for message in to_guest.deliver(now) {
            guest.receive(message);
        }
        for message in to_host.deliver(now) {
            host.receive(message);
        }

        // Corrupt one peer, to show the checksum exchange earning its keep.
        //
        // This causes a *real* divergence rather than faking a bad digest: the
        // guest is told the host turned up on tick 20, before the host's actual
        // input for that tick arrives. Inputs are first-writer-wins, so the real
        // one is then ignored and the guest simulates a host snake that turned
        // when the host's own copy did not. Neither peer can tell locally — both
        // are running a perfectly consistent game — which is precisely why the
        // checksums are exchanged.
        if desync && round == 5 {
            // A rotating sequence, each turn perpendicular to the one before,
            // so none is refused as a reversal — queueing the direction the
            // snake is already travelling would change nothing and the "desync"
            // demo would quietly show no desync at all.
            for (tick, turn) in [
                (20, Direction::Left),
                (22, Direction::Down),
                (23, Direction::Right),
                (24, Direction::Up),
            ] {
                guest.receive(Message::Input {
                    tick,
                    turn: Some(turn),
                });
            }
        }

        for (name, peer) in [("host", &mut host), ("guest", &mut guest)] {
            loop {
                match peer.try_step() {
                    Ok(true) => continue,
                    Ok(false) => break,
                    Err(desync) => {
                        let Desync::ChecksumMismatch {
                            tick,
                            local,
                            remote,
                        } = desync;
                        println!(
                            "DESYNC detected by {name} at tick {tick}:\n  \
                             local  {local:016x}\n  remote {remote:016x}"
                        );
                        break;
                    }
                }
            }
        }

        if round.is_multiple_of(60) {
            println!(
                "  tick {:>4}  host {:>2} pts   guest {:>2} pts   checksums {}",
                host.tick(),
                host.game(Seat::Host).score(),
                host.game(Seat::Guest).score(),
                if host.checksum() == guest.checksum() {
                    "agree"
                } else {
                    "DIFFER"
                }
            );
        }
        round += 1;
    };

    println!();
    println!("match ended: {outcome}");
    println!("  host  reached tick {}", host.tick());
    println!("  guest reached tick {}", guest.tick());
    match host.winner() {
        Some(seat) => println!("  winner: {seat:?}"),
        None => println!("  winner: none (draw, or still running)"),
    }
    println!(
        "  final checksums: {}",
        if host.checksum() == guest.checksum() {
            "identical"
        } else {
            "DIFFERENT — the peers diverged"
        }
    );
}

#[cfg(test)]
mod tests {
    use super::*;

    // `Link` and `scripted_turn` are logic, not wiring, and mutation testing
    // found every one of their mutants surviving — the binary had no tests at
    // all. A demo harness still gets to be correct.

    #[test]
    fn a_link_with_no_latency_delivers_at_once() {
        let mut link = Link::new(0);
        link.send(
            0,
            Message::Input {
                tick: 1,
                turn: None,
            },
        );
        assert_eq!(link.deliver(0).len(), 1);
        assert!(
            link.deliver(0).is_empty(),
            "delivered messages are consumed"
        );
    }

    #[test]
    fn a_link_holds_a_message_for_exactly_its_latency() {
        let mut link = Link::new(3);
        link.send(
            10,
            Message::Input {
                tick: 1,
                turn: None,
            },
        );
        for now in 10..13 {
            assert!(link.deliver(now).is_empty(), "too early at {now}");
        }
        assert_eq!(link.deliver(13).len(), 1, "due at 13");
    }

    #[test]
    fn a_link_delivers_everything_already_due() {
        let mut link = Link::new(2);
        for tick in 0..5 {
            link.send(tick, Message::Input { tick, turn: None });
        }
        // By tick 4, the ones sent at 0, 1 and 2 are due.
        assert_eq!(link.deliver(4).len(), 3);
    }

    #[test]
    fn a_link_preserves_order() {
        let mut link = Link::new(1);
        for tick in 0..4 {
            link.send(0, Message::Input { tick, turn: None });
        }
        let ticks: Vec<u64> = link
            .deliver(1)
            .into_iter()
            .map(|m| match m {
                Message::Input { tick, .. } => tick,
                Message::Checksum { tick, .. } => tick,
            })
            .collect();
        assert_eq!(ticks, vec![0, 1, 2, 3]);
    }

    #[test]
    fn the_script_is_a_pure_function_of_seed_and_round() {
        // Both peers derive their inputs from this, so it has to give the same
        // answer every time or the demo would desync for the wrong reason.
        for round in 0..50 {
            assert_eq!(scripted_turn(1, round), scripted_turn(1, round));
        }
    }

    #[test]
    fn the_script_reaches_every_direction_and_also_stands_still() {
        let seen: Vec<Turn> = (0..200).map(|r| scripted_turn(1, r)).collect();
        for direction in [
            Direction::Up,
            Direction::Down,
            Direction::Left,
            Direction::Right,
        ] {
            assert!(
                seen.contains(&Some(direction)),
                "{direction:?} never came up"
            );
        }
        assert!(seen.contains(&None), "the script never holds its course");
    }

    #[test]
    fn different_seeds_give_different_scripts() {
        let a: Vec<Turn> = (0..40).map(|r| scripted_turn(1, r)).collect();
        let b: Vec<Turn> = (0..40).map(|r| scripted_turn(2, r)).collect();
        assert_ne!(a, b, "both players would press the same keys");
    }
}
