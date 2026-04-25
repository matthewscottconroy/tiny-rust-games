# Procedural Dungeon

A demo of **Binary Space Partitioning (BSP)** dungeon generation: a classic algorithm for producing varied, interconnected room layouts without manual design work.

Press **SPACE** to regenerate the dungeon with a new random seed.

---

## What This Is Illustrating

Procedural content generation — specifically, how to algorithmically build a playable dungeon map from scratch at runtime. The output is a 2D grid of floor and wall tiles, which is the fundamental representation used by virtually every tile-based RPG, roguelike, and dungeon crawler.

---

## Game Design Principle: Procedural Level Generation

Hand-crafting every level of a game is expensive and limits replay value. Procedural generation lets the game create its own content by applying a set of rules to a source of randomness (a seed). The same seed always produces the same dungeon, which enables repeatable experiences, shareable seeds, and deterministic multiplayer.

**How BSP works:**

1. Start with the entire playable area as one large rectangle (the root).
2. Recursively split that rectangle along a random axis (horizontal or vertical) at a random position. Each split produces two smaller child rectangles.
3. Keep splitting until every leaf rectangle is small enough to become a room.
4. Carve a room slightly smaller than each leaf rectangle into the grid (leaving a wall margin).
5. Connect every pair of sibling rooms with an L-shaped corridor.

The result is a fully connected graph of rooms with no dead-end isolated areas, because siblings are always linked during the connection step.

**Why BSP specifically:** BSP guarantees that every leaf room is reachable from every other leaf room. Alternative algorithms (random room scatter, cellular automata) can produce isolated pockets. BSP's recursive structure also gives natural control over minimum room size and overall room count.

---

## How Bevy Achieves It

**Deterministic RNG without dependencies.** The demo uses a hand-rolled 64-bit linear congruential generator (`lcg_next`) seeded from the `DungeonSeed` resource. No external crate is needed, and the function is a pure `u64 → u64` transform that is trivially testable.

**Pure geometry functions.** `split_room(rect, axis, pos, min_size)` and `room_interior(rect, margin)` operate only on the `Rect` struct — no ECS, no Bevy types. This keeps the algorithm fully unit-testable and separates generation logic from rendering.

**Tile grid as a flat `Vec<Vec<Tile>>`.** The map is built in plain Rust; only after generation is complete does the code iterate over it and call `commands.spawn(...)` to create one sprite entity per tile. This one-shot spawn pattern is cheap and keeps the ECS free of intermediary generation state.

**Regeneration via despawn + respawn.** Pressing SPACE despawns all entities tagged with the `TileSprite` marker component, advances the seed with `lcg_next`, regenerates, and respawns. Bevy's command queue defers the despawns until the end of the current frame, so the old tiles are visible for exactly one more frame before the new ones appear — which is imperceptible.

---

## Running

```
cargo run
```

Controls: **SPACE** — new dungeon.
