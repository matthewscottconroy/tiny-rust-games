# Breakout

Breakout implemented once as an engine-agnostic library, with a Bevy frontend.
It is the third demonstration of goal #4, and it was chosen because it looked
like the one that would break the pattern.

| Crate | What it is |
|-------|------------|
| `breakout-lib` | The rules: continuous ball physics, paddle, brick collision, lives. No dependencies, no clock. |
| `breakout-bevy` | The game in [Bevy](https://bevyengine.org/), stepped from `FixedUpdate`. |

## Why this game

Tic-tac-toe is turn-based; Snake moves on a grid. Both have state that is
exactly representable and advances in whole units, so keeping the rules out of
the engine was never really under threat.

Breakout has a ball at a floating-point position moving at a floating-point
velocity, bouncing off things. Both Bevy and Godot ship physics engines, and the
obvious move is to use one — at which point the rules live in the engine and
goal #4 is finished.

## The pattern holds, on one condition

`BreakoutGame::step()` advances the world by a **fixed** `DT`, never by a
frame's elapsed time.

Taking `dt` as an argument is the obvious API and is a trap. It makes the
simulation frame-rate dependent — a ball that crosses a brick in one 8 ms step
tunnels straight through it in one 40 ms step — and it destroys reproducibility,
because the physics then depends on how busy the machine was.

With a fixed step the library is deterministic in the strong sense. The
determinism test asserts two runs are *bit-identical*, not merely close, and it
passes: this game uses only `+`, `-`, `*`, `/` and `sqrt`, all of which IEEE-754
specifies exactly, applied in a fixed order.

## What continuous motion actually added

The pattern held. Something new did appear, and it is the real finding here:
**interpolation**.

Snake draws its simulation state directly, because a snake is either in a cell
or it is not. A ball simulated at 120 Hz and drawn at 144 Hz judders visibly if
you draw the last simulated position — the ball's steps and the monitor's
refreshes do not line up.

So `breakout-lib` keeps the previous ball position alongside the current one and
exposes `ball_at(alpha)`, which blends between them. The frontend passes how far
it currently sits between two fixed steps. **Rendering interpolates; the
simulation never does.**

That is the boundary this game found. Not "physics cannot be engine-agnostic",
but "continuous state needs a rendering-side concept that discrete state does
not". `Ticker::alpha()` existed in `snake-lib` from the start and was never
needed; here it is essential.

## Two ways to drive a fixed timestep

Snake's frontends hand-roll the accumulator, because `snake_lib::Ticker` has to
work from a terminal loop and a Godot callback as well as from Bevy.

`breakout-bevy` deliberately does the opposite: Bevy already ships a
fixed-timestep scheduler, so `step()` is called from `FixedUpdate` and the
interpolation fraction comes from `Time<Fixed>::overstep_fraction()`.

Same design — fixed simulation, interpolated rendering — reached through the
engine's machinery instead of the library's. The library does not care which,
which is the point: it exposes a step function and a way to interpolate, and
stays out of the argument about who owns the clock.

## Playing

```bash
just breakout
cargo run --manifest-path ../tech-demos/bevy/Cargo.toml -p breakout-bevy
```

**Controls:** left/right or A/D to move, Space to launch, R to restart.

## Testing

```bash
cargo test --manifest-path breakout-lib/Cargo.toml
cargo test --manifest-path ../tech-demos/bevy/Cargo.toml -p breakout-bevy
```

Collision response is tested by *placing* the ball rather than by playing.
Landing a ball on a specific brick face by choosing a launch angle would be a
coincidence, not a test, so there is a `#[cfg(test)]` helper that puts the ball
exactly where a case needs it — kept test-only so the public API stays honest
about what a player can do.

Mutation testing (`cargo mutants -d breakout-lib`) catches 201 of 225 viable
mutants. The first run caught 47 survivors and was worth every minute: the whole
of `collide_bricks` — the most intricate code in the game — turned out to be
reachable only through full games that never asserted *how* the ball bounced.
The 24 that remain are exact-boundary comparisons and layout constants whose
mutations are physically indistinguishable without pinning float literals, which
would trade readability for a number.
