# Breakout

Breakout implemented once as an engine-agnostic library, with Bevy and Godot
frontends. It is the third demonstration of goal #4, and it was chosen because
it looked like the one that would break the pattern.

| Crate | What it is |
|-------|------------|
| `breakout-lib` | The rules: continuous ball physics, paddle, brick collision, lives. No dependencies, no clock. |
| `breakout-bevy` | The game in [Bevy](https://bevyengine.org/), stepped from `FixedUpdate`. |
| `breakout-godot` | The game in [Godot](https://godotengine.org/), stepped from `_physics_process`. |

## Why this game

Tic-tac-toe is turn-based; Snake moves on a grid. Both have state that is
exactly representable and advances in whole units, so keeping the rules out of
the engine was never really under threat.

Breakout has a ball at a floating-point position moving at a floating-point
velocity, bouncing off things. Both Bevy and Godot ship physics engines, and the
obvious move is to use one — at which point the rules live in the engine and
goal #4 is finished.

The Godot frontend is where that temptation is strongest, because Godot's
physics is not a library you opt into but part of the scene tree you are already
using: `CharacterBody2D`, collision shapes, a solver, all of it one node away.
`breakout-godot` declines all of it and uses Godot for a window, a draw call and
an input event. If it had not, the two frontends would be free to disagree about
how a ball bounces, and neither could be called the game.

## Two engines, the same shape

Neither frontend hand-rolls an accumulator, because both engines already ship a
fixed-timestep scheduler. Each is simply told to run at the library's rate:

| | Bevy | Godot |
|---|---|---|
| fixed step | `FixedUpdate` + `Time<Fixed>::from_hz` | `_physics_process` + `physics_ticks_per_second` |
| interpolation | `Time<Fixed>::overstep_fraction()` | `Engine::get_physics_interpolation_fraction()` |
| y axis | up and centred — needs a flip | down from the top-left — matches the library |

The y-axis row is the only place the two genuinely differ, and it is a rendering
detail rather than a rule. That is the whole argument in one table: what changes
between engines is how you draw and how you are called, not what the game *is*.

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

Mutation testing (`cargo mutants -d breakout-lib`) catches 208 of 225 viable
mutants. The first run caught 47 survivors and was worth every minute: the whole
of `collide_bricks` — the most intricate code in the game — turned out to be
reachable only through full games that never asserted *how* the ball bounced.

A later run found two more that were worth fixing, both in the paddle bounce,
and both invisible to tests that looked correct:

- The steering test proved the ball went the right *way* but never the right
  *amount*, so replacing the division by the paddle half-width with a
  multiplication survived: every hit more than a fiftieth of a pixel off centre
  then clamps to full deflection. The ball still went left when struck left, and
  the entire feel of the game was gone.
- Scaling the deflection by `speed + 0.75` instead of `speed * 0.75` survived a
  ratio test, because scaling both offsets equally preserves their ratio. Its
  consequence is real: the sideways component can then exceed the ball's total
  speed, the vertical component collapses to its floor, and the ball leaves the
  paddle almost horizontally and never returns. Catching it took an absolute
  bound rather than a relative one.

The 17 that remain are exact-boundary comparisons (`<` against `<=`, where the
two differ only when a float lands precisely on an edge) and `default_layout`
constants, whose mutations are physically indistinguishable without pinning
float literals — which would trade readability for a number.

## Is it deterministic across machines?

The library steps a fixed timestep and reads no clock, so the same inputs give
the same game — on one machine. That is what
`identical_input_produces_bit_identical_physics` asserts, and it is the weaker
half of the claim. Every position and velocity here is `f32`, and floating point
is permitted to differ between targets once a compiler contracts a multiply-add
or picks a different instruction sequence, so "reproducible" needs testing
*across* machines before it can be relied on for replays or lockstep.

`cargo run -p breakout-lib --example state-hash` plays 20 000 steps under a
fixed policy and prints a digest of the final state, hashing the *bit patterns*
of the floats rather than their printed values. `snake-lib` has the same probe,
as the integer-only control: if the two ever disagree in different ways, that
separates a floating-point property from a plain bug.

CI runs both on Linux, macOS and Windows and fails if the three disagree. So
far they do not, and the digest is also unchanged between debug and release —
20 000 steps of physics through two very different optimisation pipelines. That
is expected rather than lucky: Rust does not enable fast-math or contract
multiply-adds by default, so `f32` arithmetic follows IEEE-754 exactly.

If that ever stops holding on some target, the honest response is to document
that Breakout replays are portable within a target rather than to force the
digests to agree — the same principle that made the `spatial-partitioning`
benchmark worth running when it falsified its own demo's documentation.

## Colour carries meaning, so it is measured

A brick with two hits left is drawn at full brightness and one with a single hit
at half. That difference is the only cue that a brick will survive the next ball,
so it has to be visible at a glance rather than on inspection.

It was not. The dimming was 12%, which the docs described as "dimmed, so the
player can see it is weakened" — about 6 units of CIE ΔE, which essentially
nobody can see on a moving target, colour vision or not. Half brightness is
ΔE 26 across every row.

`tools/check-palette.py` measures every colour pair that carries meaning, under
normal vision and under simulated protanopia, deuteranopia and tritanopia, and
CI fails if one drops below ΔE 25. Snake failed it worse: its food was red on a
green snake, ΔE 98 to ordinary vision and **10.9** under deuteranopia, so the
food was invisible to roughly one man in twelve. It is magenta now.

## Sound, and what it proved

`breakout-bevy` plays a note for each thing that happens: a wall, the paddle, a
brick chipped, a brick broken, a life lost. **`breakout-lib` did not change by a
line to allow it.**

That is the test of an engine-agnostic boundary worth having. Anyone can draw a
boundary that survives the features it was drawn around; this one survived a
feature nobody had in mind when it was drawn, because `step()` already returned
a `StepOutcome` naming exactly what happened. The frontend decides what a bounce
*sounds* like — an effect — while what counts as a bounce stays a rule.

Two details are worth copying:

- **The sound comes from the library's report, not from the state.** `advance`
  forwards the `StepOutcome` it received; nothing re-derives "did that hit a
  brick?" by diffing positions. Re-deriving is how a frontend ends up with its
  own quietly different idea of the rules.
- **Tones are synthesised, not loaded.** A sine wave with a linear fade is a few
  lines, and it means no asset files to ship, to license, or to fail to fetch in
  a browser. The fade is not decoration: without it the sample stops mid-wave
  and clicks, which a test asserts.

The ordering is a design decision rather than something the engine imposes, so
it is tested: a step that both breaks a brick and loses a life plays the *loss*,
because that is the one the player must hear.
