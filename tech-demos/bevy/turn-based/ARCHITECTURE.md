# Architecture: ASCII NPC Simulation

## 1. How the ECS Pattern Was Applied

Entity-Component-System (ECS) structures a program around three primitives:

| Primitive | Role in this game |
|-----------|-------------------|
| **Entity** | An opaque ID representing one NPC |
| **Component** | Data attached to an entity — here `Position { x, y }` and the marker `Npc` |
| **System** | A function that queries components and transforms them — here `spawn_npcs` and `tick_turn` |
| **Resource** | Singleton data shared across systems — `GridConfig`, `TurnTimer`, `RngState`, `TurnCount` |

### Entities and components

Every NPC is a Bevy entity with two components:

```
Entity ─┬─ Npc          (marker — no data, used as a query filter)
        └─ Position     (x: usize, y: usize)
```

There are no other entity types.  The grid itself has no entities; it exists
implicitly as the set of all `Position` values.

### Systems

**`spawn_npcs` (Startup schedule)**  
Reads `GridConfig` and `RngState`, spawns N entities with randomised `Position`
components so that no two share a cell, then prints the initial frame.

**`tick_turn` (Update schedule)**  
Fires once per turn (gated by `TurnTimer`).  The system:
1. Snapshots all `(Entity, x, y)` pairs from the ECS query.
2. Sorts them right-to-left / top-to-bottom (in-memory — no ECS involvement).
3. Walks the sorted list, maintaining a `HashSet` of occupied cells and
   computing each NPC's next position.
4. Writes updated `Position` components back via `query.get_mut(entity)`.
5. Renders the new grid to stdout using ANSI escape sequences.

### Why headless?

The simulation is a pure data problem: no sprites, no window, no GPU.
`MinimalPlugins` (with `ScheduleRunnerPlugin::run_loop`) gives us the ECS
scheduler, the `Time` system, and a 50 ms poll loop — nothing more.  This
keeps compile times reasonable and keeps the architecture honest: the ECS
provides state management and scheduling; rendering is a plain string write.

---

## 2. Rationale Behind Key Architectural Decisions

### Single `Position` component, no velocity or direction

NPCs have no persistent velocity, path, or intent — each turn their direction
is chosen fresh.  Storing a direction component would be wasteful state that
is never read between turns.

### In-memory sort before writing back to ECS

Bevy's query API does not allow stable iteration in a user-defined order while
mutably borrowing components.  The solution is a two-phase approach:

1. **Collect** — immutable borrow, materialize a `Vec<(Entity, x, y)>` and
   release the borrow.
2. **Sort + process** — pure in-memory work, no ECS involvement.
3. **Write back** — one `get_mut` per entity.

This is idiomatic Bevy and avoids unsafe code.

### `HashSet` occupancy, rebuilt every turn

A `HashSet<(usize, usize)>` is rebuilt at the start of each `tick_turn` call
and mutated as NPCs move.  Alternatives considered:

| Option | Problem |
|--------|---------|
| Per-turn query scan | O(N) scan to check each candidate cell |
| Persistent occupancy resource | Requires careful sync on every position change |
| Per-cell entity marker | Spawning/despawning entities for occupancy is slow |

Rebuilding costs O(N) but then each lookup/insert is O(1) average.  For
N = 1 000 NPCs and a 100 × 100 grid this is negligible within a 1-second turn.

### "Re-roll" implemented as uniform pick from free neighbours

The specification says "re-roll until a valid cell is found."  Repeated random
selection with replacement from all 8 directions produces a uniform
distribution over the *free* neighbours — identical to selecting uniformly from
the pre-filtered free list.  Pre-filtering avoids the theoretical infinite-loop
risk at high densities and is O(free) rather than expected O(8 / free_count).

### `ScheduleRunnerPlugin::run_loop(50 ms)`

Without a window, Bevy's default headless loop spins as fast as the CPU allows.
A 50 ms poll interval costs one `Timer::tick` call per loop and keeps CPU usage
near zero between turns while still waking within 50 ms of the timer expiring.

---

## 3. ECS vs. Object-Oriented Design

### What an OOP version would look like

```
class Grid {
    cells: Vec<Vec<Option<Arc<Npc>>>>
    npcs:  Vec<Arc<Npc>>
    fn tick(&mut self) { ... }
}

class Npc {
    x: usize, y: usize
    fn move_random(&mut self, grid: &mut Grid) { ... }
}
```

Each `Npc` owns its state, `Grid` owns the `Npc` list, and `tick` drives
everything through method calls.

### Comparison

| Criterion | ECS (this implementation) | OOP |
|-----------|--------------------------|-----|
| **Data layout** | Components stored in contiguous typed arrays (archetypes); cache-friendly for bulk iteration | Objects scattered in heap; pointer chasing per entity |
| **Parallelism** | System dependency graph enables auto-parallelism across independent systems | Shared mutable state (`Grid`, `Npc`) requires explicit locking |
| **Testability** | Pure functions (`valid_neighbors`, `render_grid`) are extracted and tested independently; ECS state can be set up in a minimal `App` | Methods are coupled to object graphs; unit-testing `Npc::move_random` requires a `Grid` stub |
| **Flexibility** | Adding a component (e.g. `Health`, `Team`) requires no changes to existing systems | Adding a field to `Npc` cascades through the class hierarchy |
| **Readability** | Systems make data dependencies explicit through query signatures | Behaviour is co-located with data; easier to read for small models |
| **Boilerplate** | More upfront ceremony (resources, components, system registration) | Less boilerplate for a small project |
| **Ownership** | Bevy enforces Rust borrow rules at the system level, preventing data races | Shared mutable `Grid` requires `Arc<Mutex<...>>` or unsafe |

### Verdict for this use case

At 1 000 NPCs the performance difference between ECS and OOP is immaterial.
The ECS approach pays off in:

- **Testability**: `valid_neighbors` and `render_grid` are pure functions with
  no object dependencies, making the test suite trivial to write.
- **Extensibility**: adding NPCs with different behaviours (e.g. a `Predator`
  marker that chases others) is a new component + new system, not a subclass
  hierarchy.
- **Safety**: Bevy's query system prevents two systems from mutating the same
  component simultaneously without explicit ordering.

The main cost of ECS here is the two-phase collect/write-back pattern required
to process NPCs in a defined order — something an OOP `tick` loop handles
trivially with a mutable `for npc in &mut self.npcs` loop.
