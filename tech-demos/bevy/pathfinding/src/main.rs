//! Pathfinding demo — A\* on a grid with a seeker chasing the player.
//!
//! Key ideas:
//! - The grid is a `Vec<Vec<bool>>` where `true` means "walkable".
//! - `astar` is a pure function: it takes the grid, start, and goal cells and
//!   returns a `Vec<IVec2>` path (or `None` if unreachable).
//! - The `Seeker` component stores the current path and a move timer so it
//!   steps cell-by-cell at a readable pace.
//! - The player moves on the same grid with arrow keys; the seeker recomputes
//!   its path each time the player moves.
//!
//! **Controls:** Arrow keys / WASD to move the player (cyan).

use bevy::prelude::*;
use std::cmp::Reverse;
use std::collections::{BinaryHeap, HashMap};

/// Tile size in pixels.
const TILE: f32 = 40.0;
/// Seconds per seeker step.
const STEP_INTERVAL: f32 = 0.22;

/// The walkability grid shared across systems.
#[derive(Resource)]
struct PathGrid(Vec<Vec<bool>>);

/// Player grid position.
#[derive(Resource)]
struct PlayerCell(IVec2);

/// Marker for the player visual entity.
#[derive(Component)]
struct PlayerMarker;

/// Chasing entity with its current A\* path.
#[derive(Component)]
struct Seeker {
    cell: IVec2,
    path: Vec<IVec2>,
    timer: f32,
}

fn main() {
    App::new()
        .add_plugins(DefaultPlugins.set(WindowPlugin {
            primary_window: Some(Window {
                title: "A* Pathfinding — arrow keys to move".to_string(),
                resolution: (800.0, 560.0).into(),
                ..default()
            }),
            ..default()
        }))
        .add_systems(Startup, setup)
        .add_systems(Update, (handle_input, move_seeker, sync_visuals))
        .run();
}

/// The hard-coded level grid (`false` = wall, `true` = floor).
fn make_grid() -> Vec<Vec<bool>> {
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
fn cell_to_world(cell: IVec2, cols: usize, rows: usize) -> Vec3 {
    let offset_x = -(cols as f32 * TILE) / 2.0 + TILE / 2.0;
    let offset_y = (rows as f32 * TILE) / 2.0 - TILE / 2.0;
    Vec3::new(
        offset_x + cell.x as f32 * TILE,
        offset_y - cell.y as f32 * TILE,
        0.0,
    )
}

/// Spawns tiles, the player sprite, and the seeker.
fn setup(mut commands: Commands) {
    commands.spawn(Camera2d);

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
            let pos = cell_to_world(IVec2::new(c as i32, r as i32), cols, rows);
            commands.spawn((
                Sprite { color, custom_size: Some(Vec2::splat(TILE - 2.0)), ..default() },
                Transform::from_translation(pos),
            ));
        }
    }

    let player_start = IVec2::new(1, 1);
    let seeker_start = IVec2::new(12, 10);

    // Player.
    let player_world = cell_to_world(player_start, cols, rows);
    commands.spawn((
        Sprite { color: Color::srgb(0.2, 0.9, 0.9), custom_size: Some(Vec2::splat(TILE * 0.6)), ..default() },
        Transform::from_translation(player_world.with_z(1.0)),
        PlayerMarker,
    ));

    // Seeker.
    let seeker_world = cell_to_world(seeker_start, cols, rows);
    commands.spawn((
        Sprite { color: Color::srgb(0.9, 0.2, 0.2), custom_size: Some(Vec2::splat(TILE * 0.55)), ..default() },
        Transform::from_translation(seeker_world.with_z(1.0)),
        Seeker { cell: seeker_start, path: Vec::new(), timer: 0.0 },
    ));

    commands.insert_resource(PlayerCell(player_start));
    commands.insert_resource(PathGrid(grid));

    commands.spawn((
        Text::new("Arrow keys / WASD — move   Red chases you"),
        TextFont { font_size: 18.0, ..default() },
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
        (KeyCode::ArrowUp,    IVec2::new(0, -1)),
        (KeyCode::ArrowDown,  IVec2::new(0,  1)),
        (KeyCode::ArrowLeft,  IVec2::new(-1, 0)),
        (KeyCode::ArrowRight, IVec2::new( 1, 0)),
        (KeyCode::KeyW,       IVec2::new(0, -1)),
        (KeyCode::KeyS,       IVec2::new(0,  1)),
        (KeyCode::KeyA,       IVec2::new(-1, 0)),
        (KeyCode::KeyD,       IVec2::new( 1, 0)),
    ];
    let rows = grid.0.len() as i32;
    let cols = grid.0[0].len() as i32;
    let mut moved = false;
    for (key, delta) in dirs {
        if input.just_pressed(key) {
            let next = player_cell.0 + delta;
            if next.x >= 0 && next.y >= 0 && next.x < cols && next.y < rows
                && grid.0[next.y as usize][next.x as usize]
            {
                player_cell.0 = next;
                moved = true;
                break;
            }
        }
    }
    if moved {
        if let Ok(mut seeker) = seeker_query.get_single_mut() {
            seeker.path = astar(&grid.0, seeker.cell, player_cell.0)
                .unwrap_or_default();
        }
    }
}

/// Steps the seeker along its cached path at a fixed interval.
fn move_seeker(time: Res<Time>, grid: Res<PathGrid>, player_cell: Res<PlayerCell>, mut query: Query<&mut Seeker>) {
    let Ok(mut seeker) = query.get_single_mut() else { return };
    seeker.timer += time.delta_secs();
    if seeker.timer < STEP_INTERVAL {
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
    mut player_query: Query<&mut Transform, With<PlayerMarker>>,
    mut seeker_query: Query<(&mut Transform, &Seeker), Without<PlayerMarker>>,
) {
    let rows = grid.0.len();
    let cols = grid.0[0].len();
    if let Ok(mut t) = player_query.get_single_mut() {
        t.translation = cell_to_world(player_cell.0, cols, rows).with_z(1.0);
    }
    if let Ok((mut t, seeker)) = seeker_query.get_single_mut() {
        t.translation = cell_to_world(seeker.cell, cols, rows).with_z(1.0);
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

    let neighbors = [IVec2::new(1, 0), IVec2::new(-1, 0), IVec2::new(0, 1), IVec2::new(0, -1)];

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
    fn step_interval_is_positive() {
        assert!(STEP_INTERVAL > 0.0);
    }
}
