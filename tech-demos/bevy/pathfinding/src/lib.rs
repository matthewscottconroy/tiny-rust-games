//! Pathfinding — a reusable Bevy plugin.
//!
//! This crate is a *building block*: drop [`PathfindingPlugin`] into any Bevy
//! app with `app.add_plugins(PathfindingPlugin)` and it spawns a grid level, a
//! player you move with arrow keys / WASD, and a red seeker that chases the
//! player using A\* pathfinding.
//!
//! Key ideas:
//! - The grid is a `Vec<Vec<bool>>` where `true` means "walkable".
//! - [`astar`] is a pure function: it takes the grid, start, and goal cells and
//!   returns a `Vec<IVec2>` path (or `None` if unreachable). It is the star of
//!   this demo and is thoroughly unit-tested.
//! - The [`Seeker`] component stores the current path and a move timer so it
//!   steps cell-by-cell at a readable pace.
//! - The player moves on the same grid with arrow keys; the seeker recomputes
//!   its path each time the player moves.
//!
//! Tune the tile size and seeker speed through the [`PathfindingConfig`]
//! resource without editing the plugin's internals.
//!
//! **Controls:** Arrow keys / WASD to move the player (cyan).
//!
//! # Example
//! ```no_run
//! use bevy::prelude::*;
//! use pathfinding::PathfindingPlugin;
//!
//! App::new()
//!     .add_plugins(DefaultPlugins)
//!     .add_plugins(PathfindingPlugin)
//!     .run();
//! ```
//!
//! Counterpart: tech-demos/godot/pathfinding-astar — the same concept in Godot.

use bevy::prelude::*;
use std::cmp::Reverse;
use std::collections::{BinaryHeap, HashMap};

/// Bundles every system, resource, and the config for the pathfinding feature.
///
/// Add it with `app.add_plugins(PathfindingPlugin)`. It does **not** add
/// `DefaultPlugins`, so the host app stays in control of window and rendering
/// setup.
pub struct PathfindingPlugin;

impl Plugin for PathfindingPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<PathfindingConfig>()
            .add_systems(Startup, setup)
            .add_systems(Update, (handle_input, move_seeker, sync_visuals));
    }
}

// --- Configuration ---

/// Tunable parameters for the feature. Override before adding the plugin, e.g.
/// `app.insert_resource(PathfindingConfig { tile: 32.0, ..default() })`.
#[derive(Resource, Clone, Copy, Debug)]
pub struct PathfindingConfig {
    /// Tile size in pixels.
    pub tile: f32,
    /// Seconds per seeker step.
    pub step_interval: f32,
}

impl Default for PathfindingConfig {
    fn default() -> Self {
        Self {
            tile: 40.0,
            step_interval: 0.22,
        }
    }
}

// --- Resources ---

/// The walkability grid shared across systems.
#[derive(Resource)]
pub struct PathGrid(pub Vec<Vec<bool>>);

/// Player grid position.
#[derive(Resource)]
pub struct PlayerCell(pub IVec2);

// --- Components ---

/// Marker for the player visual entity.
#[derive(Component)]
pub struct PlayerMarker;

/// Chasing entity with its current A\* path.
#[derive(Component)]
pub struct Seeker {
    /// Current grid cell of the seeker.
    pub cell: IVec2,
    /// Remaining cells to walk (not including the current cell).
    pub path: Vec<IVec2>,
    /// Accumulated time since the last step, in seconds.
    pub timer: f32,
}

/// The hard-coded level grid (`false` = wall, `true` = floor).
pub fn make_grid() -> Vec<Vec<bool>> {
    // 14 columns × 12 rows, '#' = wall, '.' = floor.
    let rows = [
        "##############",
        "#....#.......#",
        "#....#..###..#",
        "#.......#....#",
        "###.####.....#",
        "#....#...###.#",
        "#....#...#...#",
        "#....#...#.###",
        "#........#...#",
        "#....#####...#",
        "#............#",
        "##############",
    ];
    rows.iter()
        .map(|row| row.chars().map(|c| c == '.').collect())
        .collect()
}

/// Returns the world-space centre of a grid cell.
pub fn cell_to_world(cell: IVec2, cols: usize, rows: usize, tile: f32) -> Vec3 {
    let offset_x = -(cols as f32 * tile) / 2.0 + tile / 2.0;
    let offset_y = (rows as f32 * tile) / 2.0 - tile / 2.0;
    Vec3::new(
        offset_x + cell.x as f32 * tile,
        offset_y - cell.y as f32 * tile,
        0.0,
    )
}

/// Spawns tiles, the player sprite, and the seeker.
fn setup(mut commands: Commands, config: Res<PathfindingConfig>) {
    commands.spawn(Camera2d);

    let tile = config.tile;
    let grid = make_grid();
    let rows = grid.len();
    let cols = grid[0].len();

    // Draw tiles.
    for (r, row) in grid.iter().enumerate() {
        for (c, &walkable) in row.iter().enumerate() {
            let color = if walkable {
                Color::srgb(0.2, 0.2, 0.28)
            } else {
                Color::srgb(0.5, 0.5, 0.6)
            };
            let pos = cell_to_world(IVec2::new(c as i32, r as i32), cols, rows, tile);
            commands.spawn((
                Sprite {
                    color,
                    custom_size: Some(Vec2::splat(tile - 2.0)),
                    ..default()
                },
                Transform::from_translation(pos),
            ));
        }
    }

    let player_start = IVec2::new(1, 1);
    let seeker_start = IVec2::new(12, 10);

    // Player.
    let player_world = cell_to_world(player_start, cols, rows, tile);
    commands.spawn((
        Sprite {
            color: Color::srgb(0.2, 0.9, 0.9),
            custom_size: Some(Vec2::splat(tile * 0.6)),
            ..default()
        },
        Transform::from_translation(player_world.with_z(1.0)),
        PlayerMarker,
    ));

    // Seeker.
    let seeker_world = cell_to_world(seeker_start, cols, rows, tile);
    commands.spawn((
        Sprite {
            color: Color::srgb(0.9, 0.2, 0.2),
            custom_size: Some(Vec2::splat(tile * 0.55)),
            ..default()
        },
        Transform::from_translation(seeker_world.with_z(1.0)),
        Seeker {
            cell: seeker_start,
            path: Vec::new(),
            timer: 0.0,
        },
    ));

    commands.insert_resource(PlayerCell(player_start));
    commands.insert_resource(PathGrid(grid));

    commands.spawn((
        Text::new("Arrow keys / WASD - move   Red chases you"),
        TextFont {
            font_size: 18.0,
            ..default()
        },
        TextColor(Color::WHITE),
        Node {
            position_type: PositionType::Absolute,
            bottom: Val::Px(8.0),
            left: Val::Px(8.0),
            ..default()
        },
    ));
}

/// Moves the player and triggers a seeker path recompute on change.
fn handle_input(
    input: Res<ButtonInput<KeyCode>>,
    grid: Res<PathGrid>,
    mut player_cell: ResMut<PlayerCell>,
    mut seeker_query: Query<&mut Seeker>,
) {
    let dirs = [
        (KeyCode::ArrowUp, IVec2::new(0, -1)),
        (KeyCode::ArrowDown, IVec2::new(0, 1)),
        (KeyCode::ArrowLeft, IVec2::new(-1, 0)),
        (KeyCode::ArrowRight, IVec2::new(1, 0)),
        (KeyCode::KeyW, IVec2::new(0, -1)),
        (KeyCode::KeyS, IVec2::new(0, 1)),
        (KeyCode::KeyA, IVec2::new(-1, 0)),
        (KeyCode::KeyD, IVec2::new(1, 0)),
    ];
    let rows = grid.0.len() as i32;
    let cols = grid.0[0].len() as i32;
    let mut moved = false;
    for (key, delta) in dirs {
        if input.just_pressed(key) {
            let next = player_cell.0 + delta;
            if next.x >= 0
                && next.y >= 0
                && next.x < cols
                && next.y < rows
                && grid.0[next.y as usize][next.x as usize]
            {
                player_cell.0 = next;
                moved = true;
                break;
            }
        }
    }
    if moved && let Ok(mut seeker) = seeker_query.single_mut() {
        seeker.path = astar(&grid.0, seeker.cell, player_cell.0).unwrap_or_default();
    }
}

/// Steps the seeker along its cached path at a fixed interval.
fn move_seeker(
    time: Res<Time>,
    grid: Res<PathGrid>,
    player_cell: Res<PlayerCell>,
    config: Res<PathfindingConfig>,
    mut query: Query<&mut Seeker>,
) {
    let Ok(mut seeker) = query.single_mut() else {
        return;
    };
    seeker.timer += time.delta_secs();
    if seeker.timer < config.step_interval {
        return;
    }
    seeker.timer = 0.0;
    if seeker.path.is_empty() {
        // Recompute if we have no path.
        seeker.path = astar(&grid.0, seeker.cell, player_cell.0).unwrap_or_default();
    }
    if let Some(next) = seeker.path.first().copied() {
        seeker.cell = next;
        seeker.path.remove(0);
    }
}

/// Syncs sprite world positions from logical cell positions.
fn sync_visuals(
    grid: Res<PathGrid>,
    player_cell: Res<PlayerCell>,
    config: Res<PathfindingConfig>,
    mut player_query: Query<&mut Transform, With<PlayerMarker>>,
    mut seeker_query: Query<(&mut Transform, &Seeker), Without<PlayerMarker>>,
) {
    let rows = grid.0.len();
    let cols = grid.0[0].len();
    let tile = config.tile;
    if let Ok(mut t) = player_query.single_mut() {
        t.translation = cell_to_world(player_cell.0, cols, rows, tile).with_z(1.0);
    }
    if let Ok((mut t, seeker)) = seeker_query.single_mut() {
        t.translation = cell_to_world(seeker.cell, cols, rows, tile).with_z(1.0);
    }
}

/// A\* shortest-path search on a 2D walkability grid.
///
/// Returns the path from `start` to `goal` (not including `start`, including
/// `goal`), or `None` if no path exists.
pub fn astar(grid: &[Vec<bool>], start: IVec2, goal: IVec2) -> Option<Vec<IVec2>> {
    let rows = grid.len() as i32;
    let cols = grid[0].len() as i32;

    // Min-heap: (f, g, cell_x, cell_y)
    let mut open: BinaryHeap<Reverse<(i32, i32, i32, i32)>> = BinaryHeap::new();
    let mut g_score: HashMap<(i32, i32), i32> = HashMap::new();
    let mut came_from: HashMap<(i32, i32), (i32, i32)> = HashMap::new();

    let h = |p: IVec2| (p.x - goal.x).abs() + (p.y - goal.y).abs();
    g_score.insert((start.x, start.y), 0);
    open.push(Reverse((h(start), 0, start.x, start.y)));

    let neighbors = [
        IVec2::new(1, 0),
        IVec2::new(-1, 0),
        IVec2::new(0, 1),
        IVec2::new(0, -1),
    ];

    while let Some(Reverse((_, g, cx, cy))) = open.pop() {
        let current = IVec2::new(cx, cy);
        if current == goal {
            // Reconstruct path.
            let mut path = Vec::new();
            let mut cur = (goal.x, goal.y);
            while cur != (start.x, start.y) {
                path.push(IVec2::new(cur.0, cur.1));
                cur = *came_from.get(&cur)?;
            }
            path.reverse();
            return Some(path);
        }
        let current_g = *g_score.get(&(cx, cy)).unwrap_or(&i32::MAX);
        if g > current_g {
            continue; // Stale entry.
        }
        for delta in neighbors {
            let nb = current + delta;
            if nb.x < 0 || nb.y < 0 || nb.x >= cols || nb.y >= rows {
                continue;
            }
            if !grid[nb.y as usize][nb.x as usize] {
                continue;
            }
            let tentative_g = g + 1;
            let nb_key = (nb.x, nb.y);
            if tentative_g < *g_score.get(&nb_key).unwrap_or(&i32::MAX) {
                g_score.insert(nb_key, tentative_g);
                came_from.insert(nb_key, (cx, cy));
                open.push(Reverse((tentative_g + h(nb), tentative_g, nb.x, nb.y)));
            }
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    fn open_grid(rows: usize, cols: usize) -> Vec<Vec<bool>> {
        vec![vec![true; cols]; rows]
    }

    fn walled_grid() -> Vec<Vec<bool>> {
        // Wall column 2 to block path.
        let mut g = open_grid(5, 5);
        for r in 0..5 {
            g[r][2] = false;
        }
        g
    }

    // --- astar pure-function tests ---

    #[test]
    fn straight_path_found() {
        let grid = open_grid(5, 5);
        let path = astar(&grid, IVec2::new(0, 0), IVec2::new(4, 0)).unwrap();
        assert!(!path.is_empty());
        assert_eq!(*path.last().unwrap(), IVec2::new(4, 0));
    }

    #[test]
    fn path_avoids_wall() {
        let grid = walled_grid();
        // No open columns between 0-1 and 3-4 → unreachable.
        let result = astar(&grid, IVec2::new(0, 0), IVec2::new(4, 0));
        assert!(result.is_none(), "should be unreachable through solid wall");
    }

    #[test]
    fn path_through_gap_is_found() {
        let mut grid = walled_grid();
        grid[4][2] = true; // open a gap at the bottom
        let path = astar(&grid, IVec2::new(0, 0), IVec2::new(4, 0));
        assert!(path.is_some(), "path through gap should exist");
    }

    #[test]
    fn same_start_and_goal_returns_empty_path() {
        let grid = open_grid(5, 5);
        let path = astar(&grid, IVec2::new(2, 2), IVec2::new(2, 2)).unwrap();
        assert!(path.is_empty(), "zero-step path should be empty");
    }

    #[test]
    fn path_does_not_include_start_includes_goal() {
        let grid = open_grid(3, 3);
        let start = IVec2::new(0, 0);
        let goal = IVec2::new(2, 2);
        let path = astar(&grid, start, goal).unwrap();
        assert_ne!(path[0], start, "path should not include start");
        assert_eq!(*path.last().unwrap(), goal);
    }

    #[test]
    fn manhattan_path_length_matches_distance() {
        let grid = open_grid(1, 6);
        let path = astar(&grid, IVec2::new(0, 0), IVec2::new(5, 0)).unwrap();
        assert_eq!(path.len(), 5);
    }

    #[test]
    fn path_steps_are_adjacent() {
        // Each consecutive cell in a returned path must be a 4-neighbour move.
        let grid = open_grid(6, 6);
        let path = astar(&grid, IVec2::new(0, 0), IVec2::new(5, 5)).unwrap();
        let mut prev = IVec2::new(0, 0);
        for &cell in &path {
            let d = (cell - prev).abs();
            assert_eq!(d.x + d.y, 1, "steps must be single-cell moves");
            prev = cell;
        }
    }

    #[test]
    fn make_grid_is_rectangular_and_walled() {
        let g = make_grid();
        assert_eq!(g.len(), 12);
        let cols = g[0].len();
        assert!(g.iter().all(|row| row.len() == cols));
        // Border is solid wall.
        assert!(g[0].iter().all(|&w| !w));
        assert!(g[11].iter().all(|&w| !w));
    }

    // --- config ---

    #[test]
    fn config_default_matches_documented_values() {
        let c = PathfindingConfig::default();
        assert_eq!(c.tile, 40.0);
        assert!(c.step_interval > 0.0);
    }

    // --- ECS setup test ---

    #[test]
    fn setup_spawns_player_and_seeker() {
        let mut app = App::new();
        app.add_plugins(MinimalPlugins)
            .init_resource::<PathfindingConfig>()
            .add_systems(Startup, setup);
        app.update();

        let mut pq = app.world_mut().query::<&PlayerMarker>();
        assert_eq!(pq.iter(app.world()).count(), 1);
        let mut sq = app.world_mut().query::<&Seeker>();
        assert_eq!(sq.iter(app.world()).count(), 1);
    }
}
