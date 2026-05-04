# Godot + Rust Tech Demos

Each demo is a standalone Godot 4.3+ project with its game logic written in Rust
via the [gdext](https://github.com/godot-rust/gdext) GDExtension binding (`godot = "0.5"`).

## How it works

Godot loads compiled Rust code as a native extension (`.so`/`.dll`/`.dylib`).
Rust classes annotated with `#[derive(GodotClass)]` appear inside the Godot editor
exactly like built-in node types — they show up in the inspector, accept signals, and
integrate with the scene tree with zero GDScript glue.

## Running a demo

```bash
# 1. Build the Rust library (Godot not needed for this step)
cd <demo-name>
cargo build

# 2. Open the project in Godot 4.3+
godot4 --editor .
# or open Godot, click "Import", select the project.godot file
```

Godot will automatically find the compiled `.so`/`.dll` via the `.gdextension` file
and register all Rust classes so you can use them in scenes.

## Demos

| Demo | Concept |
|------|---------|
| `hello-world` | Minimal extension setup — `Node2D` subclass, `godot_print!` |
| `exported-properties` | `#[export]` fields visible in the Godot inspector |
| `player-movement` | `CharacterBody2D` with WASD movement and gravity |
| `signals` | Custom `#[signal]` declarations and `emit_signal` |
| `custom-resource` | `Resource` subclass for game-data containers |
| `scene-tree` | Traversing the scene tree and `Gd<T>` smart pointers |
| `groups` | Node groups: adding, querying, and calling across groups |
| `spawner` | Dynamically instantiating nodes at runtime from Rust |

## Project structure

Each demo follows this layout:

```
demo-name/
├── Cargo.toml               # Rust lib crate (crate-type = ["cdylib", "lib"])
├── src/
│   └── lib.rs               # GDExtension classes + pure-function helpers
├── project.godot            # Godot project file (Godot 4.3)
├── demo-name.gdextension    # Tells Godot where to find the compiled library
└── scenes/
    └── main.tscn            # Entry-point scene referencing the Rust class
```

## Testing

Each demo extracts pure (non-Godot) functions and tests them with `cargo test`.
The Godot types themselves require a running Godot instance and are not unit-testable.

```bash
cd hello-world && cargo test
```

## Requirements

- Rust stable (`cargo build` works without Godot)
- Godot 4.3 or later to actually run the demos
