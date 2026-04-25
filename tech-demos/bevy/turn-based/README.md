# Turn-Based Combat

A demo of **initiative-ordered turn management** with action points: actors are sorted by their initiative score at game start, then take turns in order. On the player's turn, WASD moves (costs 1 AP) or bump-attacks an adjacent enemy (costs 2 AP). Enemies automatically step toward the player and attack if adjacent.

**WASD / Arrow keys** — move or attack. **SPACE** — end turn early.

---

## What This Is Illustrating

Turn-based game loop design — the pattern that underpins RPGs, roguelikes, strategy games, and card games. Rather than all actors updating simultaneously each frame, each actor gets a discrete turn in a fixed order. This lets the player take as long as they want to think, and gives the game full control over pacing and information presentation.

---

## Game Design Principle: Giving the Player Time to Think

Real-time games test reflexes. Turn-based games test decisions. The key design challenge is making turns feel fair and legible:

- **Initiative order** determines who acts first. Higher initiative = acting earlier = a meaningful character stat. The player can see that the fast enemy will move before the slow one and plan accordingly.
- **Action points (AP)** constrain what the player can do per turn. Moving twice costs 2 AP; attacking costs 2 AP; moving then attacking costs 3 AP. AP transforms each turn into a small resource-allocation puzzle: what is the highest-value use of my remaining AP?
- **Bump combat** (moving into an enemy to attack) keeps the input model simple — movement and attack are the same action depending on what is in the destination cell. This is the standard roguelike model and eliminates a separate "attack" button.

**Turn loop:**
1. Sort all actors by descending initiative → turn order list.
2. The first actor in the list takes their turn (player input or enemy AI).
3. When the turn ends (AP exhausted, SPACE pressed, or AI done), advance to the next actor.
4. Remove defeated actors from the order. Repeat until one side is eliminated.

---

## How Bevy Achieves It

**Pure initiative sort.** `sort_by_initiative(actors: &mut Vec<(usize, i32)>)` sorts a list of `(index, initiative)` pairs in descending order. It is completely independent of Bevy's ECS — indices rather than `Entity` IDs are used so the function can be called in unit tests without a `World`. The game builds the input list from a query at startup and stores the resulting `Vec<Entity>` in `TurnState`.

**`TurnState` resource models the loop.** `TurnState { order: Vec<Entity>, current_idx: usize, phase: TurnPhase }` is the complete turn loop state. `advance_turn` increments `current_idx`, removes dead actors from `order`, and sets `phase` based on whether the new current actor is a player or enemy. All turn transitions go through this single function.

**Borrow-safe mutation via sequential `get_mut` calls.** Bevy's borrow checker prevents holding two mutable query borrows simultaneously. The `run_enemy_turn` system is structured as two phases: a *data-collection phase* using immutable `actors.iter()` and `actors.get()` calls (which drop before the mutation phase begins), followed by a *mutation phase* that calls `actors.get_mut(entity)` for one entity at a time. This pattern — collect then mutate — is the standard way to avoid overlapping mutable borrows in Bevy ECS systems.

**`step_toward` and `manhattan` as pure helpers.** Enemy AI consists of two pure functions: `manhattan(a, b) -> i32` (total grid distance) and `step_toward(from, to) -> IVec2` (one-step move toward a target). These are independently testable and keep the AI logic readable. An enemy with 0 manhattan distance to the player attacks; otherwise it steps toward the player if the destination is unoccupied.

---

## Running

```
cargo run
```

Controls: **WASD / Arrow keys** — move or attack · **SPACE** — end turn.
