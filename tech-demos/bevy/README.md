# Bevy Tech Demos

A collection of small, self-contained [Bevy](https://bevyengine.org/) `0.18` demos,
each isolating a single engine concept or gameplay system. Every demo has
module-level rustdoc and `///` docs on every public item — `missing_docs` is
enabled workspace-wide and CI denies warnings, so this cannot silently rot — and every demo with logic to exercise
has a `#[cfg(test)]` test module — most game logic is factored into `pub fn`
pure functions so it can be unit-tested headlessly without a window. The sole
exception is `draw-window`, ten lines that open a window and contain no logic.

See [`DEMO_ANATOMY.md`](DEMO_ANATOMY.md) for the shape every demo follows.

## Workspace

All demos live in a single Cargo workspace, so Bevy is compiled once and shared
across every demo instead of rebuilt per crate.

[`tic-tac-toe-bevy`](../../tic-tac-toe/tic-tac-toe-bevy/) is also a member of
this workspace even though it lives under `tic-tac-toe/`, for the same reason —
it is the Bevy frontend of the shared tic-tac-toe rules, not a tech demo, so it
is not in the index below. Run it with `cargo run -p tic-tac-toe-bevy`.

```bash
# From this directory (tech-demos/bevy):
cargo run -p hello-world        # run a specific demo
cargo test --workspace          # test every demo
cargo build -p boids-flocking   # build one demo

# Or from inside a demo directory:
cd hello-world && cargo run
```

The workspace pins one Bevy version centrally (`[workspace.dependencies]`) and
uses Bevy's recommended dev profile — dependencies are compiled optimized so the
demos run smoothly, while your own code stays fast to recompile.

Some demos load assets from their own `assets/` directory; run those from the
demo's own folder (or with `cargo run -p <demo>`) so the paths resolve.

## Reusing a demo as a building block

Every demo is structured so it works as **both** a runnable example **and** a
drop-in block for a larger project. Each crate splits into:

- `src/lib.rs` — the reusable feature as a `Plugin` (e.g. `HealthAndDamagePlugin`),
  tuned by a `Config` resource, with its components, messages, and pure functions
  exposed as `pub`;
- `src/main.rs` — a thin runner that adds `DefaultPlugins` and the plugin.

So you can either copy `lib.rs` into your project, or depend on the crate and add
its plugin:

```rust
app.add_plugins((DefaultPlugins, health_and_damage::HealthAndDamagePlugin));
```

Start a new demo by copying [`_template/`](_template/). The full conventions are
in [`DEMO_ANATOMY.md`](DEMO_ANATOMY.md).

> Many demos define generically named components (`Player`, `Velocity`, `Enemy`).
> That is deliberate for standalone clarity — rename or namespace them when
> combining several demos into one app.

## Demos

### ECS Fundamentals

| Demo | Concept |
|------|---------|
| [`hello-world`](hello-world/) | The minimal Bevy ECS starting point ([also in Godot](../godot/hello-world/)) |
| [`draw-window`](draw-window/) | The absolute minimum app that opens a window |
| [`hello-plugin`](hello-plugin/) | Wrapping systems in a reusable `Plugin` |
| [`sprite-demo`](sprite-demo/) | The simplest way to display an image |
| [`movable-sprite`](movable-sprite/) | WASD movement applied to a sprite entity |
| [`two-players`](two-players/) | Two independent sprites with separate key bindings |
| [`events`](events/) | Decoupled communication with the `Message` API |
| [`observer-events`](observer-events/) | `Message`/`MessageWriter` vs `On<E>` observers, side by side |
| [`one-shot-systems`](one-shot-systems/) | `register_system` + `run_system` invoked on demand |
| [`required-components`](required-components/) | `#[require(...)]` auto-adds companion components |

### Scheduling, State & Performance

| Demo | Concept |
|------|---------|
| [`fixed-timestep`](fixed-timestep/) | Deterministic physics in the `FixedUpdate` schedule |
| [`time-scale`](time-scale/) | Slow-mo / fast-forward via `Time<Virtual>` |
| [`scene-pause`](scene-pause/) | Pausing systems with a `GameState` toggle |
| [`scene-transition`](scene-transition/) | Swapping scenes with `AppState` and entity teardown |
| [`spatial-partitioning`](spatial-partitioning/) | Grid-cell bucketing for O(1) neighbour queries |
| [`object-pooling`](object-pooling/) | Reusing pre-spawned entities instead of spawn/despawn |

### Input & Cameras

| Demo | Concept |
|------|---------|
| [`mouse-input`](mouse-input/) | Cursor position, clicks, and hover highlighting |
| [`gamepad-input`](gamepad-input/) | Analog sticks, buttons, dead-zone, keyboard fallback |
| [`drag-and-drop`](drag-and-drop/) | Click-drag entities with grid snap on drop |
| [`rubber-band-selection`](rubber-band-selection/) | RTS marquee box selection |
| [`camera-follow`](camera-follow/) | Smooth exponential camera lerp toward a target ([also in Godot](../godot/camera-follow/)) |
| [`minimap`](minimap/) | Two `Camera2d` instances rendering to viewports ([also in Godot](../godot/minimap/)) |
| [`pixel-perfect-camera`](pixel-perfect-camera/) | Virtual 320×180 canvas at integer scale |
| [`parallax-background`](parallax-background/) | Layered backgrounds scrolling at different rates ([also in Godot](../godot/parallax-background/)) |
| [`screen-wrap`](screen-wrap/) | Wrapping entities around screen edges |
| [`screen-shake`](screen-shake/) | Trauma/decay camera shake |

### Rendering, Animation & UI

| Demo | Concept |
|------|---------|
| [`hud-and-ui`](hud-and-ui/) | Bevy UI nodes for a heads-up display |
| [`floating-text`](floating-text/) | World-space `Text2d` damage numbers |
| [`y-sort`](y-sort/) | Depth sorting sprites by their Y position |
| [`spritesheet-animation`](spritesheet-animation/) | Frame-by-frame texture-atlas animation |
| [`tween-animation`](tween-animation/) | Easing-based property tweens |
| [`custom-shader`](custom-shader/) | Custom `Material2d` + WGSL ripple shader |
| [`lighting-2d`](lighting-2d/) | Distance-based light intensity on a tile grid ([also in Godot](../godot/lighting-2d/)) |
| [`day-night-cycle`](day-night-cycle/) | Ambient colour lerp over a virtual 24-hour clock |
| [`cutscene-sequencer`](cutscene-sequencer/) | Data-driven cutscene step sequencer |
| [`circle-buttons`](circle-buttons/) | Coloured circles acting as clickable buttons |
| [`inventory-ui`](inventory-ui/) | Grid-based inventory with pick-up / place / swap |
| [`menu-navigation`](menu-navigation/) | Arrow-key navigable menu with state transitions |
| [`notification-system`](notification-system/) | Timed UI toast messages spawned by game events |

### Physics & Simulation

| Demo | Concept |
|------|---------|
| [`collision-detection`](collision-detection/) | AABB overlap and minimum-translation-vector resolution |
| [`platformer-physics`](platformer-physics/) | Gravity, AABB, coyote time, and jump buffering |
| [`projectiles`](projectiles/) | Spawning and culling off-screen projectiles |
| [`particle-system`](particle-system/) | Hand-rolled particle emitter with a custom RNG |
| [`boids-flocking`](boids-flocking/) | Separation / alignment / cohesion flocking |
| [`soft-body`](soft-body/) | 5×5 spring-mass soft body with Euler integration |
| [`rope-simulation`](rope-simulation/) | Verlet integration with iterative distance constraints |
| [`water-ripple`](water-ripple/) | Discrete wave-equation ripple simulation |
| [`destructible-terrain`](destructible-terrain/) | Grid tiles with hit points that break open |
| [`weather-system`](weather-system/) | Weather states with sky colour, rain, and wind |
| [`bullet-pattern`](bullet-pattern/) | Danmaku radial / spiral / aimed bullet patterns |

### AI & Pathfinding

| Demo | Concept |
|------|---------|
| [`enemy-chase-ai`](enemy-chase-ai/) | An enemy that steers toward the player |
| [`state-machine-ai`](state-machine-ai/) | Per-entity finite-state-machine behaviour |
| [`stealth-ai`](stealth-ai/) | FOV-cone detection with Patrol / Alert / Chase states |
| [`behavior-tree`](behavior-tree/) | Composable AI via Sequence / Selector / Leaf nodes |
| [`pathfinding`](pathfinding/) | A\* on a grid with a seeker chasing the player |
| [`flow-field-pathfinding`](flow-field-pathfinding/) | BFS flow field steering many agents at once |
| [`line-of-sight`](line-of-sight/) | Bresenham ray casting for visibility |
| [`fog-of-war`](fog-of-war/) | Three-state tile visibility on a grid map |
| [`turn-based`](turn-based/) | Initiative order, action points, and grid movement |
| [`wave-spawner`](wave-spawner/) | Escalating enemy waves with an inter-wave countdown |

### Gameplay & RPG Systems

| Demo | Concept |
|------|---------|
| [`health-and-damage`](health-and-damage/) | Health bars, damage, and respawning |
| [`knockback-hitstop`](knockback-hitstop/) | Knockback and hit-stop game-feel mechanics |
| [`status-effects`](status-effects/) | Time-decaying debuffs (Poison, Burn, Slow, Stun) |
| [`ability-cooldowns`](ability-cooldowns/) | Multiple abilities with independent cooldown timers ([also in Godot](../godot/ability-cooldowns/)) |
| [`combo-system`](combo-system/) | Input-buffer sequence matching for combos |
| [`upgrade-tree`](upgrade-tree/) | Branching skill unlocks with prerequisites |
| [`crafting-system`](crafting-system/) | Inventory collection and recipe matching |
| [`pickup-and-inventory`](pickup-and-inventory/) | Picking up world items into an inventory |
| [`loot-table`](loot-table/) | Weighted random drops without the `rand` crate |
| [`save-load`](save-load/) | Serializing and restoring game state |
| [`dialog-system`](dialog-system/) | Conversation tree with a typing effect |
| [`achievement-system`](achievement-system/) | Progress-tracked milestones with unlock toasts ([also in Godot](../godot/achievement-system/)) |

### Procedural & Grid Worlds

| Demo | Concept |
|------|---------|
| [`procedural-dungeon`](procedural-dungeon/) | BSP room splitting and corridor carving |
| [`noise-map`](noise-map/) | Seeded value noise with fractal layering |
| [`tilemap`](tilemap/) | Rendering a tile grid from map data |
| [`hex-grid`](hex-grid/) | Flat-top axial hex coordinates and neighbours ([also in Godot](../godot/hex-grid/)) |
| [`grid-movement`](grid-movement/) | Tile-by-tile snapped movement ([also in Godot](../godot/grid-movement/)) |

### Games, Simulations & Audio

| Demo | Concept |
|------|---------|
| [`card-game`](card-game/) | 52-card deck with shuffle, draw, and play |
| [`ascii-npc-sim`](ascii-npc-sim/) | Headless terminal ASCII NPC grid simulation |
| [`ecs-ascii-sim`](ecs-ascii-sim/) | Pedagogical ECS ASCII grid sim (see its `ARCHITECTURE.md`) |
| [`rabbit_carrot_wolf`](rabbit_carrot_wolf/) | Rabbit–carrot–wolf ecosystem simulation |
| [`audio`](audio/) | Background music and triggered sound effects |

## Testing

Game logic is extracted into pure `pub fn`s and tested directly; ECS wiring is
tested headlessly with `MinimalPlugins`. A few asset-dependent demos
(`sprite-demo`, `movable-sprite`, `two-players`, `audio`) can't run their setup
under `MinimalPlugins`, so those test constants and pure functions only.

```bash
cargo test --workspace   # from tech-demos/bevy
```

Building the Bevy demos on Linux requires the ALSA and udev development headers:

```bash
sudo apt-get install -y libasound2-dev libudev-dev
```

## Also implemented in Godot

These nine concepts exist in both engine suites, so you can read the same
problem solved two ways — Bevy's ECS versus Godot's scene tree. Some of their pure
helper functions are byte-identical; that duplication is deliberate and is
explained in [`DEMO_ANATOMY.md`](DEMO_ANATOMY.md).

| Concept | This suite | Godot |
|---------|-----------|-------|
| `ability-cooldowns` | [`ability-cooldowns`](ability-cooldowns/) | [`ability-cooldowns`](../godot/ability-cooldowns/) |
| `achievement-system` | [`achievement-system`](achievement-system/) | [`achievement-system`](../godot/achievement-system/) |
| `camera-follow` | [`camera-follow`](camera-follow/) | [`camera-follow`](../godot/camera-follow/) |
| `grid-movement` | [`grid-movement`](grid-movement/) | [`grid-movement`](../godot/grid-movement/) |
| `hello-world` | [`hello-world`](hello-world/) | [`hello-world`](../godot/hello-world/) |
| `hex-grid` | [`hex-grid`](hex-grid/) | [`hex-grid`](../godot/hex-grid/) |
| `lighting-2d` | [`lighting-2d`](lighting-2d/) | [`lighting-2d`](../godot/lighting-2d/) |
| `minimap` | [`minimap`](minimap/) | [`minimap`](../godot/minimap/) |
| `parallax-background` | [`parallax-background`](parallax-background/) | [`parallax-background`](../godot/parallax-background/) |

When you change a shared helper in one suite, change the other in the same
commit — they have silently diverged before.
