# Object Pooling

A demo of the **object pool pattern**: all bullets are spawned at startup and kept alive for the entire session. Firing a weapon does not create a new entity — it activates an existing dormant one. When a bullet leaves the screen it is deactivated and returned to the pool, ready to be reused.

Press **SPACE** or left-click to fire. Watch the active/total counter in the corner.

---

## What This Is Illustrating

The object pool design pattern applied to game entities — one of the most important performance optimisations in real-time games. In games with many short-lived objects (bullets, particles, explosion fragments, enemies), the cost of constantly allocating and deallocating memory — or spawning and despawning entities — adds up quickly and can cause frame-rate stutters.

---

## Game Design Principle: Predictable Performance Through Preallocated Resources

Games run in real time. A garbage collection pause or a heap allocation spike during a firefight is not a minor inconvenience — it breaks the player's sense of control at exactly the moment when precision matters most. Object pooling trades a fixed upfront cost (N entities spawned at startup) for a guarantee that shooting never triggers allocation.

**How the pattern works:**

1. At startup, spawn N entities in a dormant state (invisible, off-screen).
2. Maintain a pool list (a `Vec<Entity>`) tracking all pooled entities.
3. When a new object is needed: find the first inactive slot, configure it (position, velocity), mark it active, make it visible.
4. When an object's lifetime ends: mark it inactive, hide it, move it to a safe off-screen position. Do not despawn.

The pool is exhausted only if all N slots are simultaneously active. In that case, the fire event is silently dropped (or the oldest active object is recycled). The HUD displays how many slots are currently in use, making the pool state observable.

**Object pooling versus Bevy `despawn`:** Bevy's `despawn` is not free — it removes the entity from the world, which requires updating data structures. More importantly, spawning a replacement requires re-running all `Added<C>` observers and change-detection machinery. Pooling sidesteps all of that by keeping the entity alive and only toggling its `Visibility`.

---

## How Bevy Achieves It

**`Visibility::Hidden` as deactivation.** Bevy's `Visibility` component controls whether an entity is rendered. Setting it to `Visibility::Hidden` removes the entity from the render pipeline without removing it from the world. The entity continues to exist — its components remain, its `Entity` ID is valid — it just does not appear on screen.

**`BulletPool` resource as the slot list.** A `Vec<Entity>` resource stores all pool entity IDs in order. To fire, the system collects the `active: bool` field of each entity into a plain `Vec<bool>`, passes that slice to `find_inactive` (a pure `&[bool] → Option<usize>` function), and uses the returned index to look up the entity ID. Pure function + resource ID lookup = straightforward unit testing and clear separation of concerns.

**Pure off-screen detection.** `is_off_screen(pos: Vec2, half: Vec2) -> bool` checks whether a position is outside the window rectangle (with a small margin so bullets fully exit before deactivating). This pure function is tested independently of the ECS. The `move_bullets` system calls it each frame for every active bullet and deactivates those that return `true`.

**Fire cooldown as a resource.** `FireCooldown(f32)` is decremented each frame in `tick_cooldown`. The `handle_fire` system early-returns if `cd.0 > 0.0`. This decouples the rate-limiting logic from input detection and prevents holding SPACE from saturating the pool instantly.

---

## Running

```
cargo run
```

Controls: **SPACE** or **left-click** — fire.
