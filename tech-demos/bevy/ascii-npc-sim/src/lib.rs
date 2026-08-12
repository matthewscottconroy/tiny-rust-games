//! ASCII NPC simulation — a reusable, headless Bevy plugin.
//!
//! This crate is a *building block*: add [`AsciiNpcSimPlugin`] to a headless
//! Bevy app (`MinimalPlugins` + `ScheduleRunnerPlugin`) and it scatters NPCs
//! across a 2D grid, moves them one step per turn, and renders each frame to
//! stdout with ANSI escape codes — no window or GPU required.
//!
//! Each turn every NPC moves one step in a randomly chosen direction (up to 8
//! neighbours).  NPCs are processed right-to-left within each row, rows
//! top-to-bottom.  If the chosen cell is occupied the NPC re-rolls; if all
//! reachable neighbours are occupied the NPC skips its move.
//!
//! The host supplies the grid via [`GridConfig`] and seeds the per-turn clock
//! and RNG through [`TurnTimer`] / [`RngState`]; the plugin owns the systems.
//!
//! # Example
//! ```no_run
//! use bevy::prelude::*;
//! use bevy::app::ScheduleRunnerPlugin;
//! use std::time::Duration;
//! use rand::prelude::*;
//! use ascii_npc_sim::{AsciiNpcSimPlugin, GridConfig, TurnTimer, RngState, TurnCount};
//!
//! App::new()
//!     .add_plugins(MinimalPlugins.set(ScheduleRunnerPlugin::run_loop(Duration::from_millis(50))))
//!     .insert_resource(GridConfig { width: 100, height: 100, npc_count: 1000, turn_secs: 1.0 })
//!     .insert_resource(TurnTimer(Timer::from_seconds(1.0, TimerMode::Repeating)))
//!     .insert_resource(RngState(StdRng::from_entropy()))
//!     .insert_resource(TurnCount::default())
//!     .add_plugins(AsciiNpcSimPlugin)
//!     .run();
//! ```

use bevy::prelude::*;
use rand::prelude::*;
use std::collections::HashSet;
use std::io::{self, Write as IoWrite};

/// Bundles the simulation's startup and per-turn systems.
///
/// Add it with `app.add_plugins(AsciiNpcSimPlugin)`. It does **not** add
/// `MinimalPlugins` or a schedule runner, so the host app owns the loop; it
/// also expects the host to insert [`GridConfig`], [`TurnTimer`], [`RngState`],
/// and [`TurnCount`] (all runtime-dependent).
pub struct AsciiNpcSimPlugin;

impl Plugin for AsciiNpcSimPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, spawn_npcs)
            .add_systems(Update, tick_turn);
    }
}

// ─── Config resource ─────────────────────────────────────────────────────────

/// Immutable configuration set at startup (typically from user input).
#[derive(Resource, Clone)]
pub struct GridConfig {
    /// Grid width in cells.
    pub width: usize,
    /// Grid height in cells.
    pub height: usize,
    /// Number of NPCs to scatter across the grid.
    pub npc_count: usize,
    /// Seconds per simulation turn.
    pub turn_secs: f32,
}

// ─── Components ──────────────────────────────────────────────────────────────

/// Grid coordinates of an entity (column `x`, row `y`; origin top-left).
#[derive(Component, Clone, Copy, Debug, PartialEq, Eq)]
pub struct Position {
    /// Column (0-based, from the left).
    pub x: usize,
    /// Row (0-based, from the top).
    pub y: usize,
}

/// Marker: this entity is an NPC.
#[derive(Component)]
pub struct Npc;

// ─── Resources ───────────────────────────────────────────────────────────────

/// Drives the fixed-interval turn clock.
#[derive(Resource)]
pub struct TurnTimer(pub Timer);

/// Shared RNG — stored as a resource so systems can mutate it sequentially.
#[derive(Resource)]
pub struct RngState(pub StdRng);

/// Monotonically increasing turn counter for the status line.
#[derive(Resource, Default)]
pub struct TurnCount(pub u64);

// ─── Pure helpers ────────────────────────────────────────────────────────────

/// Returns every grid cell reachable in one diagonal/cardinal step from
/// `(x, y)` that lies within a `width × height` grid.
pub fn valid_neighbors(x: usize, y: usize, width: usize, height: usize) -> Vec<(usize, usize)> {
    let mut out = Vec::with_capacity(8);
    let (xi, yi) = (x as i64, y as i64);
    let (w, h) = (width as i64, height as i64);
    for dy in -1i64..=1 {
        for dx in -1i64..=1 {
            if dx == 0 && dy == 0 {
                continue;
            }
            let (nx, ny) = (xi + dx, yi + dy);
            if nx >= 0 && nx < w && ny >= 0 && ny < h {
                out.push((nx as usize, ny as usize));
            }
        }
    }
    out
}

/// Builds a complete rendered frame string.
///
/// Every cell is `'.'`; each coordinate in `positions` is drawn as `'@'`.
/// Each row is terminated by a newline.
pub fn render_grid(width: usize, height: usize, positions: &[(usize, usize)]) -> String {
    let mut grid = vec!['.'; width * height];
    for &(x, y) in positions {
        if x < width && y < height {
            grid[y * width + x] = '@';
        }
    }
    let mut out = String::with_capacity((width + 1) * height);
    for row in grid.chunks(width) {
        for &ch in row {
            out.push(ch);
        }
        out.push('\n');
    }
    out
}

// ─── Systems ─────────────────────────────────────────────────────────────────

/// Startup: scatter N NPCs randomly across the grid and show the initial frame.
fn spawn_npcs(mut commands: Commands, config: Res<GridConfig>, mut rng: ResMut<RngState>) {
    let cap = config.width * config.height;
    let count = config.npc_count.min(cap);
    let mut occupied: HashSet<(usize, usize)> = HashSet::with_capacity(count);

    for _ in 0..count {
        let pos = loop {
            let x = rng.0.gen_range(0..config.width);
            let y = rng.0.gen_range(0..config.height);
            if occupied.insert((x, y)) {
                break (x, y);
            }
        };
        commands.spawn((Npc, Position { x: pos.0, y: pos.1 }));
    }

    let positions: Vec<(usize, usize)> = occupied.into_iter().collect();
    let frame = render_grid(config.width, config.height, &positions);
    print!("\x1B[2J\x1B[HTurn 0  NPCs: {count}\n{frame}");
    let _ = io::stdout().flush();
}

/// Update: advance one turn when the timer fires.
///
/// Processing order: right-to-left within each row, rows top-to-bottom.
fn tick_turn(
    time: Res<Time>,
    mut timer: ResMut<TurnTimer>,
    config: Res<GridConfig>,
    mut rng: ResMut<RngState>,
    mut turn: ResMut<TurnCount>,
    mut query: Query<(Entity, &mut Position), With<Npc>>,
) {
    if !timer.0.tick(time.delta()).just_finished() {
        return;
    }
    turn.0 += 1;

    // ── 1. Snapshot all positions (immutable borrow ends before we mutate) ───
    let mut npcs: Vec<(Entity, usize, usize)> = query.iter().map(|(e, p)| (e, p.x, p.y)).collect();

    // top→bottom (y ascending), right→left (x descending)
    npcs.sort_unstable_by(|a, b| a.2.cmp(&b.2).then(b.1.cmp(&a.1)));

    // ── 2. Build a mutable occupancy set ─────────────────────────────────────
    let mut occupied: HashSet<(usize, usize)> = npcs.iter().map(|&(_, x, y)| (x, y)).collect();

    // ── 3. Move each NPC in turn, updating occupancy immediately ─────────────
    let mut updates: Vec<(Entity, usize, usize)> = Vec::with_capacity(npcs.len());

    for &(entity, x, y) in &npcs {
        let neighbors = valid_neighbors(x, y, config.width, config.height);

        // Re-roll semantics: collect free in-bounds cells, pick uniformly.
        // This is equivalent to rolling a random direction and re-rolling if
        // occupied — both yield a uniform distribution over free neighbours.
        let free: Vec<(usize, usize)> = neighbors
            .into_iter()
            .filter(|p| !occupied.contains(p))
            .collect();

        if free.is_empty() {
            // All reachable neighbours occupied — skip this NPC's move.
            updates.push((entity, x, y));
            continue;
        }

        let new_pos = free[rng.0.gen_range(0..free.len())];
        occupied.remove(&(x, y));
        occupied.insert(new_pos);
        updates.push((entity, new_pos.0, new_pos.1));
    }

    // ── 4. Write positions back into ECS components ──────────────────────────
    for &(entity, new_x, new_y) in &updates {
        if let Ok((_, mut pos)) = query.get_mut(entity) {
            pos.x = new_x;
            pos.y = new_y;
        }
    }

    // ── 5. Render ─────────────────────────────────────────────────────────────
    let positions: Vec<(usize, usize)> = updates.iter().map(|&(_, x, y)| (x, y)).collect();
    let frame = render_grid(config.width, config.height, &positions);
    print!(
        "\x1B[2J\x1B[HTurn {}  NPCs: {}\n{frame}",
        turn.0,
        updates.len()
    );
    let _ = io::stdout().flush();
}

// ─── Tests ───────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    // ── valid_neighbors ──────────────────────────────────────────────────────

    #[test]
    fn valid_neighbors_center_returns_8() {
        assert_eq!(valid_neighbors(5, 5, 10, 10).len(), 8);
    }

    #[test]
    fn valid_neighbors_top_left_corner_returns_3() {
        assert_eq!(valid_neighbors(0, 0, 10, 10).len(), 3);
    }

    #[test]
    fn valid_neighbors_left_edge_returns_5() {
        assert_eq!(valid_neighbors(0, 5, 10, 10).len(), 5);
    }

    #[test]
    fn valid_neighbors_1x1_grid_returns_empty() {
        assert!(valid_neighbors(0, 0, 1, 1).is_empty());
    }

    #[test]
    fn valid_neighbors_contains_no_self() {
        let n = valid_neighbors(3, 3, 10, 10);
        assert!(!n.contains(&(3, 3)));
    }

    // ── render_grid ──────────────────────────────────────────────────────────

    #[test]
    fn render_grid_empty_is_all_dots() {
        assert_eq!(render_grid(3, 2, &[]), "...\n...\n");
    }

    #[test]
    fn render_grid_places_npc_at_top_row() {
        assert_eq!(render_grid(3, 2, &[(1, 0)]), ".@.\n...\n");
    }

    #[test]
    fn render_grid_places_npc_at_bottom_right() {
        assert_eq!(render_grid(3, 2, &[(2, 1)]), "...\n..@\n");
    }

    #[test]
    fn render_grid_multiple_npcs() {
        let s = render_grid(3, 3, &[(0, 0), (2, 2)]);
        assert_eq!(s, "@..\n...\n..@\n");
    }

    // ── ECS integration ──────────────────────────────────────────────────────

    #[test]
    fn ecs_spawns_correct_npc_count() {
        let mut app = App::new();
        app.add_plugins(MinimalPlugins)
            .insert_resource(GridConfig {
                width: 10,
                height: 10,
                npc_count: 7,
                turn_secs: 1.0,
            })
            .insert_resource(RngState(StdRng::seed_from_u64(42)))
            .add_systems(Startup, spawn_npcs);
        app.update();
        let count = app.world_mut().query::<&Npc>().iter(app.world()).count();
        assert_eq!(count, 7);
    }

    #[test]
    fn ecs_npc_count_capped_at_grid_size() {
        let mut app = App::new();
        app.add_plugins(MinimalPlugins)
            .insert_resource(GridConfig {
                width: 2,
                height: 2,
                npc_count: 999,
                turn_secs: 1.0,
            })
            .insert_resource(RngState(StdRng::seed_from_u64(0)))
            .add_systems(Startup, spawn_npcs);
        app.update();
        let count = app.world_mut().query::<&Npc>().iter(app.world()).count();
        assert_eq!(count, 4);
    }

    #[test]
    fn ecs_no_two_npcs_share_a_cell() {
        let mut app = App::new();
        app.add_plugins(MinimalPlugins)
            .insert_resource(GridConfig {
                width: 20,
                height: 20,
                npc_count: 50,
                turn_secs: 1.0,
            })
            .insert_resource(RngState(StdRng::seed_from_u64(7)))
            .add_systems(Startup, spawn_npcs);
        app.update();
        let positions: Vec<Position> = app
            .world_mut()
            .query::<&Position>()
            .iter(app.world())
            .copied()
            .collect();
        let unique: HashSet<(usize, usize)> = positions.iter().map(|p| (p.x, p.y)).collect();
        assert_eq!(unique.len(), positions.len());
    }
}
