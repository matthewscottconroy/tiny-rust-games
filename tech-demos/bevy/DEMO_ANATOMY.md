# Anatomy of a Bevy demo

Every demo in this workspace follows one shape so it works as **both** a
runnable example **and** a drop-in building block for a larger project. The
`_template` demo is a minimal, copyable instance of this shape.

## The shape

```
demo-name/
├── Cargo.toml        # edition 2024; bevy.workspace = true; [lints] workspace = true
└── src/
    ├── lib.rs        # the reusable block: a Plugin + Config + components + pure fns + tests
    └── main.rs       # a thin runner: DefaultPlugins + the plugin
```

A single crate produces two targets: a **library** (`src/lib.rs`, importable as
`demo_name`) and a **binary** (`src/main.rs`, runnable with `cargo run`). Cargo
detects both automatically — no `[lib]`/`[[bin]]` needed.

### `lib.rs` — the building block

```rust
use bevy::prelude::*;

/// One plugin that registers everything the feature needs.
pub struct FeaturePlugin;

impl Plugin for FeaturePlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<FeatureConfig>()      // tunables
            .add_message::<SomeEvent>()           // I/O contract (out)
            .add_systems(Startup, setup)
            .add_systems(Update, (system_a, system_b));
    }
    // NOTE: a plugin never adds DefaultPlugins — the host app owns that.
}

/// Tunables the host can override before adding the plugin.
#[derive(Resource, Clone, Copy)]
pub struct FeatureConfig { pub speed: f32 }
impl Default for FeatureConfig {
    fn default() -> Self { Self { speed: 200.0 } }
}

/// Components/messages the host may query or react to are `pub`.
#[derive(Component)] pub struct Player;

/// Self-contained logic is a `pub fn` so it is unit-testable without a World.
pub fn step(pos: f32, dir: f32, speed: f32, dt: f32) -> f32 { pos + dir * speed * dt }

fn setup(/* ... */) { /* ... */ }
fn system_a(/* ... */) { /* ... */ }

#[cfg(test)]
mod tests {
    use super::*;
    // Pure-function tests + MinimalPlugins ECS tests live here.
}
```

### `main.rs` — the runner

```rust
use bevy::prelude::*;
use demo_name::FeaturePlugin;

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .add_plugins(FeaturePlugin)
        .run();
}
```

## The five conventions

1. **Plugin encapsulation.** All `add_systems` / `init_resource` /
   `add_message` / `init_state` calls live in `impl Plugin for FeaturePlugin`,
   never in `main`. This is what makes the demo liftable.
2. **`main` owns the bootstrap.** `DefaultPlugins`, window settings, and
   `ClearColor` stay in `main`. The plugin must be host-agnostic.
3. **`pub` the seams.** The plugin, its config resource, and any component or
   message a consumer needs to query or react to are public. Internal systems
   and marker helpers can stay private.
4. **Config resource for tunables.** Hoist magic numbers (speeds, sizes, spawn
   counts, colors) into a `#[derive(Resource)] FeatureConfig` with a `Default`
   impl, so the block is retunable without editing it.
5. **Communicate via messages/resources.** Take input as resources, emit
   results as `Message`s. A block that assumes it owns the whole screen does not
   compose; one with a clear I/O contract does.

## Reusing a demo in your own project

Either copy `src/lib.rs` into your project as a module, or depend on the crate
and add its plugin:

```rust
app.add_plugins((DefaultPlugins, health_and_damage::HealthAndDamagePlugin));
```

> **Component name collisions:** many demos define generically named components
> (`Player`, `Velocity`, `Enemy`). That is deliberate for standalone clarity,
> but when combining several demos into one app, rename or namespace the copied
> components to avoid clashes.

## Testing

```bash
cargo test --workspace           # from tech-demos/bevy
cargo test -p health-and-damage  # one demo
```

Pure functions are tested directly; ECS wiring is tested headlessly with
`MinimalPlugins` (and, where the plugin is host-agnostic, by adding the plugin
itself to a `MinimalPlugins` app).

One demo has no `#[cfg(test)]` module: `draw-window` is ten lines that open a
window, with no logic to assert on. Do not add a ceremonial test to a demo in
that category — say it is wiring-only instead. (`audio` used to be the second
such demo; it stopped being one when its sound generation moved into a pure
function, which is usually the better fix.)

## Duplication across engines is deliberate

Fifteen concepts are implemented in both this suite and `tech-demos/godot/`, and
some of their pure functions (`hex_neighbors`, `axial_distance`, `cube_round`)
are byte-identical. That is **not** an oversight waiting to be factored into a
shared crate.

Repository goal #4 says to extract engine-agnostic logic, but it also says goal
#1 — transparent, self-contained code — takes precedence when the two conflict.
Here they conflict. A demo whose whole purpose is "read this one file and learn
hex grids" is ruined by sending the reader to a shared crate for the half that
matters. `tic-tac-toe/` is where goal #4 is demonstrated properly: a real game
whose rules genuinely must not diverge between frontends.

What the duplication *does* buy is the comparison goal #2 asks for — the same
problem solved in an ECS and in a scene tree, side by side. To make that useful
rather than accidental:

- **Keep paired demos in sync.** If you fix a bug in a shared pure function,
  fix it in the other engine's copy in the same change. Both copies of
  `cube_round` previously carried the same dead-assignment bug.
- **Cross-link them.** Paired demos name their counterpart in the README index
  and in their module rustdoc, with a `Counterpart:` line.

Both of those are now checked rather than merely asked for, by
`tools/check-paired-demos.py`, which CI runs. It finds the pairs from each
demo's `Concept:` tag (or its directory name), fails if either side does not
name the other, and fails if a function listed as shared has drifted — comparing
bodies with comments stripped, so a reformat is not mistaken for divergence.
Add a function to its `MUST_MATCH` table when you copy one between suites.

The check is what makes the trade above honest. Deliberate duplication is a
defensible choice; duplication that silently diverges is just a bug in two
places, and the difference between them is enforcement.
- **Extract only when logic outgrows a demo.** If a helper becomes large enough
  that a reader would skip it anyway, it has stopped being teaching material
  and can move to a crate of its own.

## Lints

Every member's `Cargo.toml` ends with:

```toml
[lints]
workspace = true
```

which pulls in `[workspace.lints.rust]` and `[workspace.lints.clippy]` from the
workspace manifest. `missing_docs` is warned there, so with CI's `-D warnings`
**every public item must carry a `///` comment** — the README's promise of item
docs is enforced rather than aspirational. Three clippy lints are allowed — `type_complexity` and `too_many_arguments`, because
Bevy system signatures are legitimately wide, and `needless_range_loop`, because
the demos do 2D grid work where the index *is* the meaning. Everything else is a
hard error: CI runs `cargo clippy --all-targets -- -D warnings`.

A new demo that omits the `[lints]` section silently opts out of all of this, so
copy it along with the rest of `Cargo.toml`.
