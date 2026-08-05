//! Tilemap — a reusable Bevy plugin.
//!
//! This crate is a *building block*: drop [`TilemapPlugin`] into any Bevy app
//! with `app.add_plugins(TilemapPlugin)` and it decodes a compact string-slice
//! map into colored tile sprites parented under a single root entity. Tune it
//! through the [`TilemapConfig`] resource without editing the plugin's
//! internals.
//!
//! Key ideas:
//! - A `const` string-slice array ([`MAP`]) encodes tile types compactly.
//! - Tile sprites are spawned as children of a single root entity so that
//!   moving or scaling the root transforms the entire map.
//! - Two tile types (`#` = wall, `.` = floor) are distinguished by color;
//!   space characters are skipped (void tiles).
//!
//! # Example
//! ```no_run
//! use bevy::prelude::*;
//! use tilemap::TilemapPlugin;
//!
//! App::new()
//!     .add_plugins(DefaultPlugins)
//!     .add_plugins(TilemapPlugin)
//!     .run();
//! ```

use bevy::prelude::*;

/// Bundles the setup system and configuration for the tilemap feature.
///
/// Add it with `app.add_plugins(TilemapPlugin)`. It does **not** add
/// `DefaultPlugins`, so the host app stays in control of window and rendering
/// setup. This is a fresh type defined in this crate; Bevy's prelude does not
/// export a `TilemapPlugin`, so there is no conflict.
pub struct TilemapPlugin;

impl Plugin for TilemapPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<TilemapConfig>()
            .add_systems(Startup, setup);
    }
}

// --- Configuration ---

/// Tunable parameters for the feature. Override before adding the plugin, e.g.
/// `app.insert_resource(TilemapConfig { tile_size: 48.0, ..default() })`.
#[derive(Resource, Clone, Copy, Debug)]
pub struct TilemapConfig {
    /// World-space size of a single tile in pixels.
    pub tile_size: f32,
    /// Color used for wall (`'#'`) tiles.
    pub wall_color: Color,
    /// Color used for floor (`'.'`) tiles.
    pub floor_color: Color,
}

impl Default for TilemapConfig {
    fn default() -> Self {
        Self {
            tile_size: 32.0,
            wall_color: Color::srgb(0.35, 0.28, 0.45),
            floor_color: Color::srgb(0.18, 0.18, 0.22),
        }
    }
}

/// Map layout: `'#'` = wall, `'.'` = floor, `' '` = void (not spawned).
pub const MAP: &[&str] = &[
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

/// Spawns the camera, a root entity whose children are the tile sprites, and
/// an info label.  The map is centered on screen.
fn setup(mut commands: Commands, config: Res<TilemapConfig>) {
    commands.spawn(Camera2d);

    let tile_size = config.tile_size;
    let rows = MAP.len() as f32;
    let cols = MAP.iter().map(|r| r.len()).max().unwrap_or(0) as f32;

    let offset_x = -(cols * tile_size) / 2.0 + tile_size / 2.0;
    let offset_y =  (rows * tile_size) / 2.0 - tile_size / 2.0;

    commands
        .spawn(Transform::default())
        .with_children(|parent| {
            for (row_idx, row_str) in MAP.iter().enumerate() {
                for (col_idx, ch) in row_str.chars().enumerate() {
                    let color = match ch {
                        '#' => config.wall_color,
                        '.' => config.floor_color,
                        _   => continue,
                    };

                    let x = offset_x + col_idx as f32 * tile_size;
                    let y = offset_y - row_idx as f32 * tile_size;

                    parent.spawn((
                        Sprite {
                            color,
                            custom_size: Some(Vec2::splat(tile_size - 1.0)),
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
        assert!(TilemapConfig::default().tile_size > 0.0);
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

    #[test]
    fn setup_spawns_tiles_under_root() {
        // Building-block path: the plugin composes onto a headless app. Setup
        // spawns only sprites/text (no AssetServer), so MinimalPlugins is fine.
        let mut app = App::new();
        app.add_plugins((MinimalPlugins, TilemapPlugin));
        app.update();

        // Count spawned tile sprites: one per '#' or '.' character.
        let expected = MAP
            .iter()
            .flat_map(|r| r.chars())
            .filter(|&c| c == '#' || c == '.')
            .count();
        let mut q = app.world_mut().query::<&Sprite>();
        assert_eq!(q.iter(app.world()).count(), expected);
    }
}
