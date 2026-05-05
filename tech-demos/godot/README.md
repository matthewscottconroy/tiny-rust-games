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

### Fundamentals

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

### Input & Physics

| Demo | Concept |
|------|---------|
| `input-actions` | `Input` singleton polling; `is_action_pressed` / `is_action_just_pressed` |
| `grid-movement` | `VecDeque` direction queue; lerp-based snap-to-grid movement |
| `drag-and-drop-ui` | `Control._gui_input`; `InputEventMouseButton` / `InputEventMouseMotion` |
| `virtual-joystick` | Touch/mouse virtual joystick; dead-zone; `draw_arc` / `draw_circle` |
| `area-detection` | `Area2D` `body_entered` / `body_exited` signals connected in Rust |
| `ray-casting` | `PhysicsDirectSpaceState2D` ray queries; hit point and normal |
| `collision-layers` | Programmatic collision layer/mask configuration from Rust |
| `one-shot-collision` | `move_and_collide` + `KinematicCollision2D` response; bounce |

### Animation, Visual & UI

| Demo | Concept |
|------|---------|
| `animated-sprite` | `AnimatedSprite2D` state-based animation switching from Rust |
| `tweening` | `Tween` node created and sequenced entirely from Rust |
| `shader-params` | Set `ShaderMaterial` uniforms from Rust at runtime |
| `line-drawing` | Procedural polyline drawing via `_draw()`; undo / clear |
| `camera-follow` | Smooth lerp `Camera2D`; configurable dead zone; group-based target |
| `ui-health-bar` | `ProgressBar` + `Label` wired to a Rust health component |
| `timer-and-countdown` | `Timer` node driven from Rust; HUD countdown display |
| `tooltip-system` | `PanelContainer` tooltip shown/hidden on hover via Rust signals |

### Architecture Patterns

| Demo | Concept |
|------|---------|
| `state-machine` | Rust `enum` FSM (Idle/Walk/Jump/Attack); exported current-state label |
| `singleton-autoload` | Autoload Rust node as a global service; consumer class reads from it |
| `event-bus` | Autoload signal hub; decoupled nodes communicate without direct refs |
| `command-pattern` | `Box<dyn Command>` undo/redo stack applied to a pure-Rust `GameState` |
| `scene-manager` | Autoload `change_scene_to_file`; scene history breadcrumb |

### Data & Persistence

| Demo | Concept |
|------|---------|
| `save-and-load` | Hand-rolled JSON serialization; `FileAccess` `user://` storage |
| `inventory-system` | `Vec<Item>` with slot capacity; add / remove / query from Rust |
| `quest-system` | `QuestStatus` enum; objective progress; auto-complete |
| `menu-with-options` | `OptionButton` / `CheckBox` / `HSlider` values read into Rust config |

### Procedural & AI

| Demo | Concept |
|------|---------|
| `noise-generation` | `FastNoiseLite::new_gd()`; greyscale terrain grid rendered via `_draw()` |
| `navigation` | `NavigationAgent2D` path-follow driven from Rust |
| `pathfinding-astar` | `AStar2D::new_gd()`; click to set start/goal; path visualised with polyline |
| `steering-behaviors` | Seek / Flee / Wander pure-Rust vectors applied to `CharacterBody2D` |
| `ability-cooldowns` | Per-ability cooldown timers; ready/active/cooling states; HUD display |

### Audio

| Demo | Concept |
|------|---------|
| `audio-manager` | Pool of `AudioStreamPlayer` children; polyphonic SFX playback from Rust |
| `music-with-transitions` | Cross-fade between two `AudioStreamPlayer` nodes via Rust lerp |

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
- The `navigation` demo additionally requires the `experimental-godot-api` feature flag (already set in its `Cargo.toml`)
