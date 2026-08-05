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
| [`tic-tac-toe/`](tic-tac-toe/) | The reference "well-known game": an engine-agnostic `tic-tac-toe-lib` core with `-cli` and `-brackets` front-ends (goal #4 in practice). |

Every demo ships module-level rustdoc and a `#[cfg(test)]` test module. Continuous
integration builds and tests all demos on each push — see
[`.github/workflows/ci.yml`](.github/workflows/ci.yml).

## Building

```bash
# Bevy demos (shared workspace — Bevy compiles once):
cd tech-demos/bevy && cargo run -p hello-world

# Godot demos (standalone crates; Godot 4.3+ needed only to *run* them):
cd tech-demos/godot/hello-world && cargo build
```

Building the Bevy demos on Linux needs the ALSA and udev headers:
`sudo apt-get install -y libasound2-dev libudev-dev`.
