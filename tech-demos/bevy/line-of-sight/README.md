# Line of Sight

A demo of **grid-based visibility** using Bresenham's line algorithm: each frame, the game determines which tiles the player can currently see, which tiles have been seen before, and which remain completely unknown.

Move with **WASD** or arrow keys. Watch the fog reveal as you explore.

---

## What This Is Illustrating

Visibility calculation — answering the question "what can this entity see from where it is standing?" This is foundational for stealth games, roguelikes, strategy games, and any game where information is a resource. A player who cannot see around corners has strategic reasons to explore carefully; an AI that can only react to visible threats feels believable.

---

## Game Design Principle: Information as Gameplay

When players can see everything on a map at once, movement becomes a logistics puzzle: find the shortest path to the goal. When vision is limited, movement becomes an exploration risk: the player must decide whether unknown territory is safe to enter. Line of sight is the primary mechanism that transforms map space into *explored* versus *unknown*, giving the map layers of meaning rather than displaying it all at once.

**Three-tier visibility model:**

| State | Meaning | Rendering |
|---|---|---|
| Hidden | Never seen | Black |
| Remembered | Seen before, not currently visible | Dimmed colours |
| Visible | Inside current sight radius with no blocking wall | Full colour |

The *Remembered* tier is important: showing remembered tiles in a dimmed state lets the player navigate using their accumulated knowledge without giving away real-time enemy positions.

**How sight blocking works:** For each candidate cell within the sight radius, the demo traces an integer line from the player's cell to the candidate cell using Bresenham's algorithm. If any cell along that line is a wall, the candidate is not visible — even if it is within the radius. This correctly models corners blocking vision.

---

## How Bevy Achieves It

**Pure rasterisation function.** `bresenham_cells(from, to) -> Vec<IVec2>` is a self-contained Rust function with no Bevy imports. It produces the sequence of grid cells a line passes through using only integer arithmetic. `can_see(grid, from, to)` wraps it and returns `false` at the first wall encountered. Both are unit-tested independently of the ECS.

**`TileCell` component as a position cache.** Each tile sprite carries a `TileCell(IVec2)` component storing its grid coordinates. The `update_visibility` system queries all tiles with `Query<(&TileCell, &mut Sprite)>` and reads coordinates directly from the component instead of back-calculating position from `Transform`. This avoids a division and keeps the system's intent obvious.

**`Revealed` as a `HashSet<IVec2>` resource.** Persistent memory across frames is stored in a `HashSet` resource. Inserting a cell is O(1) and checking membership (`revealed.0.contains(cell)`) is also O(1). The set only grows — revealed tiles are never removed — modelling perfect map memory.

**Per-frame full recompute.** The visibility update runs every frame, recomputing the entire visible set from scratch. This is simpler than maintaining a dirty flag and incrementally updating, and with a grid of ~600 tiles and a radius of 8, the inner loop executes fewer than 25 000 iterations — well under any frame-time budget.

---

## Running

```
cargo run
```

Controls: **WASD / Arrow keys** — move.
