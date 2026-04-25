# Stealth AI

A demo of **field-of-view cone detection** combined with a three-state finite state machine: a guard patrols waypoints, transitions to an alert state when it spots the player in its FOV cone, and chases the player's last known position before eventually returning to patrol.

Move the player (cyan) with **WASD** or arrow keys. Avoid the guard's yellow cone.

---

## What This Is Illustrating

AI perception and reactive state machines — the two mechanisms that make enemies feel aware of their environment without being omniscient. An enemy that can always see the player has no interesting behaviour; an enemy with bounded perception gives the player space to plan, hide, and outmanoeuvre.

---

## Game Design Principle: AI Perception and Believable Behaviour

Players accept game AI not because it is realistic but because it is *legible*: its rules are consistent enough that the player can predict and respond to them. A guard that reacts correctly to what it can see (and ignores what it cannot) is legible. A guard that instantly knows the player's position at all times is not.

**Field-of-view (FOV) cone:** The guard's vision is modelled as a cone in world space: a direction vector (facing), a half-angle (aperture), and a maximum range. Any point inside all three constraints is detected. This matches how humans perceive peripheral vision and creates natural blind spots the player can exploit.

**Three-state FSM:**

| State | Trigger to enter | Behaviour | Trigger to exit |
|---|---|---|---|
| Patrol | Start / alert timeout | Walk waypoint loop | Player spotted |
| Alert | Player enters FOV | Stop, face last sighting | Player seen again → Chase; timeout → Patrol |
| Chase | Player seen while alert | Run toward last known pos | Reached position → Alert |

The Alert state is a buffer between Patrol and Chase: it gives the guard time to confirm a sighting before committing to a chase, which feels more believable than an instant reaction. It also gives the player a brief window to hide.

---

## How Bevy Achieves It

**Pure FOV function.** `in_fov_cone(target, origin, forward, half_angle, range) -> bool` operates entirely on `Vec2` values. It normalises the to-target vector, takes the dot product with the guard's facing direction, and compares the result against the cosine of the half-angle. Because `cos(half_angle) ≥ dot(dir, forward)` iff `angle ≤ half_angle`, no trigonometric arc functions are needed at runtime — only one cosine computed from the constant `FOV_HALF_ANGLE`. This is fully unit-testable with known angles.

**Guard state stored as a Rust enum on a component.** `GuardState` is a Rust `enum` stored directly on the `Guard` component. Enum variants can carry data (`Alert { timer: f32 }`, `Chase { last_known: Vec2 }`), so there is no need for a separate component per state. The transition logic is a single `match` block inside `update_guard` that reads the current state, computes the new state, and writes it back — all in one system.

**FOV visualisation via a fan of sprite segments.** The cone is drawn as `FOV_RAYS = 24` thin sprite rectangles, each rotated to a different angle within `[base − half_angle, base + half_angle]`. Their positions are recomputed every frame in `draw_fov` from the guard's current `facing` direction. This is simpler than a custom mesh and requires no rendering plugins.

**Status tile for state colour feedback.** A separate `StatusTile` sprite entity sits behind the guard at Z = 0.5. The `update_status` system reads the guard's state and sets the tile's colour: green for Patrol, orange for Alert, red for Chase. This makes the AI's internal state immediately readable without a HUD.

---

## Running

```
cargo run
```

Controls: **WASD / Arrow keys** — move player.
