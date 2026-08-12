# Godot + Rust Tech Demos

Each demo is a standalone Godot 4.3+ project with its game logic written in Rust
via the [gdext](https://github.com/godot-rust/gdext) GDExtension binding (`godot = "0.5"`).

Every demo follows the same layout and conventions — see
[`DEMO_ANATOMY.md`](DEMO_ANATOMY.md) before adding or modifying one, and copy
[`_template/`](_template/) to start a new one. In
particular, all demos pin the **same** `godot` version; because each demo is its
own crate with its own `Cargo.lock`, running `cargo update` in a single demo is
how they drift apart.

See also [`tic-tac-toe-godot`](../../tic-tac-toe/tic-tac-toe-godot/) — a
complete game built on these conventions, sharing its rules with the Bevy,
terminal, and bracket-lib frontends.

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

### Fundamentals

| Demo | Concept |
|------|---------|
| `hello-world` | Minimal extension setup — `Node2D` subclass, `godot_print!` ([also in Bevy](../bevy/hello-world/)) |
| `exported-properties` | `#[export]` fields visible in the Godot inspector |
| `player-movement` | `CharacterBody2D` with WASD movement and gravity |
| `signals` | Custom `#[signal]` declarations and `emit_signal` |
| `custom-resource` | `Resource` subclass for game-data containers |
| `scene-tree` | Traversing the scene tree and `Gd<T>` smart pointers |
| `groups` | Node groups: adding, querying, and calling across groups |
| `spawner` | Dynamically instantiating nodes at runtime from Rust |

### Input & Physics

| Demo | Concept |
|------|---------|
| `input-actions` | `Input` singleton polling; `is_action_pressed` / `is_action_just_pressed` |
| `grid-movement` | `VecDeque` direction queue; lerp-based snap-to-grid movement ([also in Bevy](../bevy/grid-movement/)) |
| `drag-and-drop-ui` | `Control._gui_input`; `InputEventMouseButton` / `InputEventMouseMotion` |
| `virtual-joystick` | Touch/mouse virtual joystick; dead-zone; `draw_arc` / `draw_circle` |
| `area-detection` | `Area2D` `body_entered` / `body_exited` signals connected in Rust |
| `ray-casting` | `PhysicsDirectSpaceState2D` ray queries; hit point and normal |
| `collision-layers` | Programmatic collision layer/mask configuration from Rust |
| `one-shot-collision` | `move_and_collide` + `KinematicCollision2D` response; bounce |
| `rigid-body-2d` | `RigidBody2D` impulse-based movement; `apply_central_impulse_ex()` builder; velocity clamping |
| `joints-2d` | `PinJoint2D` + `DampedSpringJoint2D` wired at runtime; bodies connected via `get_path()` |
| `hitbox-hurtbox` | `Area2D` attack hitbox vs hurtbox; damage, knockback, and death handling |

### Animation, Visual & UI

| Demo | Concept |
|------|---------|
| `animated-sprite` | `AnimatedSprite2D` state-based animation switching from Rust |
| `tweening` | `Tween` node created and sequenced entirely from Rust |
| `shader-params` | Set `ShaderMaterial` uniforms from Rust at runtime |
| `line-drawing` | Procedural polyline drawing via `_draw()`; undo / clear |
| `camera-follow` | Smooth lerp `Camera2D`; configurable dead zone; group-based target ([also in Bevy](../bevy/camera-follow/)) |
| `ui-health-bar` | `ProgressBar` + `Label` wired to a Rust health component |
| `timer-and-countdown` | `Timer` node driven from Rust; HUD countdown display |
| `tooltip-system` | `PanelContainer` tooltip shown/hidden on hover via Rust signals |
| `animation-player` | `AnimationPlayer` timeline control: play, seek, speed scale, `animation_finished` signal |
| `canvas-layer` | `CanvasLayer` pins HUD to screen regardless of camera position |
| `rich-text-label` | `RichTextLabel` `push_color` / `push_bold` / `append_text` / `pop` from Rust |
| `particles-2d` | `CpuParticles2D` emission, lifetime, direction, velocity from Rust; burst mode |
| `lighting-2d` | `PointLight2D` energy and color driven by sine-wave; warm/cool color cycle ([also in Bevy](../bevy/lighting-2d/)) |
| `parallax-background` | `ParallaxBackground` + `ParallaxLayer` scroll ratios; camera drives scrolling ([also in Bevy](../bevy/parallax-background/)) |
| `camera-zoom` | Mouse-wheel `Camera2D` zoom with clamped range and smooth lerp toward target |
| `game-clock` | In-game day/night clock; sky `ColorRect` colour driven by time of day; adjustable speed |
| `line-trail` | Motion trail rendered per-segment in `_draw()` with fading alpha |
| `minimap` | Overhead minimap drawn in `_draw()`; world-to-minimap projection each frame ([also in Bevy](../bevy/minimap/)) |
| `screen-effects` | Camera shake (trauma/decay) plus colour-flash overlay fade |
| `split-screen` | Two `SubViewportContainer` + `SubViewport` pairs with independent per-player cameras |

### Architecture Patterns

| Demo | Concept |
|------|---------|
| `state-machine` | Rust `enum` FSM (Idle/Walk/Jump/Attack); exported current-state label |
| `singleton-autoload` | Autoload Rust node as a global service; consumer class reads from it |
| `event-bus` | Autoload signal hub; decoupled nodes communicate without direct refs |
| `command-pattern` | `Box<dyn Command>` undo/redo stack applied to a pure-Rust `GameState` |
| `scene-manager` | Autoload `change_scene_to_file`; scene history breadcrumb |
| `callable-deferred` | `call_deferred` for safe scene-tree mutation; `Callable::from_fn` inline callbacks |
| `node-lifecycle` | `_enter_tree` / `_ready` / `_exit_tree` ordering; `NodeNotification` handling |
| `object-pool` | Pre-spawned pool; `ProcessMode::DISABLED`/`INHERIT` toggling instead of spawn/free |

### Data & Persistence

| Demo | Concept |
|------|---------|
| `save-and-load` | Hand-rolled JSON serialization; `FileAccess` `user://` storage |
| `inventory-system` | `Vec<Item>` with slot capacity; add / remove / query from Rust |
| `quest-system` | `QuestStatus` enum; objective progress; auto-complete |
| `menu-with-options` | `OptionButton` / `CheckBox` / `HSlider` values read into Rust config |
| `dialogue-tree` | Branching dialogue as a flat node `Vec`; keyboard-navigated choices |
| `achievement-system` | Progress-tracked achievements with unlock banners ([also in Bevy](../bevy/achievement-system/)) |

### Interop & GDScript

| Demo | Concept |
|------|---------|
| `gdscript-interop` | Bidirectional Rust↔GDScript via `call()` + `#[func]`; `Variant` conversion |
| `http-request` | `HttpRequest` node; `request_completed` signal; `PackedByteArray` → String parsing |

### Procedural & AI

| Demo | Concept |
|------|---------|
| `noise-generation` | `FastNoiseLite::new_gd()`; greyscale terrain grid rendered via `_draw()` |
| `navigation` | `NavigationAgent2D` path-follow driven from Rust |
| `pathfinding-astar` | `AStar2D::new_gd()`; click to set start/goal; path visualised with polyline |
| `steering-behaviors` | Seek / Flee / Wander pure-Rust vectors applied to `CharacterBody2D` |
| `tilemap-basic` | `TileMap.set_cell_ex()` builder; bordered room generation from Rust |
| `tilemap-procedural` | Cellular-automata cave generator written cell-by-cell into `TileMap` |
| `scene-instancing` | `ResourceLoader::load()` → `try_cast::<PackedScene>()` → `instantiate()` |
| `hex-grid` | Flat-top axial hex grid; `axial_to_pixel` / `pixel_to_axial` round-trip; click to select ([also in Bevy](../bevy/hex-grid/)) |

### Abilities & Audio

| Demo | Concept |
|------|---------|
| `ability-cooldowns` | Per-ability cooldown timers; ready/active/cooling states; HUD display ([also in Bevy](../bevy/ability-cooldowns/)) |
| `audio-manager` | Pool of `AudioStreamPlayer` children; polyphonic SFX playback from Rust |
| `music-with-transitions` | Cross-fade between two `AudioStreamPlayer` nodes via Rust lerp |

### Advanced

| Demo | Concept |
|------|---------|
| `editor-plugin` | `EditorPlugin` subclass with dockable panel; button with click counter; extends the Godot editor |

## Project structure

Each demo follows this layout:

```
demo-name/
├── Cargo.toml               # Rust lib crate (crate-type = ["cdylib", "lib"])
├── src/
│   └── lib.rs               # GDExtension classes + pure-function helpers + tests
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
- The `navigation` and `editor-plugin` demos require the `experimental-godot-api` feature flag (already set in their `Cargo.toml`)

## Also implemented in Bevy

These nine concepts exist in both engine suites, so you can read the same
problem solved two ways — Godot's scene tree versus Bevy's ECS. Some of their pure
helper functions are byte-identical; that duplication is deliberate and is
explained in [`DEMO_ANATOMY.md`](DEMO_ANATOMY.md).

| Concept | This suite | Bevy |
|---------|-----------|------|
| `ability-cooldowns` | [`ability-cooldowns`](ability-cooldowns/) | [`ability-cooldowns`](../bevy/ability-cooldowns/) |
| `achievement-system` | [`achievement-system`](achievement-system/) | [`achievement-system`](../bevy/achievement-system/) |
| `camera-follow` | [`camera-follow`](camera-follow/) | [`camera-follow`](../bevy/camera-follow/) |
| `grid-movement` | [`grid-movement`](grid-movement/) | [`grid-movement`](../bevy/grid-movement/) |
| `hello-world` | [`hello-world`](hello-world/) | [`hello-world`](../bevy/hello-world/) |
| `hex-grid` | [`hex-grid`](hex-grid/) | [`hex-grid`](../bevy/hex-grid/) |
| `lighting-2d` | [`lighting-2d`](lighting-2d/) | [`lighting-2d`](../bevy/lighting-2d/) |
| `minimap` | [`minimap`](minimap/) | [`minimap`](../bevy/minimap/) |
| `parallax-background` | [`parallax-background`](parallax-background/) | [`parallax-background`](../bevy/parallax-background/) |

When you change a shared helper in one suite, change the other in the same
commit — they have silently diverged before.
