//! Tilemap demo.
//!
//! Key ideas:
//! - A `const` string-slice array (`MAP`) encodes tile types compactly.
//! - Tile sprites are spawned as children of a single root entity so that
//!   moving or scaling the root transforms the entire map.
//! - Two tile types (`#` = wall, `.` = floor) are distinguished by color;
//!   space characters are skipped (void tiles).

use bevy::prelude::*;

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .add_systems(Startup, setup)
        .run();
}

/// Map layout: `'#'` = wall, `'.'` = floor, `' '` = void (not spawned).
const MAP: &[&str] = &[
    "####################",
    "#..................#",
    "#.####.......####.#",
    "#.#..............#.#",
    "#.#..##...##..#..#.#",
    "#................. #",
    "#.##.........##....#",
    "#....###.###.......#",
    "#..................#",
    "#.#####...#####....#",
    "#..................#",
    "####################",
];

/// World-space size of a single tile in pixels.
const TILE_SIZE: f32 = 32.0;

const WALL_COLOR:  Color = Color::srgb(0.35, 0.28, 0.45);
const FLOOR_COLOR: Color = Color::srgb(0.18, 0.18, 0.22);

/// Spawns the camera, a root entity whose children are the tile sprites, and
/// an info label.  The map is centered on screen.
fn setup(mut commands: Commands) {
    commands.spawn(Camera2d);

    let rows = MAP.len() as f32;
    let cols = MAP.iter().map(|r| r.len()).max().unwrap_or(0) as f32;

    let offset_x = -(cols * TILE_SIZE) / 2.0 + TILE_SIZE / 2.0;
    let offset_y =  (rows * TILE_SIZE) / 2.0 - TILE_SIZE / 2.0;

    commands
        .spawn(Transform::default())
        .with_children(|parent| {
            for (row_idx, row_str) in MAP.iter().enumerate() {
                for (col_idx, ch) in row_str.chars().enumerate() {
                    let color = match ch {
                        '#' => WALL_COLOR,
                        '.' => FLOOR_COLOR,
                        _   => continue,
                    };

                    let x = offset_x + col_idx as f32 * TILE_SIZE;
                    let y = offset_y - row_idx as f32 * TILE_SIZE;

                    parent.spawn((
                        Sprite {
                            color,
                            custom_size: Some(Vec2::splat(TILE_SIZE - 1.0)),
                            ..default()
                        },
                        Transform::from_xyz(x, y, 0.0),
                    ));
                }
            }
        });

    commands.spawn((
        Text::new("Tilemap: walls (purple) and floor (dark) decoded from a const string slice"),
        TextFont { font_size: 13.0, ..default() },
        TextColor(Color::srgb(0.65, 0.65, 0.65)),
        Node {
            position_type: PositionType::Absolute,
            bottom: Val::Px(10.0),
            left: Val::Px(10.0),
            ..default()
        },
    ));
}

#[cfg(test)]
mod tests {
    use super::*;

    // --- MAP validation ---

    #[test]
    fn map_has_twelve_rows() {
        assert_eq!(MAP.len(), 12);
    }

    #[test]
    fn map_contains_wall_tiles() {
        let walls = MAP.iter().flat_map(|r| r.chars()).filter(|&c| c == '#').count();
        assert!(walls > 0, "map should contain wall tiles");
    }

    #[test]
    fn map_contains_floor_tiles() {
        let floors = MAP.iter().flat_map(|r| r.chars()).filter(|&c| c == '.').count();
        assert!(floors > 0, "map should contain floor tiles");
    }

    #[test]
    fn first_row_is_all_walls() {
        assert!(MAP[0].chars().all(|c| c == '#'),
            "border row should be all walls");
    }

    #[test]
    fn last_row_is_all_walls() {
        assert!(MAP[MAP.len() - 1].chars().all(|c| c == '#'),
            "border row should be all walls");
    }

    #[test]
    fn tile_size_is_positive() {
        assert!(TILE_SIZE > 0.0);
    }

    #[test]
    fn only_valid_tile_chars() {
        for row in MAP {
            for ch in row.chars() {
                assert!(
                    ch == '#' || ch == '.' || ch == ' ',
                    "unexpected tile character: '{ch}'"
                );
            }
        }
    }
}
