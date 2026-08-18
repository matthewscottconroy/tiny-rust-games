# tiny-rust-games

Educational example implementations of simple games and engine tech demos in
Rust. The audience is someone reading the code to learn, so **clarity outranks
cleverness everywhere**.

## The four goals (from README.md)

1. Transparent and idiomatic code, for educational use.
2. Portable across game libraries and platforms.
3. Extensible — usable as a starter template.
4. Core game logic factored into an engine-agnostic library. **Where goal 4
   fights goal 1, goal 1 wins.**

Goal 4 is the one most easily violated by accident: if a frontend has to
re-derive a rule the core library should have told it, the rule is in the wrong
place. `tic-tac-toe/` is the reference example for turn-based games.

`breakout/` extends it to continuous physics, and the rule generalises: a
library `step()` advances a **fixed** timestep, never a frame's `dt`. Taking
`dt` makes the simulation frame-rate dependent and destroys reproducibility.
What continuous state adds is interpolated *rendering* — `ball_at(alpha)` — 
which discrete games never need.

`snake/` is the reference for real-time ones, and the rule there is sharper:
**the library owns the rules, never the clock.** `SnakeGame::step()` advances
exactly one tick and never reads a clock, which is what makes it both
engine-agnostic and deterministic enough to property-test. `Ticker` converts
frame time into whole steps so no frontend hand-rolls an accumulator. If you add
a real-time game, copy that split.

## Layout

| Path | What |
|------|------|
| `tech-demos/bevy/` | One Cargo **workspace**, 82 demos + `_template`. Bevy compiles once. |
| `tech-demos/godot/` | 67 **standalone crates** (see below), Rust logic via gdext. |
| `tech-demos/brackets/` | bracket-lib demos. |
| `tic-tac-toe/` | Engine-agnostic `-lib` core with `-cli`, `-brackets`, `-bevy`, `-godot` frontends. |
| `snake/` | The real-time counterpart: `snake-lib` + `-bevy` + `-godot`. Also replays. |
| `breakout/` | Continuous physics on a fixed timestep: `breakout-lib` + `-bevy` + `-godot`. |

Read the conventions doc for the engine you are touching **before** writing a
demo — they are prescriptive, not advisory:

- `tech-demos/bevy/DEMO_ANATOMY.md`
- `tech-demos/godot/DEMO_ANATOMY.md`

## Non-obvious constraints

**`tic-tac-toe-bevy` is a member of the Bevy workspace despite living under
`tic-tac-toe/`.** Its `Cargo.toml` names the root via `workspace = "../../tech-demos/bevy"`
because it sits outside that workspace's directory tree. This keeps Bevy from
being compiled a second time. The consequence for CI and the justfile: the
`misc` job must **not** glob `tic-tac-toe/*` — it lists crates explicitly, or
Bevy and gdext get rebuilt from scratch.

**The Godot demos are deliberately not a workspace.** Each `.gdextension` file
resolves the compiled library through `res://target/debug/lib*.so`, where
`res://` is the crate's own directory. A shared workspace `target/` would put
the binary where Godot cannot find it. Do not "fix" this by merging them.

The cost is 71 separate `Cargo.lock` files — the 67 demos, `_template`, and
the three game frontends (`tic-tac-toe-godot`, `snake-godot`,
`breakout-godot`), which the pre-commit hook checks as a single set. They must all pin the **same**
`godot` version — running `cargo update` in one crate alone is exactly how
they drifted to three different versions once before. Update all of them or
none.

**Bevy demos inherit lints from the workspace.** Every member `Cargo.toml` ends
with `[lints]\nworkspace = true`. A new demo that omits it silently opts out of
`-D warnings`. Three clippy lints are allowed workspace-wide
(`type_complexity`, `too_many_arguments`, `needless_range_loop`) because Bevy
system signatures are wide and grid code reads better with indices; the
rationale is in `tech-demos/bevy/Cargo.toml`. Do not add more allows to dodge a
warning — fix the code.

**`cargo fmt` is authoritative, with one exception.** A few hand-aligned data
tables (recipes, upgrade trees, ability lists) are marked `#[rustfmt::skip]`
because the column alignment is what makes them readable. When a table like that
sits inside a builder chain, extract it into a `#[rustfmt::skip]` function
rather than dropping the alignment — stable Rust does not allow the attribute on
an expression.

## Commands

```bash
# Bevy: one workspace
cd tech-demos/bevy && cargo run -p hello-world
cargo test --workspace --manifest-path tech-demos/bevy/Cargo.toml

# Godot: per crate. Share a target dir when checking many at once.
CARGO_TARGET_DIR=/tmp/godot-shared cargo test --manifest-path tech-demos/godot/hex-grid/Cargo.toml

# Formatting is per manifest — there is no repo-wide `cargo fmt`.
cargo fmt --manifest-path <manifest> --all
```

Prefer the `justfile` when it covers the task: `just ci` runs exactly what CI
runs, `just fmt` reformats everything, `just sync-godot-locks` re-pins the 67
Godot lockfiles after a gdext bump.

**Linux build dependencies.** Bevy and bracket-lib need ALSA, udev, wayland, and
xkbcommon headers. A missing wayland header shows up as a `wayland-sys`
build-script panic (`Package 'wayland-client' was not found`), which reads like
a Rust error but is not. Install commands for Debian and Fedora are in
`README.md`. The Godot demos need none of this.

bracket-lib's transitive `expat-sys` fails under CMake ≥ 4.0 unless
`CMAKE_POLICY_VERSION_MINIMUM=3.5` is set.

## Git hooks

`.githooks/` is only active once `core.hooksPath` points at it — `just
install-hooks`. pre-commit does fast staged-content checks (formatting, build
artifacts, and the repo invariants that have broken before: workspace
membership, gdext version drift, `.gdextension` name match, `Teaches:` lines).
pre-push compiles and tests the affected crates. Keep pre-commit
compile-free — the build is minutes long, and a slow hook just gets bypassed.

**Godot projects must declare `run/main_scene`.** Without it a project hangs
forever under `--headless` rather than erroring — it waits on a UI that never
appears. Twelve projects were missing it. `tools/validate-godot.sh` fails such a
project instead of hanging, and CI runs it.

**Performance claims in docs need a benchmark.** `benchmarks/` measures them.
The first run falsified one: `spatial-partitioning`'s grid is 15x *slower* than
brute force at its default 60 entities. Prefer measuring to asserting.

## Bar for a change

CI enforces all of these on every crate, so run them before claiming done:

- `cargo fmt --check`
- `cargo clippy --all-targets -- -D warnings`
- `cargo test`
- `--locked` — never let a build silently rewrite a committed `Cargo.lock`.

Doc coverage is enforced by `missing_docs` everywhere except the Godot demos,
where gdext's `#[export]` generates accessor methods that cannot be documented.
Godot demos keep docs by hand; their module doc opens with a `Teaches:` line.

Every demo needs module-level `//!` rustdoc and a `#[cfg(test)]` module
exercising its pure functions. The only exemptions are demos that are pure
engine wiring with no logic (`bevy/draw-window`); if you find
yourself writing a ceremonial test that asserts nothing real, the demo probably
belongs in that category — say so rather than padding it.

Per-demo `README.md` files are only for demos whose architecture is not obvious
from the source. Do not add one to every demo; the engine's demo index plus
module rustdoc is the default.
