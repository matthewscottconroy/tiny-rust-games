# Architecture: ECS ASCII NPC Simulation

This document explains the Entity Component System (ECS) pattern from first
principles, using this simulation as the running example throughout. By the end
you should be able to:

1. Explain ECS clearly to a non-programmer.
2. Recognize ECS problems before you have written any code.
3. Design an ECS architecture for a new project from scratch.
4. Understand the concrete tradeoffs against object-oriented design.

---

## Table of Contents

1. [The Problem ECS Solves](#1-the-problem-ecs-solves)
2. [The Three Primitives](#2-the-three-primitives)
3. [The World: an ECS Database](#3-the-world-an-ecs-database)
4. [How This Simulation Maps onto ECS](#4-how-this-simulation-maps-onto-ecs)
5. [Resources: Global Singleton Data](#5-resources-global-singleton-data)
6. [Systems in Depth](#6-systems-in-depth)
7. [System Scheduling and Ordering](#7-system-scheduling-and-ordering)
8. [Key Architectural Decisions](#8-key-architectural-decisions)
9. [ECS vs Object-Oriented Design](#9-ecs-vs-object-oriented-design)
10. [Benefits of ECS](#10-benefits-of-ecs)
11. [Designing an ECS Architecture from Scratch](#11-designing-an-ecs-architecture-from-scratch)
12. [Glossary](#12-glossary)

---

## 1. The Problem ECS Solves

Before explaining what ECS *is*, it helps to understand what it is *for*.

### 1.1 The object-oriented trap

Suppose you are writing a game and you create a class `Character`:

```python
class Character:
    def __init__(self):
        self.x = 0
        self.y = 0
        self.hp = 100
        self.name = "Hero"

    def move(self, dx, dy):
        self.x += dx
        self.y += dy

    def take_damage(self, amount):
        self.hp -= amount
```

This works fine. Now you add an `Enemy` class. Enemies have positions and
hit points, and they can move, so you factor out a shared base class:

```python
class Actor:
    def move(self, dx, dy): ...
    def take_damage(self, amount): ...

class Character(Actor): ...
class Enemy(Actor): ...
```

Now you add a `Chest` — it has a position but does not move. It takes damage
(it can be smashed). Does it extend `Actor`? Partially. You add a `Wall` — it
has a position but cannot move or take damage. A `Projectile` — it moves but
has no hit points. A `ParticleEffect` — it moves, fades out, and vanishes.

By the time you have fifty entity types, your inheritance tree looks like a
bowl of spaghetti. Features belong to some classes but not others. Sharing
behaviour requires either deep inheritance (fragile) or duplicating code.

This is called the **fragile base class problem**, and it is endemic in
object-oriented game code.

### 1.2 The composition alternative

The core insight of ECS is:

> **Do not define what a thing is. Define what a thing has.**

Instead of asking "what kind of object is this?", you ask "what data does this
thing carry?" A chest has a position. A character has a position *and* hit
points *and* a name. A projectile has a position *and* a velocity.

An object's behaviour emerges from the combination of data it carries, not
from where it sits in an inheritance tree.

This is **composition over inheritance**, taken to its logical conclusion.

### 1.3 The ECS answer

ECS takes composition further by *also* separating the data from the code that
operates on it:

- **Data** lives in **components** attached to **entities**.
- **Logic** lives in **systems** that query for entities with specific
  component combinations.

No entity object owns any logic. No system owns any state. The result is a
clean separation between *what exists* (entities + components) and *what
happens* (systems).

---

## 2. The Three Primitives

ECS has exactly three building blocks. Everything else is derived from them.

### 2.1 Entity

An **entity** is nothing more than a unique integer ID.

```
Entity 0    ← an NPC
Entity 1    ← an NPC
Entity 2    ← an NPC
...
Entity 999  ← an NPC
```

An entity has no fields, no methods, no type. It is a key into the ECS
database — a handle that lets you look up what components a particular "thing"
currently has. In Bevy, an `Entity` is a 64-bit integer with some metadata
bits packed in.

**Analogy**: an entity is like a row number in a spreadsheet. The row itself
stores no data; you use the row number to find the data in the columns.

### 2.2 Component

A **component** is a plain data struct attached to an entity. It has fields
(the data) but no game-logic methods. It does not know which entity it belongs
to and does not know about other components.

In this simulation:

```rust
// A component. Just data. No logic.
#[derive(Component)]
pub struct Position {
    pub x: usize,
    pub y: usize,
}

// A marker component. Zero data. Presence alone carries meaning.
#[derive(Component)]
pub struct Npc;
```

`Position` carries two integers. `Npc` carries nothing — its presence on an
entity is enough to mean "this entity is an NPC."

Components are the *columns* of the ECS spreadsheet. Every column stores a
different kind of data, and a given row (entity) can have data in any
combination of columns.

**What components should NOT contain:**

- Methods that modify other components
- References to other entities (use IDs if needed)
- Logic about when or how to change their own values

**What components SHOULD contain:**

- Plain data: numbers, strings, booleans, enums, vecs of scalars
- Derived types that are also plain data

### 2.3 System

A **system** is a plain function. It takes a set of *queries* and *resources*
as parameters and operates on the matching data. It stores no state between
calls — any persistent state must live in a component or resource.

In Bevy, systems declare what they need through their function parameters:

```rust
fn move_npcs(
    mut query: Query<(Entity, &mut Position), With<Npc>>,  // entities
    mut grid:  ResMut<OccupancyGrid>,                      // resource
    mut rng:   ResMut<Rng>,                                // resource
    mut turn:  ResMut<TurnCount>,                          // resource
) {
    // ... reads and writes data, stores nothing locally between calls
}
```

Bevy reads the parameter types at registration time and injects the right
data from the World every time the system runs. The system does not call the
World directly — the framework takes care of that.

**What systems should NOT do:**

- Maintain local state between calls (use a Resource instead)
- Make assumptions about what other systems ran before them (use ordering)
- Contain data (they are functions, not classes)

**What systems SHOULD do:**

- Query for entities with specific component combinations
- Read and write component data
- Read and write resources
- Spawn or despawn entities via `Commands`

---

## 3. The World: an ECS Database

The **World** is the container that holds everything: all entities, all
component data, all resources. You can think of it as a relational database
with one table per component type.

### 3.1 The sparse component table

Imagine the World as a two-dimensional table:

```
Entity │ Position │ Npc │ Health │ Sprite │ …
───────┼──────────┼─────┼────────┼────────┼──
  0    │  (12,4)  │  ✓  │        │        │
  1    │  (7, 8)  │  ✓  │        │        │
  2    │  (50,50) │  ✓  │        │        │
 …     │  …       │ …   │  …     │  …     │
 999   │  (33,71) │  ✓  │        │        │
```

Each row is an entity. Each column is a component type. A cell contains data
if that entity has that component, and is empty otherwise.

In this simulation, every entity has both `Position` and `Npc`. In a more
complex game you might have entities with `Position` but no `Npc` (walls,
items) or `Npc` but no `Position` (abstract game-state entities).

### 3.2 Archetypes

Modern ECS engines including Bevy don't actually store a sparse table —
instead they group entities that share exactly the same set of component types
into **archetypes**. All entities with `{Position, Npc}` live together in one
archetype; entities with `{Position, Health, Sprite}` live in another.

Within an archetype, each component type is stored as a tightly-packed array:

```
Archetype {Position, Npc}:
  Position array: [(12,4), (7,8), (50,50), …, (33,71)]
  Npc array:      [(), (), (), …, ()]           ← zero-size, not really stored
```

This layout is the reason ECS is cache-friendly. When a system iterates all
`Position` components, it reads them sequentially from one contiguous array.
Modern CPUs are extremely fast at sequential memory access and very slow at
random pointer-chasing (the pattern produced by OOP linked structures).

### 3.3 Queries

A **query** is a filter on the World table. `Query<&Position, With<Npc>>`
means: "give me read access to the `Position` component of every entity that
has both `Position` and `Npc`."

Queries can express:

| Syntax | Meaning |
|--------|---------|
| `Query<&Position>` | Read `Position` from every entity that has one |
| `Query<&mut Position>` | Write `Position` |
| `Query<(&Position, &Health)>` | Read both fields, only on entities that have both |
| `Query<&Position, With<Npc>>` | Read `Position`, but only on entities that also have `Npc` |
| `Query<&Position, Without<Wall>>` | Read `Position`, excluding entities with `Wall` |
| `Query<(Entity, &Position)>` | Include the entity ID alongside its `Position` |

---

## 4. How This Simulation Maps onto ECS

Here is the complete mapping for this demo.

### 4.1 What are the entities?

Each NPC is one entity. There are N entities in the simulation, where N is the
number you enter at startup. Each entity has a unique integer ID assigned by
the World when it is spawned.

There are no other entities in this simulation — no walls, no items, no camera.
(The grid boundaries are enforced by `valid_neighbors`, not by wall entities.)

### 4.2 What are the components?

| Component | Data | Meaning when present |
|-----------|------|----------------------|
| `Position { x, y }` | Two `usize` values | "This entity occupies grid cell (x, y)" |
| `Npc` | Nothing (zero-size) | "This entity is an NPC" |

Only two components are needed because all NPCs are identical — they have
a position and they move. A more complex game would add components for health,
faction, AI state, inventory, and so on.

### 4.3 What are the systems?

| System | Schedule | What it does |
|--------|----------|--------------|
| `setup` | `Startup` | Spawns N entities with random positions |
| `move_npcs` | `Update` | Advances every NPC by one step |
| `render` | `Update` | Draws the grid to stdout |

`Startup` runs exactly once when the app launches. `Update` runs once per
turn, driven by `ScheduleRunnerPlugin::run_loop(turn_duration)`.

### 4.4 System data flow

```
         ┌─────────────────────────────────────────────┐
         │                  WORLD                      │
         │                                             │
         │  Entities: 0, 1, 2, …, N-1                  │
         │  Position components: [(12,4), (7,8), …]   │
         │  Npc components: [(), (), …]                │
         │                                             │
         │  Resources:                                 │
         │    SimConfig      (read by setup, render)   │
         │    OccupancyGrid  (read/write by move_npcs) │
         │    Rng            (write by setup, move)    │
         │    TurnCount      (write by move, render)   │
         └─────────────────────────────────────────────┘
              ↑ spawn         ↑ read/write    ↑ read
              │               │               │
           [setup]       [move_npcs]       [render]
```

Arrows go through the World — systems never call each other directly.
The World mediates all communication.

---

## 5. Resources: Global Singleton Data

A **resource** is a value that belongs to the simulation as a whole rather
than to any individual entity. Each resource type can exist at most once in
the World. Systems request resources by adding `Res<T>` (immutable) or
`ResMut<T>` (mutable) parameters.

### 5.1 Resources in this simulation

**`SimConfig`** — the user's inputs (width, height, NPC count, turn duration).
Read-only after startup. Any system can request it and know the grid size.

**`OccupancyGrid`** — a flat `Vec<bool>` where index `y * width + x` is
`true` when cell `(x, y)` is occupied. This resource is the performance
optimisation that makes O(1) neighbour checks possible. Without it, every NPC
move would require scanning all other NPC positions.

**`Rng`** — the pseudo-random number generator state. It is a resource rather
than a local variable so that the same RNG sequence continues across multiple
turns. It is mutable (`ResMut<Rng>`) because consuming a random number
advances the state.

**`TurnCount`** — a simple counter. Written by `move_npcs` at the start of
each turn; read by `render` to display the current turn number.

### 5.2 When to use a resource vs a component

| Use a Resource when... | Use a Component when... |
|------------------------|-------------------------|
| The data belongs to the simulation, not to any entity | The data belongs to a specific entity |
| There is exactly one of it | There can be many instances |
| Multiple systems need to share it | It travels with an entity through systems |
| Example: grid dimensions, RNG state, score | Example: position, health, velocity |

---

## 6. Systems in Depth

### 6.1 `setup` — the startup system

```rust
fn setup(
    mut commands: Commands,
    config: Res<SimConfig>,
    mut grid: ResMut<OccupancyGrid>,
    mut rng: ResMut<Rng>,
) {
    let count = config.npc_count.min(config.width * config.height);
    let mut placed = 0;
    while placed < count {
        let x = rng.range(config.width);
        let y = rng.range(config.height);
        if !grid.is_occupied(x, y) {
            grid.set(x, y, true);
            commands.spawn((Position { x, y }, Npc));
            placed += 1;
        }
    }
}
```

`commands.spawn((Position { x, y }, Npc))` creates a new entity with two
components attached. The entity ID is returned but we do not need it here —
the query in `move_npcs` will find these entities automatically.

`Commands` is a queue of deferred operations. The actual entity is not created
until after the system returns; Bevy flushes the command queue between systems.

### 6.2 `move_npcs` — the core simulation system

This system is the most complex, so its structure deserves careful attention.

```rust
fn move_npcs(
    mut query: Query<(Entity, &mut Position), With<Npc>>,
    mut grid: ResMut<OccupancyGrid>,
    mut rng: ResMut<Rng>,
    mut turn: ResMut<TurnCount>,
) {
```

The parameter `Query<(Entity, &mut Position), With<Npc>>` tells Bevy:
*"Give me mutable access to the Position of every entity that has both
Position and Npc."* Bevy will refuse to compile a system that requests two
conflicting mutable borrows of the same data.

**Step 1 — snapshot and sort:**

```rust
let mut order: Vec<(Entity, usize, usize)> = query
    .iter()
    .map(|(e, p)| (e, p.x, p.y))
    .collect();
order.sort_by(|a, b| process_order(&(a.1, a.2), &(b.1, b.2)));
```

We cannot sort a live query iterator because sorting requires random access.
We collect into a `Vec` first (a snapshot), then sort. This also frees the
query borrow so we can call `query.get_mut(entity)` in the loop below.

**Step 2 — shuffle neighbours and find a free cell:**

```rust
let mut neighbors = valid_neighbors(old_x, old_y, grid.width, grid.height);
for i in (1..neighbors.len()).rev() {
    let j = rng.range(i + 1);
    neighbors.swap(i, j);
}
if let Some(&(nx, ny)) = neighbors.iter().find(|&&(nx, ny)| !grid.is_occupied(nx, ny)) {
```

Fisher-Yates produces a uniformly shuffled list. Taking the first free element
is equivalent to re-rolling a random direction until a free cell is found, but
without any possibility of running forever when the grid is very full.

**Step 3 — commit the move:**

```rust
    grid.set(old_x, old_y, false);
    grid.set(nx, ny, true);
    if let Ok((_, mut pos)) = query.get_mut(entity) {
        pos.x = nx;
        pos.y = ny;
    }
```

The occupancy map is updated first so that subsequent NPCs see the correct
state. `query.get_mut(entity)` retrieves mutable access to one specific
entity's `Position` by its ID.

### 6.3 `render` — the output system

```rust
fn render(
    query: Query<&Position, With<Npc>>,
    config: Res<SimConfig>,
    turn: Res<TurnCount>,
) {
    let positions: Vec<(usize, usize)> = query.iter().map(|p| (p.x, p.y)).collect();
    let frame = render_grid(&positions, config.width, config.height);
    print!("\x1b[H{frame}...");
}
```

This system is **read-only** — it takes no mutable parameters. In a multi-
threaded Bevy app, Bevy could run this system in parallel with other read-only
systems. Its only job is to observe state and produce output; it never
changes anything.

The `\x1b[H` ANSI escape sequence moves the cursor to the top-left of the
terminal, allowing the frame to overwrite the previous one without clearing.

---

## 7. System Scheduling and Ordering

### 7.1 Schedules

Bevy divides execution into **schedules** — named groups of systems that run
together. The two used in this demo:

- **`Startup`** — runs once, immediately after the app initialises. Contains
  `setup`.
- **`Update`** — runs once per turn, on the interval set by
  `ScheduleRunnerPlugin::run_loop(turn_duration)`. Contains `move_npcs` and
  `render`.

### 7.2 Ordering within a schedule

By default, systems within the same schedule may run in any order (and even in
parallel on different threads). To enforce order, we use `.chain()`:

```rust
.add_systems(Update, (move_npcs, render).chain())
```

`.chain()` means: run `move_npcs` first, wait for it to complete, then run
`render`. Without this, the render system might execute before NPCs have moved.

### 7.3 Why does ordering matter?

Consider two systems, A and B, that both write to `ResMut<Score>`. If they
run concurrently they will produce a data race. Bevy detects this at
registration time and will either:

1. Automatically serialize them (run one before the other), or
2. Panic with an error if the conflict is ambiguous and you haven't specified
   an order.

Explicit ordering with `.chain()` or `.before()`/`.after()` removes the
ambiguity and makes the execution model easier to reason about.

---

## 8. Key Architectural Decisions

### 8.1 Why `OccupancyGrid` as a separate Resource?

The naive approach to "is cell (x,y) occupied?" is to scan all NPCs:

```rust
let occupied = query.iter().any(|p| p.x == nx && p.y == ny);
```

With 1000 NPCs, each NPC move requires up to 8 neighbour checks, each
requiring up to 1000 comparisons. That is 8,000,000 comparisons per turn —
O(N²) in the number of NPCs.

`OccupancyGrid` is a flat boolean array. A check costs one array index
calculation and one memory read — O(1). Total cost per turn: O(N) to move
all NPCs, O(W×H) to render. For the default 100×100 grid with 1000 NPCs,
this is roughly 100,000× faster than the naive approach.

The tradeoff is that `OccupancyGrid` must be kept in sync with the `Position`
components. Any system that modifies `Position` must also update the grid.
This is a deliberate coupling; the performance benefit justifies it.

### 8.2 Why snapshot-then-sort in `move_npcs`?

The specification requires NPCs to move in a specific order: top-to-bottom,
right-to-left within a row. But Bevy query iteration order is undefined —
entities may come out in any order depending on archetype layout.

The snapshot (`collect()`) converts the query into an owned `Vec` that we can
sort. It also releases the query's shared borrow over all `Position`
components, making it safe to call `query.get_mut(entity)` inside the loop.

The cost of the snapshot is O(N) memory allocation per turn, which is
acceptable. The sort is O(N log N).

### 8.3 Why extract pure functions?

`valid_neighbors`, `render_grid`, `process_order`, and `lcg_next` take only
plain Rust types. They have no Bevy dependencies and no side-effects.

This matters for two reasons:

1. **Testability** — you can test a pure function with a simple unit test. You
   cannot directly unit-test a Bevy system without setting up a full App and
   World. The 21 tests in this codebase test the pure functions exhaustively;
   the systems are covered by running the simulation.

2. **Readability** — the algorithm (e.g., neighbour enumeration, grid rendering)
   is visible in isolation, without the ECS machinery around it.

### 8.4 Why `MinimalPlugins` instead of `DefaultPlugins`?

`DefaultPlugins` opens a window, initialises the GPU, loads audio, and sets
up a rendering pipeline. None of that is needed for a terminal simulation.
`MinimalPlugins` provides only the ECS scheduler, time, and task pool — the
bare minimum to drive the `Update` schedule.

`ScheduleRunnerPlugin::run_loop(duration)` replaces the vsync loop with a
fixed-interval timer. Without it, `Update` would spin as fast as possible,
using 100% of one CPU core.

### 8.5 Why Fisher-Yates shuffle instead of random re-roll?

The specification says: pick a random direction, re-roll if occupied. The
re-roll approach works but has a subtle flaw when the grid is nearly full:
if 7 of 8 neighbours are occupied, each re-roll has only a 1-in-8 chance of
picking the free cell. On average you need 8 rolls to find it.

Fisher-Yates shuffles the neighbour list in O(K) time (K ≤ 8) and then takes
the first free cell in one linear scan. No redundant rolls. The probability
distribution of the chosen cell is identical.

---

## 9. ECS vs Object-Oriented Design

This is the most instructive comparison, so it includes concrete code for
both approaches applied to the same problem.

### 9.1 The OOP approach

A natural object-oriented design gives NPCs a class with data and behaviour:

```python
class NPC:
    def __init__(self, x, y):
        self.x = x
        self.y = y

    def move(self, grid, rng):
        neighbours = valid_neighbours(self.x, self.y, grid.width, grid.height)
        rng.shuffle(neighbours)
        for (nx, ny) in neighbours:
            if not grid.is_occupied(nx, ny):
                grid.set(self.x, self.y, False)
                grid.set(nx, ny, True)
                self.x, self.y = nx, ny
                break

class Simulation:
    def __init__(self, width, height, npc_count):
        self.grid = Grid(width, height)
        self.npcs = [NPC(random_x, random_y) for _ in range(npc_count)]

    def step(self):
        # Sort by processing order
        self.npcs.sort(key=lambda n: (n.y, -n.x))
        for npc in self.npcs:
            npc.move(self.grid, self.rng)

    def render(self):
        for npc in self.npcs:
            draw('@', npc.x, npc.y)
```

This is clean, intuitive, and immediately understandable.

### 9.2 The ECS approach (this codebase)

```rust
// Data: zero methods
#[derive(Component)]
pub struct Position { pub x: usize, pub y: usize }

#[derive(Component)]
pub struct Npc;

// Logic: no data owned by the function
fn move_npcs(
    mut query: Query<(Entity, &mut Position), With<Npc>>,
    mut grid: ResMut<OccupancyGrid>,
    mut rng: ResMut<Rng>,
) {
    let mut order: Vec<_> = query.iter().map(|(e, p)| (e, p.x, p.y)).collect();
    order.sort_by(|a, b| process_order(&(a.1, a.2), &(b.1, b.2)));
    for (entity, old_x, old_y) in order {
        // … move logic …
    }
}
```

### 9.3 Side-by-side comparison

| Property | OOP | ECS |
|----------|-----|-----|
| **Data location** | Inside object instances | In component arrays in the World |
| **Behaviour location** | Methods on objects | In standalone system functions |
| **Adding a feature** | Modify the class or add a subclass | Add a new component and/or system |
| **Composition** | Inheritance or mixins | Any combination of components |
| **Shared state** | Passed as arguments or via globals | Accessed via Resources |
| **Memory layout** | Objects scattered in heap memory | Components packed in contiguous arrays |
| **Parallelism** | Requires locking (shared mutable refs) | Framework handles locking via borrow analysis |
| **Testing** | Must instantiate objects | Test pure functions independently |
| **Boilerplate** | Low | Higher (framework integration) |

### 9.4 OOP pros and cons

**Pros:**
- Intuitive. Objects model the real world directly.
- Lower ceremony for small projects.
- Easy to understand the relationship between data and the code that uses it.
- Language support is mature and widely understood.

**Cons:**
- Inheritance hierarchies become rigid and hard to refactor as requirements change.
- Shared mutable state (objects calling methods on other objects) creates
  implicit dependencies that are hard to track.
- Object instances are typically scattered in heap memory, causing cache misses
  when iterating large collections.
- Adding a new capability to some-but-not-all objects requires refactoring the
  hierarchy.
- Difficult to run concurrently without explicit locking.

### 9.5 ECS pros and cons

**Pros:**
- Adding a feature is additive: add a component, add a system. Nothing else
  changes.
- Memory layout is cache-friendly: iterating 1000 `Position` components reads
  1000 × 8 bytes sequentially.
- Parallelism is structurally safe: the framework knows which systems read and
  write which components and can schedule them accordingly.
- Systems are testable in isolation via pure helper functions.
- Composition is trivial: attach any combination of components to any entity.
- No inheritance hierarchy to maintain.

**Cons:**
- Higher upfront conceptual overhead.
- More framework boilerplate (registering systems, resources, events).
- Queries are indirected through the framework rather than direct method calls.
- The separation of data and logic can make it harder to see the full picture
  of what a specific entity *does* just by reading one file.
- Not all problems map naturally onto entities and components (UI, dialog
  trees, and configuration are common misfits).

### 9.6 When ECS shines and when it doesn't

ECS is best suited to problems where:

- You have many instances of similar-but-not-identical things (enemies, particles, items).
- Behaviour composes from many small independent pieces.
- Performance at scale matters (1000+ entities updated per frame).
- You expect requirements to change and want to add features without refactoring.

OOP is better for:

- A small number of complex, highly unique objects with intricate internal logic.
- Problems with deep sequential dependencies (each step builds tightly on the last).
- Teams unfamiliar with ECS (the learning curve is real).
- Domains where the "object as a model of a real-world thing" metaphor works well.

---

## 10. Benefits of ECS

### 10.1 Cache efficiency

Modern CPUs are approximately 200× faster at sequential memory access than at
pointer-chasing. OOP objects are individually heap-allocated and scattered in
memory. Iterating 1000 NPCs means following 1000 pointers to 1000 random
memory locations.

In ECS, all `Position` components live in one contiguous array:

```
[Position(12,4), Position(7,8), Position(50,50), …, Position(33,71)]
 ←─────────────────────────── one contiguous array ───────────────────────→
```

Iterating that array is a sequential scan. The CPU prefetcher predicts the
next access and loads it before you ask for it. This is 10–100× faster in
practice for large entity counts.

### 10.2 Structural parallelism

Bevy analyses system parameters at registration time and builds a dependency
graph. Systems that do not conflict (e.g., one reads `Position`, another reads
`Health`, neither writes the other's data) are automatically scheduled in
parallel on different threads. You get parallelism for free, as a consequence
of the design, without writing a single `Mutex` or `Arc`.

Systems that *do* conflict (e.g., both write `Position`) are serialized. The
framework decides the order based on your `.before()`/`.after()` annotations
or its own heuristics.

### 10.3 Composability

In OOP, adding a "poison" status effect to some enemies requires modifying the
`Enemy` class, or creating a `PoisonedEnemy` subclass, or adding a nullable
`poison_timer: Option<f32>` field to every enemy whether they're poisoned or not.

In ECS: define a `Poisoned { timer: f32 }` component. Spawn it on enemies that
are currently poisoned. Write a `tick_poison` system that queries for entities
with both `Health` and `Poisoned`. Remove `Poisoned` when the timer expires.

The `Enemy` type does not change. The `Health` system does not change. Zero
existing code is modified. You only *add*.

### 10.4 Decoupling

Systems interact only through the World (shared components and resources). A
system does not call another system, does not hold a reference to another
system, and does not know whether another system exists. This means:

- You can add, remove, or replace systems without changing any other system.
- Systems are individually reusable in other projects.
- Bugs in one system cannot directly corrupt another system's state (only
  indirectly, through shared components — and the framework enforces borrow
  rules there).

### 10.5 Testability

Pure functions extracted from systems (like `valid_neighbors`, `render_grid`,
`process_order`) carry no Bevy dependencies. You can test them directly:

```rust
#[test]
fn center_cell_has_eight_neighbors() {
    assert_eq!(valid_neighbors(5, 5, 10, 10).len(), 8);
}
```

No app setup. No resource injection. No async machinery. Just a function call
and an assertion.

---

## 11. Designing an ECS Architecture from Scratch

Follow these five steps whenever you start a new ECS project.

### Step 1: Identify your entities

Ask: "What are the distinct **things** in my simulation or game?"

Write them down as nouns:
- NPC, wall, item, particle, projectile, camera, player, enemy, chest, door

Each noun is likely an entity type. Do not yet decide what data they have.

For this simulation: the answer is simply "NPC". There is exactly one kind of
thing.

### Step 2: Identify your components

For each entity type, ask: "What data does this thing **have**?"

Ignore behaviour completely at this step. List only data:

```
NPC has:
  - a grid position (x, y)
  - it is an NPC  ← presence-only, no data needed
```

Look for data shared across entity types — that becomes a component that
multiple entity types can carry. In a larger game:

```
Position  ← NPCs, walls, items, projectiles all have it
Health    ← NPCs, chests, doors — but not walls or projectiles
Velocity  ← NPCs, projectiles — not walls, items, or doors
Faction   ← NPCs only
```

A `Wall` has `Position` but no `Health`, `Velocity`, or `Faction`. An `Item`
has `Position` but no `Health` or `Velocity`. No class hierarchy needed — just
attach the relevant components.

Rule of thumb: a component should contain the smallest coherent unit of data.
`Position` and `Health` should be separate components, not a combined
`ActorStats` struct. You might want to query for one without the other.

### Step 3: Identify your systems

Ask: "What **operations** need to happen each frame (or once at startup)?"

Write them as verbs:
- spawn entities, move NPCs, check collisions, apply damage, play sounds,
  render the screen, tick timers, remove dead entities

Each verb is likely a system. For each system, note:
- What component data does it read?
- What component data does it write?
- What resources does it need?

For this simulation:

```
setup:      reads SimConfig, writes OccupancyGrid, spawns Position+Npc
move_npcs:  reads/writes Position, reads/writes OccupancyGrid, reads/writes Rng
render:     reads Position, reads SimConfig, reads TurnCount
```

### Step 4: Identify your resources

Resources are global data that is not per-entity. Ask:
"What data does the simulation own, rather than any particular entity?"

Red flags that something should be a Resource:
- "There is exactly one of these" (grid dimensions, score, game state)
- "Many systems need access to this" (random number generator, asset store)
- "This is configuration that never changes" (SimConfig)
- "This is an acceleration structure derived from entity data" (OccupancyGrid)

For this simulation: `SimConfig`, `OccupancyGrid`, `Rng`, `TurnCount`.

### Step 5: Define system ordering

Ask: "Which systems depend on the output of other systems?"

Draw arrows:
```
setup → move_npcs → render
```

`render` must see the positions after `move_npcs` has run, so it must come
after. `setup` must complete before `move_npcs` can process any entities.

In Bevy: startup systems run before update systems automatically. Within
`Update`, use `.chain()` to enforce ordering between systems in the same
schedule.

### 11.1 Worked example: adding a new feature

Suppose you want some NPCs to be "scared" — they always move away from the
nearest other NPC rather than randomly.

**Step 1:** The entity type is still NPC. No new entity type needed.

**Step 2:** Add a marker component: `#[derive(Component)] struct Scared;`.
Attach it to some NPCs at spawn time.

**Step 3:** Add a system: `fn move_scared(query: Query<(Entity, &mut Position), (With<Npc>, With<Scared>)>, ...)`. This system queries for entities that have *all three* of `Position`, `Npc`, and `Scared`.

**Step 4:** No new resources needed (the existing `Rng` and `OccupancyGrid` can be reused).

**Step 5:** `move_scared` must run before `render`, and in the same turn as `move_npcs`. They do not conflict (they could query different entities via filter combinations) but you may want to run them sequentially if they both modify `OccupancyGrid`.

Result: you added one component struct, one system function, and tagged some
entities at spawn time. You modified zero existing systems and zero existing
components.

---

## 12. Glossary

**Archetype** — A group of entities that all share exactly the same set of
component types. ECS engines store entities in archetypes for memory
efficiency. When you attach or detach a component from an entity, it moves
to a different archetype.

**Bundle** — In Bevy, a group of components that are spawned together.
`commands.spawn((Position { x, y }, Npc))` is a tuple bundle. Bevy
decomposes it into individual components stored in the archetype.

**Commands** — A queue of deferred World operations (spawn, despawn, insert
component, remove component). Systems append to the queue; the World flushes
it between systems. This prevents mutation during iteration.

**Component** — Plain data attached to an entity. No logic, no methods that
modify other components.

**Entity** — An opaque integer ID. The handle you use to look up components
in the World.

**ECS** — Entity Component System. An architecture pattern that separates
*what exists* (entities + components) from *what happens* (systems).

**Query** — A filtered view of the World table. Specifies which component
types to access (and whether to read or write them) and optional filter
predicates (`With<T>`, `Without<T>`).

**Resource** — A globally unique value in the World, not owned by any
entity. Accessed by systems via `Res<T>` (read) or `ResMut<T>` (write).

**Schedule** — A named group of systems that run together. `Startup` runs
once; `Update` runs every frame/turn.

**System** — A plain function that processes ECS data. Takes queries and
resources as parameters; stores no state of its own.

**World** — The container that holds all entities, components, and resources.
The single source of truth for simulation state.

---

## Summary

ECS is a pattern that separates **data** (components on entities) from
**logic** (systems). It solves the problems caused by deep class hierarchies
by making composition trivial: attach any combination of components to any
entity. Systems operate on whichever entities match their query, without
knowing anything about the others.

For this simulation:

- **1000 NPCs** are 1000 entities, each with a `Position` and an `Npc` tag.
- **Three systems** handle everything: setup, movement, rendering.
- **Four resources** hold global state: config, occupancy grid, RNG, turn
  counter.
- **Pure functions** implement the algorithms, keeping the ECS machinery
  separate from the testable logic.

The result is a simulation where adding a new behaviour (scared NPCs, fast
NPCs, coloured NPCs) requires only new components and new systems — zero
changes to existing code.
