# Snake

Snake implemented once as an engine-agnostic library, with a frontend per
engine. It is the repository's second demonstration of goal #4 — and it exists
because [`tic-tac-toe/`](../tic-tac-toe/) could not demonstrate the hard half.

| Crate | What it is |
|-------|------------|
| `snake-lib` | The rules: movement, growth, food, collision, win/loss. No engine dependencies, no clock. |
| `snake-bevy` | The game in [Bevy](https://bevyengine.org/), stepped from an ECS system. |
| `snake-godot` | The game in [Godot](https://godotengine.org/), stepped from `process(delta)`. |

## Why a second game

Tic-tac-toe is turn-based. A frontend calls into the library when the *player*
acts, and nothing happens in between. That proves a boundary, but a narrow one —
every frontend of it is a variation on "read input, call a method, draw".

Snake is real-time: the world moves whether or not anyone touches the keyboard.
Something has to own the clock, and *which* something is the entire design
question. Get it wrong and the rules end up smeared across every frontend,
because each engine's update loop looks different.

## The answer: the library never owns time

`SnakeGame::step()` advances the world by exactly one tick. It never sleeps,
never reads a clock, and never asks how long a frame took.

That single constraint buys two things at once. The library stays engine-
agnostic, because it has no opinion about frames. And it becomes *deterministic*
— the same seed and the same sequence of steps produce the same game in a test
as in a window, which is why this crate can be property-tested at all.

Converting real elapsed time into a whole number of steps is the frontend's job.
But it is the *same* job in every frontend, so the library ships it as `Ticker`:

```rust
for _ in 0..ticker.accumulate(delta_seconds) {
    game.step();
}
```

Those three lines are the entire difference between the Bevy frontend and the
Godot one. Bevy passes `Time::delta_secs()` from a system; Godot passes the
`delta` argument of `process`. Neither contains a rule of the game, and neither
can disagree with the other about how fast the snake moves, because neither one
decides.

`Ticker` carries its remainder forward rather than discarding it each frame, so
sixty short frames and six long ones covering the same second do the same amount
of work. It also caps how many steps one call may return, so a breakpoint or a
dragged window cannot bank two hundred steps and teleport the snake.

> Agreement across frame rates is within one step, not bit-for-bit: frame deltas
> are `f32` and most are not exactly representable, so a hundred frames of `0.01`
> sum to `0.99999998`. One step at nine steps per second is imperceptible — but
> do not build a lockstep network protocol on it.

## The other real-time problem: input outruns ticks

At nine steps per second a player can easily press two keys inside one tick.
Applying each immediately lets *up* then *left* — both individually legal —
turn the snake straight back into its own neck.

So `queue_turn` records the intent and `step` commits it, validating against the
direction actually **travelled** rather than the last one requested. Every
frontend gets that protection for free by calling `queue_turn` and ignoring the
answer; none of them knows what the rule is.

## Playing

```bash
just snake                                                  # Bevy window
cargo run --manifest-path ../tech-demos/bevy/Cargo.toml -p snake-bevy

cd snake-godot && cargo build && godot4 --editor .          # Godot
```

`snake-bevy` is a member of the Bevy demo workspace so Bevy compiles once for
the whole repository, which is why it is run through that manifest.

**Controls:** arrows or WASD to steer, R to restart.

## Testing

```bash
cargo test --manifest-path snake-lib/Cargo.toml     # rules + Ticker + properties
cargo test --manifest-path snake-godot/Cargo.toml   # layout, key mapping
cargo test --manifest-path ../tech-demos/bevy/Cargo.toml -p snake-bevy
```

`snake-lib` carries three layers, because the rules here are subtler than they
look:

- **unit tests** pin specific scenarios — the tail-follow case, the two-turns-
  in-one-tick reversal, the frame-rate independence of `Ticker`;
- **property tests** (`tests/properties.rs`) assert invariants across every
  board size, seed, and input sequence: the snake never overlaps itself, length
  always equals score plus one, food is never under the body, an ended game is
  frozen;
- **mutation testing** checks that those tests would actually fail if the code
  broke. `cargo mutants -d snake-lib` found four surviving mutants on the first
  run, including a win condition where `width * height` could be replaced by
  `width + height` undetected — because the only win test used a 2×2 board,
  where both are 4. The tests that kill each mutant are marked in `src/tests.rs`.
