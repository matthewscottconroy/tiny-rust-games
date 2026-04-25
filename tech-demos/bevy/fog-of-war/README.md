# Fog of War

A demo of **three-state tile visibility**: tiles transition from *Hidden* (black, never seen) to *Remembered* (dimmed, seen before) to *Visible* (bright, currently in the player's sight radius) as the player explores a grid map.

Move with **WASD** or arrow keys to reveal the map.

---

## What This Is Illustrating

Map revelation — the mechanism by which a game world is progressively uncovered as the player moves through it. Fog of war separates what the player *currently sees* from what the player *remembers seeing*, making exploration feel meaningful and giving the player a persistent record of their journey without removing tension from re-entry.

---

## Game Design Principle: Limiting and Rewarding Information

A fully revealed map transforms exploration into navigation: the player already knows where to go. Fog of war turns the map itself into a resource — each new area revealed is a small reward, and the boundary between the known and the unknown is where interesting decisions happen. Should the player enter that dark corridor? Push forward for resources, or retreat to safety?

**Three states instead of two:**

Most games use a binary visible/hidden model as a first approximation. The three-state model (Hidden → Remembered → Visible) is the standard production approach because it communicates an important distinction: the player *knew* something was here, but doesn't know if it has changed since. In an RTS, a remembered enemy base might have been reinforced. In a roguelike, a remembered shop might have been looted. Memory creates verisimilitude.

**Sight radius as a circle, not a square:** The demo uses the Euclidean distance formula (`dx² + dy² ≤ r²`) to produce a circular sight area. A square (`|dx| ≤ r AND |dy| ≤ r`) is faster to compute but feels unnatural; a circle matches player intuition about "what I can see around me." The integer arithmetic avoids floating-point drift.

---

## How Bevy Achieves It

**Pure spatial functions.** `cells_in_radius(center, radius) -> Vec<IVec2>` produces all grid cells within a Euclidean circle using integer arithmetic only. `is_within_radius(pos, center, radius) -> bool` is the per-tile fast path. Both are independent of Bevy and fully unit-tested.

**`HashSet<IVec2>` as persistent memory.** The `Revealed` resource accumulates every cell the player has ever seen. Sets support O(1) insertion and O(1) lookup, and unlike a second grid array, they require no pre-allocation and scale naturally with the actual explored area.

**Single query updates all tiles per frame.** `update_fog` queries `(&TileCell, &mut Sprite)` and re-evaluates each tile's state every frame. Because sight-radius cells are cheap to enumerate and the grid is small (~700 tiles), a full recompute is faster than maintaining a change-detection system and avoids edge cases when the player moves diagonally.

**Marker component for grid position.** `TileCell(IVec2)` stores the logical grid coordinate on each sprite entity. This separates concerns: `Transform` handles the visual world-space position; `TileCell` handles the game logic coordinate. The visibility system only needs `TileCell` and never touches `Transform`.

---

## Running

```
cargo run
```

Controls: **WASD / Arrow keys** — move.
