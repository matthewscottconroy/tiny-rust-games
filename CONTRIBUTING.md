# Contributing

Thanks for looking. This repository is a teaching resource first, so the bar for
a change is "would someone learn from reading this?" rather than "does it work?"

## Looking for something to work on

[`docs/PARITY.md`](docs/PARITY.md) lists every concept and which engines
demonstrate it. A concept with a Bevy demo and no Godot one (or the reverse) is
the easiest useful contribution available: the hard part — deciding what is
worth teaching and how to frame it — is already done, and porting it is also the
most direct way to test goal #2. That file is generated, so it is never stale.

## Get set up

```bash
git clone https://github.com/matthewscottconroy/tiny-rust-games
cd tiny-rust-games
just install-hooks     # or: git config core.hooksPath .githooks
```

**Run `just install-hooks` before your first commit.** Git ignores `.githooks/`
until told about it, so a fresh clone has no hooks and you will find out what CI
thinks several minutes later than you needed to.

[`just`](https://github.com/casey/just) is optional but wraps a repository that
genuinely needs three different command shapes:

```bash
just              # list every recipe
just ci           # exactly what CI enforces — run this before opening a PR
just fmt          # reformat everything
just bevy hex-grid    # run one Bevy demo
just godot hex-grid   # build one Godot extension
```

### Linux build dependencies

The Bevy and bracket-lib crates link against system libraries. The Godot demos
need none of it.

```bash
# Debian / Ubuntu
sudo apt-get install -y libasound2-dev libudev-dev libwayland-dev libxkbcommon-dev
# Fedora
sudo dnf install -y alsa-lib-devel systemd-devel wayland-devel libxkbcommon-devel
```

A missing wayland header surfaces as a `wayland-sys` build-script panic reading
`Package 'wayland-client' was not found`. It looks like a Rust error and is not.

## The four goals

Every decision here traces back to the goals in the [README](README.md):

1. transparent and idiomatic, for people reading to learn;
2. portable across game libraries and platforms;
3. extensible, usable as a starter template;
4. core game logic factored into an engine-agnostic library.

**Where 4 fights 1, 1 wins.** That is not a slogan — it is why the 151 tech
demos duplicate some pure functions across engines instead of sharing a crate. A
demo you can read in one sitting teaches better than one that sends you
elsewhere for the interesting half. See
[`tech-demos/bevy/DEMO_ANATOMY.md`](tech-demos/bevy/DEMO_ANATOMY.md#duplication-across-engines-is-deliberate).

Goal 4 is demonstrated properly in [`tic-tac-toe/`](tic-tac-toe/) and
[`snake/`](snake/), where rules genuinely must not diverge between frontends.

## Before you write a demo

Read the anatomy document for the engine you are touching. They are
prescriptive, not advisory:

- [`tech-demos/bevy/DEMO_ANATOMY.md`](tech-demos/bevy/DEMO_ANATOMY.md)
- [`tech-demos/godot/DEMO_ANATOMY.md`](tech-demos/godot/DEMO_ANATOMY.md)

Then copy the matching `_template/` directory rather than starting from scratch.

The short version:

- **one concept per demo** — if it needs a second idea to make sense, that idea
  is its own demo;
- **module-level `//!` rustdoc**, and for Godot demos a `Teaches:` line (a hook
  enforces this);
- **push logic into free `pub fn`s** that name no engine type, so it can be
  tested without starting an engine;
- **a `#[cfg(test)] mod tests`** covering those functions. The one exemption is
  a demo that is pure engine wiring with no logic — `bevy/draw-window` is the
  only current example. If you find yourself writing a test that asserts nothing
  real, say the demo is wiring-only instead of padding it.

## What CI enforces

`just ci` runs all of it locally. Every one of these is a hard failure:

| Check | Notes |
|-------|-------|
| `cargo fmt --check` | Hand-aligned data tables opt out with `#[rustfmt::skip]`. |
| `cargo clippy --all-targets -- -D warnings` | Includes `missing_docs`: every public item needs a `///`. |
| `cargo test` | |
| `--locked` | Never let a build silently rewrite a committed `Cargo.lock`. |
| wasm build | The games must keep compiling for `wasm32-unknown-unknown`. |
| catalogue freshness | Run `just catalogue` after adding a demo. |

The Bevy workspace is additionally built on macOS and Windows.

**The Godot suite is the one place `missing_docs` is not enforced**, because
gdext's `#[export]` generates accessor methods that cannot carry documentation.
Keep docs by hand there.

## Things that have bitten people here

Recorded because they cost real time, and none are guessable:

- **Every Godot crate (the 68 demos plus the games' `-godot` frontends) must pin the same `godot` version.** They are separate
  crates with separate lockfiles; one `cargo update` in one of them is how they
  drifted to three different versions. Use `just sync-godot-locks`.
- **`snake-bevy` and `tic-tac-toe-bevy` live outside `tech-demos/bevy/` but are
  members of that workspace**, so Bevy compiles once for the whole repository.
  Anything that globs `tic-tac-toe/*` or `snake/*` into the "other crates" CI job
  will rebuild Bevy and gdext from scratch.
- **The Godot demos are deliberately not a workspace.** A `.gdextension` resolves
  its library through `res://target/debug/lib*.so`, so the build output has to
  land in that crate's own `target/`. Do not "fix" this by merging them.
- **bracket-lib's transitive `expat-sys`** fails under CMake ≥ 4.0 unless
  `CMAKE_POLICY_VERSION_MINIMUM=3.5` is set. The justfile and CI set it.

## Tests

Unit tests are the baseline. The two game libraries go further, and new game
logic should too:

- **property tests** (`proptest`) for invariants that must hold across every
  input — board size, seed, move sequence. This is where hand-written examples
  stop covering anything.
- **mutation testing** (`cargo mutants -d <crate>`) to check the tests would
  actually fail if the code broke. It has repeatedly found tests that assert
  nothing: a win condition that read `width * height` was checked only on a 2×2
  board, where `width + height` gives the same answer.

Both libraries are currently at zero surviving mutants. Please keep it that way
for code you add there.

## Opening a pull request

- run `just ci` first;
- one logical change per PR — a new demo, or a fix, not both;
- explain **why** in the description. The what is in the diff.

Commit messages here run long on purpose: they record the reasoning and the
dead ends, because that is the part a future reader cannot reconstruct from the
code. Match that if you can, but a clear short message beats a padded long one.

## Licence

MIT. By contributing you agree your work is licensed the same way.
