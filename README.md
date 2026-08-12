# tiny-rust-games
A repository for building very simple games using Rust and its various game engines.

The purpose of this repository is to hold example implementations of some very simple, well-known games. Each game is built with three goals in mind:

1. The code for the game should be transparent and idiomatic so that it can be used for educational purposes.
2. The code should be portable so that it can be used across many game libraries and distributed on many platforms. The code should be implemented in as many game libraries as possible as a proof of concept for each system.
3. The code should be extensible so that others can use it as a starter template for a more complicated game of their own.
4. The core game logic should be factored into its own library and game engine agnostic wherever this heuristic can be sensibly applied so that the same code can be used across multiple engines. Wherever this goal detracts from goal number one, goal number one should take precedence.

## Repository layout

| Path | Contents |
|------|----------|
| [`tech-demos/bevy/`](tech-demos/bevy/) | 82 [Bevy](https://bevyengine.org/) `0.18` demos, each isolating one engine concept or gameplay system. A single Cargo workspace — see the [Bevy demo index](tech-demos/bevy/README.md). |
| [`tech-demos/godot/`](tech-demos/godot/) | 67 [Godot](https://godotengine.org/) `4.3` demos with game logic in Rust via [gdext](https://github.com/godot-rust/gdext) `0.5` — see the [Godot demo index](tech-demos/godot/README.md). |
| [`tech-demos/brackets/`](tech-demos/brackets/) | A [bracket-lib](https://github.com/amethyst/bracket-lib) mouse-control demo. |
| [`tic-tac-toe/`](tic-tac-toe/) | The reference "well-known game": an engine-agnostic `tic-tac-toe-lib` core with **four** front-ends — `-cli`, `-brackets`, `-bevy`, and `-godot` (goals #2 and #4 in practice). |

The same tic-tac-toe rules drive a terminal loop, an ASCII console, Bevy's ECS,
and Godot's scene tree without any frontend containing a rule of the game — that
is goal #4 tested against architectures that genuinely differ, rather than
against two variations on a terminal.

Every demo ships module-level rustdoc, and every demo with logic to exercise
ships a `#[cfg(test)]` test module — the sole exception is `bevy/draw-window`,
ten lines that open a window and contain no logic at all.

Each engine directory documents the shape its demos share:
[Bevy](tech-demos/bevy/DEMO_ANATOMY.md) and
[Godot](tech-demos/godot/DEMO_ANATOMY.md). Read the relevant one before adding a
demo.

Per-demo `README.md` files are reserved for demos whose architecture is not
obvious from the source; everything else is covered by the engine's demo index
plus the module rustdoc.

## Continuous integration

[`.github/workflows/ci.yml`](.github/workflows/ci.yml) runs on every push and
pull request. It enforces, across every crate:

- `cargo fmt --check` — deliberately hand-aligned data tables opt out with
  `#[rustfmt::skip]`;
- `cargo clippy --all-targets -- -D warnings`, including `missing_docs` for
  every crate except the Godot demos (gdext's `#[export]` generates accessors
  that cannot carry docs);
- `cargo test`;
- `--locked`, so the committed `Cargo.lock` files are verified rather than
  silently updated.

## Building

```bash
# Bevy demos (shared workspace — Bevy compiles once):
cd tech-demos/bevy && cargo run -p hello-world

# Godot demos (standalone crates; Godot 4.3+ needed only to *run* them):
cd tech-demos/godot/hello-world && cargo build

# Tic-tac-toe, four ways:
cd tic-tac-toe/tic-tac-toe-cli && cargo run                      # terminal
cargo run --manifest-path tech-demos/bevy/Cargo.toml -p tic-tac-toe-bevy
cd tic-tac-toe/tic-tac-toe-godot && cargo build                  # then open in Godot
```

With [`just`](https://github.com/casey/just) installed, `just` lists every
task — `just ci` reproduces exactly what CI enforces, and `just play` starts the
terminal game.

### Linux prerequisites

The Bevy and bracket-lib demos link against ALSA and udev (audio and gamepad
input) and, through `winit`, against wayland and xkbcommon. The Godot demos need
none of these — gdext bundles its own bindings.

```bash
# Debian / Ubuntu
sudo apt-get install -y libasound2-dev libudev-dev libwayland-dev libxkbcommon-dev

# Fedora
sudo dnf install -y alsa-lib-devel systemd-devel wayland-devel libxkbcommon-devel
```

Missing wayland headers surface as a `wayland-sys` build-script panic reading
`Package 'wayland-client' was not found`, which is easy to mistake for a Rust
problem.

If a bracket-lib demo fails while building its transitive `expat-sys`
dependency under CMake ≥ 4.0, build with `CMAKE_POLICY_VERSION_MINIMUM=3.5`.
