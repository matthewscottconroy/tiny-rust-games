# Anatomy of a Godot + Rust demo

Every demo in `tech-demos/godot/` follows one shape, so that reading a second
demo costs almost nothing once you have read a first. This is the Godot
counterpart to [`../bevy/DEMO_ANATOMY.md`](../bevy/DEMO_ANATOMY.md); the goals
are the same, but the constraints Godot imposes make the layout different.

## The shape

```
demo-name/
├── Cargo.toml            # edition 2024; godot = "0.5"; crate-type cdylib+lib
├── demo-name.gdextension # tells Godot where the compiled library lives
├── project.godot         # the Godot project (config/features = 4.3)
├── scenes/
│   └── main.tscn         # the scene that hosts the Rust node
└── src/
    └── lib.rs            # everything: extension entry, node classes, pure fns, tests
```

### Why each demo is its own crate

The Bevy demos share one workspace, but these deliberately do not. A
`.gdextension` file resolves the compiled library through Godot's virtual
filesystem:

```ini
linux.debug.x86_64 = "res://target/debug/libhello_world.so"
```

`res://` is the directory containing `project.godot`, so the build output has to
land in *that* crate's own `target/`. A shared workspace target directory would
put the `.so` somewhere Godot cannot see, and every `.gdextension` would need a
different relative path.

The cost of this choice is that each crate carries its own `Cargo.lock`. Keep
the pinned `godot` version identical across all demos — a `cargo update` in one
crate only, repeated over months, is how they silently drift apart.

CI works around the compile cost by pointing `CARGO_TARGET_DIR` at one shared
directory, which is safe there because CI never opens the demos in Godot.

### `Cargo.toml`

```toml
[package]
name = "hello-world"
version = "0.1.0"
edition = "2024"

[lib]
name = "hello_world"          # snake_case — must match the .gdextension filename
crate-type = ["cdylib", "lib"]

[dependencies]
godot = "0.5"
```

`cdylib` is what Godot loads at runtime; `lib` is what `cargo test` links
against. Both are needed — drop `lib` and the unit tests stop compiling.

### `src/lib.rs`

A single file, divided by banner comments in this order:

```rust
//! One-line summary of the demo.
//!
//! Teaches: the specific Godot concepts this demo isolates.

use godot::prelude::*;

// ─── Extension entry point ───────────────────────────────────────────────────

/// Declares this crate as a GDExtension library. Exactly one per crate.
struct HelloWorldExt;

#[gdextension]
unsafe impl ExtensionLibrary for HelloWorldExt {}

// ─── HelloWorld node ─────────────────────────────────────────────────────────

/// What the node does, and what the scene must contain for it to work.
#[derive(GodotClass)]
#[class(base=Node2D)]
pub struct HelloWorld {
    /// Tunables the designer can edit in the inspector.
    #[export]
    greeting: GString,
    base: Base<Node2D>,          // always last, always named `base`
}

#[godot_api]
impl INode2D for HelloWorld {
    fn init(base: Base<Node2D>) -> Self { /* defaults only — no scene access */ }
    fn ready(&mut self) { /* first point where child nodes exist */ }
}

#[godot_api]
impl HelloWorld {
    /// Anything GDScript should be able to call is `#[func]`.
    #[func]
    pub fn greeting_text(&self) -> GString { self.greeting.clone() }
}

// ─── Pure functions ───────────────────────────────────────────────────────────

/// Logic with no `Gd<T>`, no `Base<T>`, and no engine calls.
pub fn format_greeting(name: &str) -> String { format!("Hello, {name}! (from Rust)") }

// ─── Tests ────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests { /* exercises the pure functions */ }
```

## The conventions

1. **One concept per demo.** If a demo needs a second idea to make sense, that
   idea is probably its own demo.
2. **Module-level `//!` rustdoc opening with a `Teaches:` line**, so the demo
   index can be regenerated from the source.
3. **`init` sets defaults only.** Child nodes do not exist yet; anything that
   touches the scene tree belongs in `ready`.
4. **`base` is the last field and is always called `base`.**
5. **Push logic into free `pub fn`s that never mention a Godot type.** This is
   the Godot version of the repository's goal #4: `cargo test` cannot start an
   engine, so whatever is not pure is effectively untested.
6. **Every crate has a `#[cfg(test)] mod tests`** covering those pure functions.
7. **`#[func]` for anything GDScript calls; `#[export]` for anything a designer
   tunes.** Both are part of the demo's public contract — document them.
8. **`clippy` is clean at `-D warnings` and `cargo fmt` is a no-op.** CI enforces
   both.
9. **Every public item carries a `///` doc comment.** Unlike the Bevy workspace,
   this is *not* machine-enforced here: `#[export]` makes gdext generate getter
   and setter methods that no one can attach docs to, so `missing_docs` reports
   about 60 unfixable warnings across the suite and cannot be turned on. Keep it
   by hand instead — document the field, and the accessor's meaning follows.

## Duplication across engines is deliberate

Nine concepts here also exist in `tech-demos/bevy/`, and some of their pure
functions (`hex_neighbors`, `axial_distance`, `cube_round`) are byte-identical
between the two suites. This is a deliberate trade, not a missed refactor — the
full reasoning is in [`../bevy/DEMO_ANATOMY.md`](../bevy/DEMO_ANATOMY.md#duplication-across-engines-is-deliberate).

The short version: goal #4 says extract shared logic, but it yields to goal #1,
and a self-contained demo teaches better than one that sends you to a shared
crate. `tic-tac-toe/` is where goal #4 is demonstrated properly.

The obligation that comes with it: **when you fix a shared pure function, fix
the other engine's copy in the same change**, and keep the cross-links in the
README index current. Both copies of `cube_round` once carried the same
dead-assignment bug, which is exactly the failure mode this rule exists to
prevent.

## Adding a demo

1. Copy the closest existing demo and rename the crate, the `[lib] name`
   (snake_case), and the `.gdextension` file.
2. Update the six library paths inside the `.gdextension` to the new snake_case
   library name.
3. Set `config/name` in `project.godot`.
4. Point `scenes/main.tscn` at the new class.
5. Add a row to the demo table in [`README.md`](README.md).
6. Match the pinned `godot` version used by the other demos — do not let a fresh
   `cargo build` resolve a newer one on its own.

## Running

```bash
cd <demo-name>
cargo build          # Godot is not needed for this step
godot4 --editor .    # or open project.godot from the Godot project manager
```

If Godot reports that the extension failed to load, the usual cause is a missing
`cargo build` (no `target/debug/lib*.so` yet) or a `[lib] name` that does not
match the paths in the `.gdextension`.
