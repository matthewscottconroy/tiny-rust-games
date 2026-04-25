//! Line-of-sight demo — Bresenham ray casting on a grid.
//!
//! Key ideas:
//! - `bresenham_cells` returns every grid cell a line passes through, using
//!   the classic integer error-accumulation algorithm.
//! - `can_see` walks those cells and returns `false` the moment a wall is hit,
//!   giving per-cell visibility from the player's position.
//! - Each frame all tiles are recoloured based on whether the player can see
//!   them: fully-lit (visible), dim (previously seen), or black (unknown).
//! - The player moves on the same grid; walls block sight but not the HUD.
//!
//! **Controls:** WASD / Arrow keys — move player.

use bevy::prelude::*;
use bevy::window::WindowResolution;
use std::collections::HashSet;

const COLS: usize = 30;
const ROWS: usize = 20;
const TILE_PX: f32 = 26.0;
const SIGHT_RADIUS: i32 = 8;

// ─── Pure helpers ────────────────────────────────────────────────────────────

/// Returns all grid cells the line from `from` to `to` passes through,
/// including both endpoints, using Bresenham's line algorithm.
pub fn bresenham_cells(from: IVec2, to: IVec2) -> Vec<IVec2> {
    let mut cells = Vec::new();
    let (mut x, mut y) = (from.x, from.y);
    let dx = (to.x - x).abs();
    let dy = -(to.y - y).abs();
    let sx: i32 = if x < to.x { 1 } else { -1 };
    let sy: i32 = if y < to.y { 1 } else { -1 };
    let mut err = dx + dy;
    loop {
        cells.push(IVec2::new(x, y));
        if x == to.x && y == to.y {
            break;
        }
        let e2 = 2 * err;
        if e2 >= dy {
            err += dy;
            x += sx;
        }
        if e2 <= dx {
            err += dx;
            y += sy;
        }
    }
    cells
}

/// Returns `true` if `from` has an unobstructed line to `to` on `grid`.
///
/// A cell is an obstacle when `grid[y][x]` is `false`.  The `from` cell is
/// never treated as blocking (allows standing in corridors).
pub fn can_see(grid: &[Vec<bool>], from: IVec2, to: IVec2) -> bool {
    let rows = grid.len() as i32;
    let cols = grid[0].len() as i32;
    for cell in bresenham_cells(from, to) {
        if cell == from {
            continue;
        }
        if cell.x < 0 || cell.y < 0 || cell.x >= cols || cell.y >= rows {
            return false;
        }
        if !grid[cell.y as usize][cell.x as usize] {
            return false;
        }
    }
    true
}

// ─── Map ─────────────────────────────────────────────────────────────────────

fn make_grid() -> Vec<Vec<bool>> {
    let rows = [
        "##############################",
        "#....#.........#............#",
        "#....#....###..#....####....#",
        "#.........#.........#.......#",
        "###.#####.#.#####...#.######",
        "#.........#.....#...#.......#",
        "#....###..#####.#...#.###...#",
        "#....#..........#...#...#...#",
        "#....#..........#########...#",
        "#....#..........#...........#",
        "#.####.##########...####....#",
        "#......#............#...#...#",
        "#......#....###.....#...#...#",
        "#......#....#.......#...#...#",
        "#..#####....#...#########...#",
        "#..#........#...#...........#",
        "#..#........#...#...........#",
        "#..#........#...#...........#",
        "#...........................#",
        "##############################",
    ];
    rows.iter()
        .map(|row| row.chars().map(|c| c == '.').collect())
        .collect()
}

// ─── Resources & components ──────────────────────────────────────────────────

#[derive(Resource)]
struct WalkGrid(Vec<Vec<bool>>);

#[derive(Resource)]
struct PlayerCell(IVec2);

/// Tracks cells the player has ever been able to see.
#[derive(Resource, Default)]
struct Revealed(HashSet<IVec2>);

/// Attached to each tile sprite with its grid position, so the update system
/// can look up visibility without a separate position query.
#[derive(Component)]
struct TileCell(IVec2);

#[derive(Component)]
struct PlayerMarker;

// ─── Bevy app ────────────────────────────────────────────────────────────────

fn main() {
    App::new()
        .add_plugins(DefaultPlugins.set(WindowPlugin {
            primary_window: Some(Window {
                title: "Line of Sight — WASD to move".to_string(),
                resolution: (780u32, 548u32).into(),
                ..default()
            }),
            ..default()
        }))
        .init_resource::<Revealed>()
        .add_systems(Startup, setup)
        .add_systems(Update, (handle_input, update_visibility, sync_player))
        .run();
}

fn cell_to_world(cell: IVec2) -> Vec3 {
    let ox = -(COLS as f32 * TILE_PX) / 2.0 + TILE_PX / 2.0;
    let oy = (ROWS as f32 * TILE_PX) / 2.0 - TILE_PX / 2.0;
    Vec3::new(ox + cell.x as f32 * TILE_PX, oy - cell.y as f32 * TILE_PX, 0.0)
}

fn setup(mut commands: Commands, mut revealed: ResMut<Revealed>) {
    commands.spawn(Camera2d);
    let grid = make_grid();

    for (r, row) in grid.iter().enumerate() {
        for (c, _) in row.iter().enumerate() {
            let cell = IVec2::new(c as i32, r as i32);
            commands.spawn((
                Sprite { color: Color::BLACK, custom_size: Some(Vec2::splat(TILE_PX - 1.0)), ..default() },
                Transform::from_translation(cell_to_world(cell)),
                TileCell(cell),
            ));
        }
    }

    let start = IVec2::new(1, 1);
    revealed.0.insert(start);
    commands.spawn((
        Sprite { color: Color::srgb(0.3, 0.8, 1.0), custom_size: Some(Vec2::splat(TILE_PX * 0.6)), ..default() },
        Transform::from_translation(cell_to_world(start).with_z(1.0)),
        PlayerMarker,
    ));

    commands.insert_resource(WalkGrid(grid));
    commands.insert_resource(PlayerCell(start));

    commands.spawn((
        Text::new("WASD / Arrow keys — move"),
        TextFont { font_size: 16.0, ..default() },
        TextColor(Color::WHITE),
        Node {
            position_type: PositionType::Absolute,
            bottom: Val::Px(4.0),
            left: Val::Px(8.0),
            ..default()
        },
    ));
}

fn handle_input(
    input: Res<ButtonInput<KeyCode>>,
    grid: Res<WalkGrid>,
    mut player: ResMut<PlayerCell>,
) {
    let dirs = [
        (KeyCode::KeyW, IVec2::new(0, -1)),
        (KeyCode::KeyS, IVec2::new(0, 1)),
        (KeyCode::KeyA, IVec2::new(-1, 0)),
        (KeyCode::KeyD, IVec2::new(1, 0)),
        (KeyCode::ArrowUp, IVec2::new(0, -1)),
        (KeyCode::ArrowDown, IVec2::new(0, 1)),
        (KeyCode::ArrowLeft, IVec2::new(-1, 0)),
        (KeyCode::ArrowRight, IVec2::new(1, 0)),
    ];
    let rows = grid.0.len() as i32;
    let cols = grid.0[0].len() as i32;
    for (key, delta) in dirs {
        if input.just_pressed(key) {
            let next = player.0 + delta;
            if next.x >= 0 && next.y >= 0 && next.x < cols && next.y < rows
                && grid.0[next.y as usize][next.x as usize]
            {
                player.0 = next;
                break;
            }
        }
    }
}

/// Recolours every tile: bright if currently visible, dim if remembered, black if unknown.
fn update_visibility(
    grid: Res<WalkGrid>,
    player: Res<PlayerCell>,
    mut revealed: ResMut<Revealed>,
    mut tiles: Query<(&TileCell, &mut Sprite)>,
) {
    let rows = grid.0.len() as i32;
    let cols = grid.0[0].len() as i32;

    // Collect visible cells this frame via LOS checks.
    let mut visible: HashSet<IVec2> = HashSet::new();
    for dy in -SIGHT_RADIUS..=SIGHT_RADIUS {
        for dx in -SIGHT_RADIUS..=SIGHT_RADIUS {
            if dx * dx + dy * dy > SIGHT_RADIUS * SIGHT_RADIUS {
                continue;
            }
            let target = player.0 + IVec2::new(dx, dy);
            if target.x < 0 || target.y < 0 || target.x >= cols || target.y >= rows {
                continue;
            }
            if can_see(&grid.0, player.0, target) {
                visible.insert(target);
                revealed.0.insert(target);
            }
        }
    }

    for (TileCell(cell), mut sprite) in &mut tiles {
        let walkable = {
            let (cx, cy) = (cell.x as usize, cell.y as usize);
            cy < grid.0.len() && cx < grid.0[0].len() && grid.0[cy][cx]
        };
        sprite.color = if visible.contains(cell) {
            if walkable { Color::srgb(0.55, 0.5, 0.42) } else { Color::srgb(0.3, 0.3, 0.38) }
        } else if revealed.0.contains(cell) {
            if walkable { Color::srgb(0.22, 0.2, 0.18) } else { Color::srgb(0.12, 0.12, 0.16) }
        } else {
            Color::BLACK
        };
    }
}

fn sync_player(player: Res<PlayerCell>, mut q: Query<&mut Transform, With<PlayerMarker>>) {
    if let Ok(mut t) = q.single_mut() {
        t.translation = cell_to_world(player.0).with_z(1.0);
    }
}

// ─── Tests ───────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bresenham_horizontal_line() {
        let cells = bresenham_cells(IVec2::new(0, 0), IVec2::new(4, 0));
        assert_eq!(cells.len(), 5);
        for (i, c) in cells.iter().enumerate() {
            assert_eq!(*c, IVec2::new(i as i32, 0));
        }
    }

    #[test]
    fn bresenham_single_cell() {
        let cells = bresenham_cells(IVec2::new(3, 3), IVec2::new(3, 3));
        assert_eq!(cells, vec![IVec2::new(3, 3)]);
    }

    #[test]
    fn bresenham_diagonal_has_correct_endpoints() {
        let cells = bresenham_cells(IVec2::new(0, 0), IVec2::new(3, 3));
        assert_eq!(*cells.first().unwrap(), IVec2::new(0, 0));
        assert_eq!(*cells.last().unwrap(), IVec2::new(3, 3));
    }

    #[test]
    fn can_see_through_open_space() {
        let grid = vec![vec![true; 10]; 10];
        assert!(can_see(&grid, IVec2::new(0, 0), IVec2::new(9, 9)));
    }

    #[test]
    fn can_see_blocked_by_wall() {
        let mut grid = vec![vec![true; 10]; 10];
        // Wall column at x=3.
        for r in &mut grid {
            r[3] = false;
        }
        assert!(!can_see(&grid, IVec2::new(0, 5), IVec2::new(9, 5)));
    }

    #[test]
    fn can_see_same_cell_always_true() {
        let grid = vec![vec![true; 5]; 5];
        assert!(can_see(&grid, IVec2::new(2, 2), IVec2::new(2, 2)));
    }
}
