//! ECS ASCII-grid NPC simulation — a reusable, headless Bevy plugin.
//!
//! This crate is a *building block*: drop [`EcsAsciiSimPlugin`] into any Bevy
//! app with `app.add_plugins(EcsAsciiSimPlugin)` and it spawns a population of
//! NPCs that wander an ASCII grid, one random step per turn, rendered to stdout.
//! Tune it through the [`EcsAsciiSimConfig`] resource without editing the
//! plugin's internals.
//!
//! The plugin is deliberately **host-agnostic**: it never adds `MinimalPlugins`
//! or `ScheduleRunnerPlugin`. The host app owns the bootstrap (headless runner,
//! turn timer) and decides how fast the `Update` schedule ticks — see
//! `main.rs`, which reads the grid dimensions and turn duration from stdin.
//!
//! ## ECS map for this demo
//!
//! | ECS primitive | Concrete thing in this demo                              |
//! |---------------|----------------------------------------------------------|
//! | **Entity**    | Each NPC — an opaque integer ID, nothing more            |
//! | **Component** | [`Position`] (x, y), [`Npc`] (presence = "is an NPC")    |
//! | **System**    | [`setup`], `move_npcs`, `render`                         |
//! | **Resource**  | [`EcsAsciiSimConfig`], [`OccupancyGrid`], [`Rng`], [`TurnCount`] |
//!
//! Every system is a plain function. All persistent state lives in components
//! or resources. No system owns any state.
//!
//! # Example
//! ```no_run
//! use bevy::app::ScheduleRunnerPlugin;
//! use bevy::prelude::*;
//! use std::time::Duration;
//! use ecs_ascii_sim::EcsAsciiSimPlugin;
//!
//! App::new()
//!     .add_plugins(
//!         MinimalPlugins.set(ScheduleRunnerPlugin::run_loop(Duration::from_secs(1))),
//!     )
//!     .add_plugins(EcsAsciiSimPlugin)
//!     .run();
//! ```

use bevy::prelude::*;
use std::io::{self, Write};

// ═══════════════════════════════════════════════════════════════════════════
// PLUGIN
// ═══════════════════════════════════════════════════════════════════════════

/// Bundles every system and resource for the ASCII NPC simulation.
///
/// Add it with `app.add_plugins(EcsAsciiSimPlugin)`. It does **not** add
/// `MinimalPlugins` or `ScheduleRunnerPlugin`, so the host app stays in control
/// of the headless runner and turn timing.
///
/// The plugin seeds [`OccupancyGrid`], [`Rng`], and [`TurnCount`] from the
/// [`EcsAsciiSimConfig`] resource during `build`, so a host that wants to
/// override the grid size or NPC count should
/// `insert_resource(EcsAsciiSimConfig { .. })` **before** adding the plugin.
pub struct EcsAsciiSimPlugin;

impl Plugin for EcsAsciiSimPlugin {
    fn build(&self, app: &mut App) {
        // Read (or default) the config so we can size the grid up front.
        let config = app
            .world_mut()
            .get_resource::<EcsAsciiSimConfig>()
            .copied()
            .unwrap_or_default();

        app.insert_resource(config)
            .insert_resource(OccupancyGrid::new(config.width, config.height))
            .insert_resource(Rng::new(config.seed))
            .init_resource::<TurnCount>()
            // Startup: runs once. Setup spawns all NPC entities.
            .add_systems(Startup, setup)
            // Update: runs every turn. `.chain()` guarantees move → render order.
            .add_systems(Update, (move_npcs, render).chain());
    }
}

// ═══════════════════════════════════════════════════════════════════════════
// CONFIGURATION
// ═══════════════════════════════════════════════════════════════════════════

/// Tunable parameters for the simulation. Override before adding the plugin,
/// e.g. `app.insert_resource(EcsAsciiSimConfig { width: 40, ..default() })`.
///
/// `main.rs` builds this from interactive stdin input; the [`Default`] values
/// below are the same defaults the prompt offers.
#[derive(Resource, Clone, Copy, Debug)]
pub struct EcsAsciiSimConfig {
    /// Grid width in cells (the X axis).
    pub width: usize,
    /// Grid height in cells (the Y axis).
    pub height: usize,
    /// How many NPCs to spawn (clamped to grid capacity).
    pub npc_count: usize,
    /// Seed for the deterministic pseudo-random number generator.
    pub seed: u64,
}

impl Default for EcsAsciiSimConfig {
    fn default() -> Self {
        Self { width: 100, height: 100, npc_count: 1_000, seed: 0xDEAD_BEEF }
    }
}

// ═══════════════════════════════════════════════════════════════════════════
// COMPONENTS
//
// Components are data-only structs with no methods and no logic.
// They describe *what an entity has*, not what it does.
// Logic belongs in systems.
// ═══════════════════════════════════════════════════════════════════════════

/// The grid cell an entity currently occupies.
///
/// This is pure, dumb data — two integers. There is no `move()` method here.
/// Movement belongs in the `move_npcs` system, which *reads* and *writes*
/// `Position` on many entities at once.
#[derive(Component, Clone, Copy, PartialEq, Eq, Debug)]
pub struct Position {
    pub x: usize,
    pub y: usize,
}

/// Marker component — a tag that means "this entity is an NPC".
///
/// It carries zero data. Its only job is to let systems write queries like
/// `Query<&Position, With<Npc>>` to select only NPC entities and ignore
/// everything else in the world.
#[derive(Component)]
pub struct Npc;

// ═══════════════════════════════════════════════════════════════════════════
// RESOURCES
//
// Resources are globally-unique values that belong to the simulation as a
// whole rather than to any individual entity. They are singletons managed
// by the ECS World.
// ═══════════════════════════════════════════════════════════════════════════

/// Flat boolean array: `cells[y * width + x]` is `true` when that cell is
/// occupied by any NPC.
///
/// Stored as a `Resource` so the movement system can answer "is cell (x,y)
/// free?" in O(1) without scanning all entities.
#[derive(Resource)]
pub struct OccupancyGrid {
    cells: Vec<bool>,
    pub width: usize,
    pub height: usize,
}

impl OccupancyGrid {
    pub fn new(width: usize, height: usize) -> Self {
        Self { cells: vec![false; width * height], width, height }
    }

    #[inline]
    pub fn is_occupied(&self, x: usize, y: usize) -> bool {
        self.cells[y * self.width + x]
    }

    #[inline]
    pub fn set(&mut self, x: usize, y: usize, occupied: bool) {
        self.cells[y * self.width + x] = occupied;
    }
}

/// Minimal linear-congruential pseudo-random number generator.
///
/// Stored as a `Resource` so every system that needs randomness can request
/// `ResMut<Rng>` without re-initialising or passing state around.
#[derive(Resource)]
pub struct Rng {
    state: u64,
}

impl Rng {
    pub fn new(seed: u64) -> Self {
        Self { state: seed }
    }

    pub fn next_u64(&mut self) -> u64 {
        self.state = lcg_next(self.state);
        self.state
    }

    /// Returns a value in `0..len`.
    pub fn range(&mut self, len: usize) -> usize {
        self.next_u64() as usize % len
    }
}

/// Monotonically increasing count of turns elapsed.
#[derive(Resource, Default)]
pub struct TurnCount(pub u64);

// ═══════════════════════════════════════════════════════════════════════════
// PURE HELPER FUNCTIONS
//
// These functions take only plain Rust types — no Bevy types, no ECS types.
// They contain no side-effects and no global state.
//
// Keeping logic here instead of inside systems has two advantages:
//   1. They are trivially unit-testable (see the #[cfg(test)] block below).
//   2. Reading them communicates the algorithm without ECS boilerplate.
// ═══════════════════════════════════════════════════════════════════════════

/// LCG step — advances the pseudo-random state by one step.
///
/// Constants from Knuth. The function is pure: the same input always
/// produces the same output.
pub fn lcg_next(s: u64) -> u64 {
    s.wrapping_mul(6_364_136_223_846_793_005)
        .wrapping_add(1_442_695_040_888_963_407)
}

/// Returns every in-bounds neighbour of `(x, y)` within a `w × h` grid.
///
/// Includes all 8 surrounding cells (cardinal + diagonal); cells near walls
/// return fewer neighbours because out-of-bounds positions are excluded.
pub fn valid_neighbors(x: usize, y: usize, w: usize, h: usize) -> Vec<(usize, usize)> {
    let (xi, yi, wi, hi) = (x as i64, y as i64, w as i64, h as i64);
    let mut out = Vec::with_capacity(8);
    for dy in -1i64..=1 {
        for dx in -1i64..=1 {
            if dx == 0 && dy == 0 {
                continue;
            }
            let nx = xi + dx;
            let ny = yi + dy;
            if nx >= 0 && nx < wi && ny >= 0 && ny < hi {
                out.push((nx as usize, ny as usize));
            }
        }
    }
    out
}

/// Renders a set of occupied positions onto a `w × h` character grid.
///
/// Returns a `String` where `@` marks an occupied cell and `.` marks an
/// empty one. Each row ends with `\n`. This function is pure: given the
/// same inputs it always returns the same string.
pub fn render_grid(occupied: &[(usize, usize)], w: usize, h: usize) -> String {
    let mut cells = vec![b'.'; w * h];
    for &(x, y) in occupied {
        if x < w && y < h {
            cells[y * w + x] = b'@';
        }
    }
    let mut out = String::with_capacity((w + 1) * h);
    for row in 0..h {
        let start = row * w;
        out.extend(cells[start..start + w].iter().map(|&c| c as char));
        out.push('\n');
    }
    out
}

/// Ordering function for NPC processing each turn.
///
/// Rules (matching the spec):
/// - Top-to-bottom within the grid   → ascending `y`
/// - Right-to-left within the same row → descending `x`
pub fn process_order(a: &(usize, usize), b: &(usize, usize)) -> std::cmp::Ordering {
    // First sort by row (y), then within a row sort by column descending (x).
    a.1.cmp(&b.1).then(b.0.cmp(&a.0))
}

// ═══════════════════════════════════════════════════════════════════════════
// SYSTEMS
//
// Systems are plain functions. Bevy inspects their parameter types and
// automatically injects the right data from the World before calling them.
//
// A system NEVER stores state in a local variable between calls. Every
// piece of persistent state is a Component or a Resource.
// ═══════════════════════════════════════════════════════════════════════════

/// Spawns the initial NPC population.
///
/// **ECS role — Startup system**: runs exactly once, immediately after the
/// World is ready. It uses `Commands` to create entities and attach
/// components to them, and `ResMut<OccupancyGrid>` to record which cells
/// are taken.
///
/// Notice that this system does not know what *other* systems exist. It only
/// writes to the resources and entity set it needs. That decoupling is a
/// core ECS property.
pub fn setup(
    mut commands: Commands,
    config: Res<EcsAsciiSimConfig>,
    mut grid: ResMut<OccupancyGrid>,
    mut rng: ResMut<Rng>,
) {
    let capacity = config.width * config.height;
    let count = config.npc_count.min(capacity);
    let mut placed = 0;

    while placed < count {
        let x = rng.range(config.width);
        let y = rng.range(config.height);
        if !grid.is_occupied(x, y) {
            grid.set(x, y, true);
            // `commands.spawn(bundle)` creates a new entity with the given
            // components attached. The entity ID is assigned by the World.
            commands.spawn((Position { x, y }, Npc));
            placed += 1;
        }
    }
}

/// Advances every NPC by one step.
///
/// **ECS role — Update system**: runs every turn. It queries all entities
/// that have both `Position` and `Npc` components, computes new positions,
/// and writes them back.
///
/// Processing order: top-to-bottom, right-to-left within a row.
/// Each NPC tries random free neighbours; skips its move if none are free.
fn move_npcs(
    mut query: Query<(Entity, &mut Position), With<Npc>>,
    mut grid: ResMut<OccupancyGrid>,
    mut rng: ResMut<Rng>,
    mut turn: ResMut<TurnCount>,
) {
    turn.0 += 1;

    // ── Step 1: snapshot ──────────────────────────────────────────────────
    // Collect (entity, x, y) before touching any positions. We need to sort
    // by processing order, and sorting requires ownership of the data.
    // We cannot sort while iterating a live query borrow.
    let mut order: Vec<(Entity, usize, usize)> = query
        .iter()
        .map(|(e, p)| (e, p.x, p.y))
        .collect();

    // ── Step 2: sort ──────────────────────────────────────────────────────
    // Top-to-bottom, right-to-left within a row. This ordering means that
    // an NPC moving rightward steps in front of the processing order; one
    // moving leftward steps behind it. Neither causes a position conflict
    // because OccupancyGrid always reflects the current state.
    order.sort_by(|a, b| process_order(&(a.1, a.2), &(b.1, b.2)));

    // ── Step 3: move ──────────────────────────────────────────────────────
    for (entity, old_x, old_y) in order {
        // Build the list of in-bounds neighbours.
        let mut neighbors = valid_neighbors(old_x, old_y, grid.width, grid.height);

        // Fisher-Yates shuffle for uniform random selection. This is
        // equivalent to "re-roll until a free cell is found" but avoids
        // the possibility of infinite retries when the grid is nearly full.
        let n = neighbors.len();
        for i in (1..n).rev() {
            let j = rng.range(i + 1);
            neighbors.swap(i, j);
        }

        // Take the first free neighbour. If none exist, the NPC stays.
        if let Some(&(nx, ny)) = neighbors.iter().find(|&&(nx, ny)| !grid.is_occupied(nx, ny)) {
            // Update the occupancy map first, then the component.
            grid.set(old_x, old_y, false);
            grid.set(nx, ny, true);

            // `query.get_mut(entity)` fetches mutable access to one specific
            // entity by its ID. This is safe here because we are not iterating
            // the query at this point (we collected into `order` above).
            if let Ok((_, mut pos)) = query.get_mut(entity) {
                pos.x = nx;
                pos.y = ny;
            }
        }
        // If no free neighbour exists, do nothing — the NPC skips this turn.
    }
}

/// Renders the full grid to stdout once per turn.
///
/// **ECS role — Update system**: runs after `move_npcs` every turn (enforced
/// by `.chain()` in the plugin). It reads `Position` from every NPC entity and
/// passes the coordinates to the pure `render_grid` function.
///
/// This system takes only *immutable* access (`Res`, `Query<&Position>`), so
/// Bevy can in principle run it in parallel with other read-only systems.
fn render(
    query: Query<&Position, With<Npc>>,
    config: Res<EcsAsciiSimConfig>,
    turn: Res<TurnCount>,
) {
    let positions: Vec<(usize, usize)> = query.iter().map(|p| (p.x, p.y)).collect();
    let frame = render_grid(&positions, config.width, config.height);

    // ANSI: move cursor to row 1 column 1 (home), then overwrite in-place.
    // This avoids clearing the terminal and prevents flicker.
    print!("\x1b[H{frame}Turn {:>6}   NPCs {:>5}   Grid {}×{}\n",
        turn.0, positions.len(), config.width, config.height);
    let _ = io::stdout().flush();
}

// ═══════════════════════════════════════════════════════════════════════════
// UNIT TESTS
//
// All tests are on pure functions — no Bevy types, no app setup required.
// ═══════════════════════════════════════════════════════════════════════════

#[cfg(test)]
mod tests {
    use super::*;

    // ── lcg_next ────────────────────────────────────────────────────────────

    #[test]
    fn lcg_advances_from_zero() {
        assert_ne!(lcg_next(0), 0);
    }

    #[test]
    fn lcg_produces_distinct_values() {
        let a = lcg_next(1);
        let b = lcg_next(a);
        assert_ne!(a, b);
    }

    #[test]
    fn lcg_is_deterministic() {
        assert_eq!(lcg_next(42), lcg_next(42));
    }

    // ── valid_neighbors ──────────────────────────────────────────────────────

    #[test]
    fn corner_top_left_has_three_neighbors() {
        assert_eq!(valid_neighbors(0, 0, 10, 10).len(), 3);
    }

    #[test]
    fn corner_bottom_right_has_three_neighbors() {
        assert_eq!(valid_neighbors(9, 9, 10, 10).len(), 3);
    }

    #[test]
    fn top_edge_center_has_five_neighbors() {
        assert_eq!(valid_neighbors(5, 0, 10, 10).len(), 5);
    }

    #[test]
    fn center_cell_has_eight_neighbors() {
        assert_eq!(valid_neighbors(5, 5, 10, 10).len(), 8);
    }

    #[test]
    fn neighbors_never_include_self() {
        let ns = valid_neighbors(3, 3, 10, 10);
        assert!(!ns.contains(&(3, 3)));
    }

    #[test]
    fn neighbors_stay_in_bounds() {
        for (x, y) in valid_neighbors(0, 0, 5, 5) {
            assert!(x < 5 && y < 5, "({x},{y}) is out of bounds");
        }
    }

    #[test]
    fn one_cell_grid_has_no_neighbors() {
        assert_eq!(valid_neighbors(0, 0, 1, 1).len(), 0);
    }

    // ── render_grid ──────────────────────────────────────────────────────────

    #[test]
    fn empty_grid_is_all_dots() {
        assert_eq!(render_grid(&[], 3, 2), "...\n...\n");
    }

    #[test]
    fn single_npc_renders_at_origin() {
        assert_eq!(render_grid(&[(0, 0)], 3, 1), "@..\n");
    }

    #[test]
    fn npc_at_mid_row() {
        assert_eq!(render_grid(&[(1, 0)], 3, 1), ".@.\n");
    }

    #[test]
    fn multiple_npcs_in_correct_positions() {
        assert_eq!(render_grid(&[(0, 0), (2, 0), (1, 1)], 3, 2), "@.@\n.@.\n");
    }

    #[test]
    fn out_of_bounds_npc_is_silently_ignored() {
        // Should not panic; the extra NPC just doesn't appear.
        let s = render_grid(&[(0, 0), (99, 99)], 3, 2);
        assert_eq!(s, "@..\n...\n");
    }

    // ── process_order ────────────────────────────────────────────────────────

    #[test]
    fn top_row_processed_before_bottom_row() {
        let mut v = vec![(0usize, 5usize), (0, 2), (0, 9)];
        v.sort_by(|a, b| process_order(a, b));
        let ys: Vec<usize> = v.iter().map(|p| p.1).collect();
        assert_eq!(ys, vec![2, 5, 9]);
    }

    #[test]
    fn within_same_row_right_to_left() {
        let mut v = vec![(1usize, 0usize), (7, 0), (3, 0), (5, 0)];
        v.sort_by(|a, b| process_order(a, b));
        let xs: Vec<usize> = v.iter().map(|p| p.0).collect();
        assert_eq!(xs, vec![7, 5, 3, 1]);
    }

    #[test]
    fn different_rows_sorted_by_row_first() {
        let a = (9usize, 0usize); // row 0, rightmost
        let b = (0usize, 1usize); // row 1, leftmost
        // a should come first (lower row)
        assert_eq!(process_order(&a, &b), std::cmp::Ordering::Less);
    }

    // ── OccupancyGrid ────────────────────────────────────────────────────────

    #[test]
    fn new_grid_is_entirely_free() {
        let g = OccupancyGrid::new(5, 5);
        assert!(!g.is_occupied(0, 0));
        assert!(!g.is_occupied(4, 4));
        assert!(!g.is_occupied(2, 3));
    }

    #[test]
    fn set_and_clear_a_cell() {
        let mut g = OccupancyGrid::new(5, 5);
        g.set(2, 3, true);
        assert!(g.is_occupied(2, 3));
        g.set(2, 3, false);
        assert!(!g.is_occupied(2, 3));
    }

    #[test]
    fn setting_one_cell_does_not_affect_neighbors() {
        let mut g = OccupancyGrid::new(5, 5);
        g.set(2, 2, true);
        assert!(!g.is_occupied(1, 2));
        assert!(!g.is_occupied(3, 2));
        assert!(!g.is_occupied(2, 1));
        assert!(!g.is_occupied(2, 3));
    }

    // ── EcsAsciiSimConfig ────────────────────────────────────────────────────

    #[test]
    fn config_default_matches_documented_values() {
        let c = EcsAsciiSimConfig::default();
        assert_eq!(c.width, 100);
        assert_eq!(c.height, 100);
        assert_eq!(c.npc_count, 1_000);
    }

    // ── ECS setup test ───────────────────────────────────────────────────────

    #[test]
    fn setup_spawns_requested_npc_count() {
        // Small grid so we can assert an exact count on a headless app.
        let mut app = App::new();
        app.add_plugins(MinimalPlugins)
            .insert_resource(EcsAsciiSimConfig { width: 5, height: 5, npc_count: 4, seed: 1 })
            .insert_resource(OccupancyGrid::new(5, 5))
            .insert_resource(Rng::new(1))
            .init_resource::<TurnCount>()
            .add_systems(Startup, setup);
        app.update();

        let mut q = app.world_mut().query::<&Npc>();
        assert_eq!(q.iter(app.world()).count(), 4);
    }

    #[test]
    fn plugin_seeds_grid_from_config() {
        // Demonstrates the building-block path: the plugin composes onto a
        // headless app and sizes its own resources from the config.
        let mut app = App::new();
        app.insert_resource(EcsAsciiSimConfig { width: 8, height: 6, npc_count: 3, seed: 7 })
            .add_plugins((MinimalPlugins, EcsAsciiSimPlugin));
        app.update();

        let grid = app.world().resource::<OccupancyGrid>();
        assert_eq!((grid.width, grid.height), (8, 6));

        let mut q = app.world_mut().query::<&Npc>();
        assert_eq!(q.iter(app.world()).count(), 3);
    }
}
