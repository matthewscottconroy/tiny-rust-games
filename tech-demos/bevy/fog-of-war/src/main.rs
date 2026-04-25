//! Fog-of-war demo — three-state tile visibility on a grid map.
//!
//! Key ideas:
//! - Each tile has one of three states: **Hidden** (never seen, black),
//!   **Remembered** (seen before but outside current view, dimmed), or
//!   **Visible** (inside the player's current sight radius, fully lit).
//! - `cells_in_radius` enumerates every cell within a circle of a given
//!   radius using integer arithmetic so there is no floating-point drift.
//! - `is_within_radius` is the fast per-cell test that drives the state logic.
//! - The player moves on a grid; sight updates every step.
//!
//! **Controls:** WASD / Arrow keys — move player.

use bevy::prelude::*;
use bevy::window::WindowResolution;
use std::collections::HashSet;

const COLS: usize = 32;
const ROWS: usize = 22;
const TILE_PX: f32 = 24.0;
const SIGHT_RADIUS: i32 = 5;

// ─── Pure helpers ────────────────────────────────────────────────────────────

/// Returns all grid cells whose Chebyshev circle (Euclidean) distance from
/// `center` is at most `radius`.
pub fn cells_in_radius(center: IVec2, radius: i32) -> Vec<IVec2> {
    let mut out = Vec::new();
    for dy in -radius..=radius {
        for dx in -radius..=radius {
            if dx * dx + dy * dy <= radius * radius {
                out.push(center + IVec2::new(dx, dy));
            }
        }
    }
    out
}

/// Returns `true` when `pos` is within `radius` cells of `center`
/// (Euclidean distance, integer arithmetic).
pub fn is_within_radius(pos: IVec2, center: IVec2, radius: i32) -> bool {
    let d = pos - center;
    d.x * d.x + d.y * d.y <= radius * radius
}

// ─── Map ─────────────────────────────────────────────────────────────────────

fn make_grid() -> Vec<Vec<bool>> {
    let rows = [
        "################################",
        "#......#.........#............#",
        "#......#....###..#....####....#",
        "#..........#.........#........#",
        "###.######.#.####....#.#######",
        "#..........#.....#...#........#",
        "#.....###..####..#...#.###....#",
        "#.....#..........#...#...#....#",
        "#.....#..........##########...#",
        "#.....#..........#............#",
        "#.#####.##########....####....#",
        "#......#..............#...#...#",
        "#......#......###.....#...#...#",
        "#......#......#.......#...#...#",
        "#...########..#...#########...#",
        "#...#.........#...#...........#",
        "#...#.........#...#...........#",
        "#...#.........#...#...........#",
        "#.............................#",
        "#.............................#",
        "#.............................#",
        "################################",
    ];
    rows.iter()
        .map(|row| row.chars().map(|c| c == '.').collect())
        .collect()
}

// ─── Tile state ──────────────────────────────────────────────────────────────

#[derive(Clone, Copy, PartialEq)]
enum Visibility {
    Hidden,
    Remembered,
    Visible,
}

// ─── Resources & components ──────────────────────────────────────────────────

#[derive(Resource)]
struct WalkGrid(Vec<Vec<bool>>);

#[derive(Resource)]
struct PlayerCell(IVec2);

/// Set of cells the player has ever had line-of-sight to.
#[derive(Resource, Default)]
struct Revealed(HashSet<IVec2>);

#[derive(Component)]
struct TileCell(IVec2);

#[derive(Component)]
struct PlayerMarker;

// ─── Bevy app ────────────────────────────────────────────────────────────────

fn main() {
    App::new()
        .add_plugins(DefaultPlugins.set(WindowPlugin {
            primary_window: Some(Window {
                title: "Fog of War — WASD to move".to_string(),
                resolution: (768u32, 556u32).into(),
                ..default()
            }),
            ..default()
        }))
        .init_resource::<Revealed>()
        .add_systems(Startup, setup)
        .add_systems(Update, (handle_input, update_fog, sync_player))
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

    for r in 0..ROWS {
        for c in 0..COLS {
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
        Sprite { color: Color::srgb(0.3, 0.9, 1.0), custom_size: Some(Vec2::splat(TILE_PX * 0.6)), ..default() },
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
        (KeyCode::KeyW, IVec2::NEG_Y),
        (KeyCode::KeyS, IVec2::Y),
        (KeyCode::KeyA, IVec2::NEG_X),
        (KeyCode::KeyD, IVec2::X),
        (KeyCode::ArrowUp, IVec2::NEG_Y),
        (KeyCode::ArrowDown, IVec2::Y),
        (KeyCode::ArrowLeft, IVec2::NEG_X),
        (KeyCode::ArrowRight, IVec2::X),
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

/// Classifies every tile and sets its sprite colour accordingly.
fn update_fog(
    grid: Res<WalkGrid>,
    player: Res<PlayerCell>,
    mut revealed: ResMut<Revealed>,
    mut tiles: Query<(&TileCell, &mut Sprite)>,
) {
    let rows = grid.0.len() as i32;
    let cols = grid.0[0].len() as i32;

    // Reveal cells in current radius.
    for cell in cells_in_radius(player.0, SIGHT_RADIUS) {
        if cell.x >= 0 && cell.y >= 0 && cell.x < cols && cell.y < rows {
            revealed.0.insert(cell);
        }
    }

    for (TileCell(cell), mut sprite) in &mut tiles {
        let walkable = {
            let (cx, cy) = (cell.x as usize, cell.y as usize);
            cy < grid.0.len() && cx < grid.0[0].len() && grid.0[cy][cx]
        };
        let vis = if is_within_radius(*cell, player.0, SIGHT_RADIUS) {
            Visibility::Visible
        } else if revealed.0.contains(cell) {
            Visibility::Remembered
        } else {
            Visibility::Hidden
        };
        sprite.color = match (vis, walkable) {
            (Visibility::Visible, true) => Color::srgb(0.55, 0.5, 0.42),
            (Visibility::Visible, false) => Color::srgb(0.3, 0.3, 0.38),
            (Visibility::Remembered, true) => Color::srgb(0.2, 0.18, 0.16),
            (Visibility::Remembered, false) => Color::srgb(0.12, 0.12, 0.16),
            (Visibility::Hidden, _) => Color::BLACK,
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
    fn cells_in_radius_includes_center() {
        let center = IVec2::new(5, 5);
        let cells = cells_in_radius(center, 3);
        assert!(cells.contains(&center));
    }

    #[test]
    fn cells_in_radius_excludes_corners_for_radius_1() {
        // Radius 1 circle: only 5 cells (cross shape), not 9 (square).
        let cells = cells_in_radius(IVec2::ZERO, 1);
        assert_eq!(cells.len(), 5);
    }

    #[test]
    fn cells_in_radius_all_within_distance() {
        let center = IVec2::new(10, 10);
        let r = 4;
        for cell in cells_in_radius(center, r) {
            assert!(is_within_radius(cell, center, r));
        }
    }

    #[test]
    fn is_within_radius_center_always_true() {
        let c = IVec2::new(3, 7);
        assert!(is_within_radius(c, c, 0));
    }

    #[test]
    fn is_within_radius_far_cell_false() {
        let center = IVec2::ZERO;
        assert!(!is_within_radius(IVec2::new(10, 10), center, 5));
    }
}
