//! Pixel-perfect camera — a reusable Bevy plugin.
//!
//! This crate is a *building block*: drop [`PixelPerfectCameraPlugin`] into any
//! Bevy app with `app.add_plugins(PixelPerfectCameraPlugin)` and it sets up a
//! virtual 320×180 canvas rendered at an integer scale, with a grid-snapped
//! player you move one cell per keypress.
//!
//! Key ideas:
//! - The camera uses `ScalingMode::Fixed { width, height }`, which makes the
//!   orthographic projection treat the window as exactly the virtual resolution
//!   regardless of the real window size.
//! - When the window is an exact integer multiple of the virtual resolution
//!   (e.g. 1280×720 = 4× 320×180), every virtual pixel maps to a square block
//!   of real pixels with no sub-pixel blur.
//! - All entity positions are snapped to a grid via [`snap_to_grid`] before
//!   rendering so sprites never land between pixel boundaries.
//! - Player movement uses `just_pressed` so each keypress advances exactly one
//!   grid cell.
//!
//! Tune the virtual resolution and grid cell size through the
//! [`PixelPerfectConfig`] resource. The host binary owns the real window size
//! (see the crate's `main.rs`), which stays out of the plugin.
//!
//! **Controls:** WASD (one grid step per press).
//!
//! # Example
//! ```no_run
//! use bevy::prelude::*;
//! use pixel_perfect_camera::PixelPerfectCameraPlugin;
//!
//! App::new()
//!     .add_plugins(DefaultPlugins)
//!     .add_plugins(PixelPerfectCameraPlugin)
//!     .run();
//! ```

use bevy::{
    camera::ScalingMode,
    math::{IVec2, UVec2},
    prelude::*,
};

/// Bundles the camera setup, config, and systems for the pixel-perfect feature.
///
/// Add it with `app.add_plugins(PixelPerfectCameraPlugin)`. It does **not** add
/// `DefaultPlugins`, so the host app stays in control of the real window size.
pub struct PixelPerfectCameraPlugin;

impl Plugin for PixelPerfectCameraPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<PixelPerfectConfig>()
            .add_systems(Startup, setup)
            .add_systems(Update, (move_player, sync_player_transform, update_hud).chain());
    }
}

// --- Configuration ---

/// Tunable parameters for the feature. Override before adding the plugin, e.g.
/// `app.insert_resource(PixelPerfectConfig { virtual_width: 256, ..default() })`.
#[derive(Resource, Clone, Copy, Debug)]
pub struct PixelPerfectConfig {
    /// Virtual canvas width in virtual pixels.
    pub virtual_width: u32,
    /// Virtual canvas height in virtual pixels.
    pub virtual_height: u32,
    /// Size of one grid cell in virtual units (also the player sprite size).
    pub grid: f32,
}

impl Default for PixelPerfectConfig {
    fn default() -> Self {
        Self { virtual_width: 320, virtual_height: 180, grid: 8.0 }
    }
}

impl PixelPerfectConfig {
    /// Number of grid cells across the canvas width.
    pub fn cols(&self) -> i32 {
        (self.virtual_width as i32) / (self.grid as i32)
    }

    /// Number of grid cells across the canvas height.
    pub fn rows(&self) -> i32 {
        (self.virtual_height as i32) / (self.grid as i32)
    }
}

// --- Components ---

/// Marks the player entity.
#[derive(Component)]
pub struct Player;

/// Stores the player's current virtual grid position (column, row).
#[derive(Component)]
pub struct GridPos(pub IVec2);

/// Marks the HUD label that shows the current player position.
#[derive(Component)]
pub struct PosLabel;

// --- Setup ---

/// Spawns the pixel-perfect camera, checkerboard floor, block obstacles, player, and HUD.
fn setup(mut commands: Commands, config: Res<PixelPerfectConfig>) {
    let grid = config.grid;
    let cols = config.cols();
    let rows = config.rows();

    // Camera with a fixed virtual resolution.
    commands.spawn((
        Camera2d,
        Projection::Orthographic(OrthographicProjection {
            scaling_mode: ScalingMode::Fixed {
                width: config.virtual_width as f32,
                height: config.virtual_height as f32,
            },
            ..OrthographicProjection::default_2d()
        }),
    ));

    // --- Checkerboard floor ---
    // Origin of the virtual canvas is (0,0) at centre. Tiles fill the whole area.
    let half_cols = cols / 2;
    let half_rows = rows / 2;
    for row in -half_rows..half_rows {
        for col in -half_cols..half_cols {
            let dark = (row + col) % 2 == 0;
            let color = if dark {
                Color::srgb(0.18, 0.20, 0.24)
            } else {
                Color::srgb(0.22, 0.24, 0.30)
            };
            let world_pos = grid_to_world(IVec2::new(col, row), grid);
            commands.spawn((
                Sprite {
                    color,
                    custom_size: Some(Vec2::splat(grid)),
                    ..default()
                },
                Transform::from_xyz(world_pos.x, world_pos.y, -1.0),
            ));
        }
    }

    // --- Block obstacles ---
    let obstacles: &[(i32, i32, Color)] = &[
        ( 5,  3, Color::srgb(0.7, 0.2, 0.2)),
        (-6,  2, Color::srgb(0.2, 0.6, 0.3)),
        ( 3, -4, Color::srgb(0.2, 0.3, 0.8)),
        (-4, -3, Color::srgb(0.7, 0.6, 0.1)),
        ( 8,  0, Color::srgb(0.6, 0.2, 0.7)),
        (-9,  1, Color::srgb(0.1, 0.6, 0.7)),
    ];

    for &(col, row, color) in obstacles {
        let world_pos = grid_to_world(IVec2::new(col, row), grid);
        commands.spawn((
            Sprite {
                color,
                custom_size: Some(Vec2::splat(grid)),
                ..default()
            },
            Transform::from_xyz(world_pos.x, world_pos.y, 0.0),
        ));
    }

    // --- Player (starts at grid origin) ---
    let start = IVec2::ZERO;
    let start_world = grid_to_world(start, grid);
    commands.spawn((
        Sprite {
            color: Color::srgb(1.0, 0.9, 0.2),
            custom_size: Some(Vec2::splat(grid)),
            ..default()
        },
        Transform::from_xyz(start_world.x, start_world.y, 1.0),
        Player,
        GridPos(start),
    ));

    // --- HUD label ---
    commands.spawn((
        Text::new("grid (0, 0)"),
        TextFont { font_size: 8.0, ..default() },
        TextColor(Color::WHITE),
        Node {
            position_type: PositionType::Absolute,
            top: Val::Px(2.0),
            left: Val::Px(2.0),
            ..default()
        },
        PosLabel,
    ));
}

// --- Systems ---

/// Reads WASD `just_pressed` and advances the player's [`GridPos`] by one cell.
fn move_player(
    input: Res<ButtonInput<KeyCode>>,
    config: Res<PixelPerfectConfig>,
    mut query: Query<&mut GridPos, With<Player>>,
) {
    let Ok(mut gp) = query.single_mut() else { return; };

    let half_cols = config.cols() / 2 - 1;
    let half_rows = config.rows() / 2 - 1;

    if input.just_pressed(KeyCode::KeyW) {
        gp.0.y = (gp.0.y + 1).min(half_rows);
    }
    if input.just_pressed(KeyCode::KeyS) {
        gp.0.y = (gp.0.y - 1).max(-half_rows);
    }
    if input.just_pressed(KeyCode::KeyD) {
        gp.0.x = (gp.0.x + 1).min(half_cols);
    }
    if input.just_pressed(KeyCode::KeyA) {
        gp.0.x = (gp.0.x - 1).max(-half_cols);
    }
}

/// Writes the player's world-space `Transform` from their [`GridPos`], grid-snapped.
fn sync_player_transform(
    config: Res<PixelPerfectConfig>,
    mut query: Query<(&GridPos, &mut Transform), With<Player>>,
) {
    let Ok((gp, mut transform)) = query.single_mut() else { return; };
    let world = grid_to_world(gp.0, config.grid);
    let snapped = snap_to_grid(world, config.grid);
    transform.translation.x = snapped.x;
    transform.translation.y = snapped.y;
}

/// Updates the HUD label to reflect the player's current grid position.
fn update_hud(
    player_q: Query<&GridPos, With<Player>>,
    mut label_q: Query<&mut Text, With<PosLabel>>,
) {
    let Ok(gp) = player_q.single() else { return; };
    let Ok(mut text) = label_q.single_mut() else { return; };
    **text = format!("grid ({}, {})", gp.0.x, gp.0.y);
}

// --- Pure helpers ---

/// Converts a grid cell coordinate to the world-space centre of that cell.
pub fn grid_to_world(cell: IVec2, grid: f32) -> Vec2 {
    Vec2::new(
        cell.x as f32 * grid + grid / 2.0,
        cell.y as f32 * grid + grid / 2.0,
    )
}

/// Snaps each component of `pos` to the nearest multiple of `grid`.
///
/// Guarantees that sprite positions always land on integer pixel boundaries
/// when the virtual pixel size equals `grid` world units.
pub fn snap_to_grid(pos: Vec2, grid: f32) -> Vec2 {
    Vec2::new(
        (pos.x / grid).round() * grid,
        (pos.y / grid).round() * grid,
    )
}

/// Returns the largest integer scale factor that fits `virtual_size` into `window_size`.
///
/// Useful for computing how many physical pixels correspond to one virtual pixel.
/// Returns `1` if the window is smaller than the virtual canvas.
pub fn integer_scale(virtual_size: UVec2, window_size: UVec2) -> u32 {
    let sx = window_size.x / virtual_size.x;
    let sy = window_size.y / virtual_size.y;
    sx.min(sy).max(1)
}

/// Converts a virtual-pixel position to a window-pixel position at `scale`.
///
/// The virtual canvas is centred in the window. The returned `Vec2` gives the
/// top-left corner of the scaled virtual pixel in window-pixel space.
pub fn virtual_to_window(virtual_pos: IVec2, scale: u32, window_size: UVec2, virtual_size: UVec2) -> Vec2 {
    let canvas_w = virtual_size.x * scale;
    let canvas_h = virtual_size.y * scale;
    let offset_x = (window_size.x.saturating_sub(canvas_w)) / 2;
    let offset_y = (window_size.y.saturating_sub(canvas_h)) / 2;
    Vec2::new(
        virtual_pos.x as f32 * scale as f32 + offset_x as f32,
        virtual_pos.y as f32 * scale as f32 + offset_y as f32,
    )
}

// --- Tests ---

#[cfg(test)]
mod tests {
    use super::*;

    // --- config ---

    #[test]
    fn config_default_matches_documented_values() {
        let c = PixelPerfectConfig::default();
        assert_eq!(c.virtual_width, 320);
        assert_eq!(c.virtual_height, 180);
        assert_eq!(c.grid, 8.0);
        assert_eq!(c.cols(), 40);
        assert_eq!(c.rows(), 22);
    }

    // --- snap_to_grid ---

    #[test]
    fn snap_already_on_grid_unchanged() {
        let p = Vec2::new(16.0, 24.0);
        let snapped = snap_to_grid(p, 8.0);
        assert!((snapped.x - 16.0).abs() < 1e-5);
        assert!((snapped.y - 24.0).abs() < 1e-5);
    }

    #[test]
    fn snap_rounds_to_nearest_multiple() {
        // 13.0 / 8.0 = 1.625 → rounds to 2 → 16.0
        let p = Vec2::new(13.0, 3.0);
        let snapped = snap_to_grid(p, 8.0);
        assert!((snapped.x - 16.0).abs() < 1e-5);
        assert!((snapped.y - 0.0).abs() < 1e-5);
    }

    #[test]
    fn snap_negative_position() {
        // -13.0 / 8.0 = -1.625 → rounds to -2 → -16.0
        let p = Vec2::new(-13.0, -3.0);
        let snapped = snap_to_grid(p, 8.0);
        assert!((snapped.x - (-16.0)).abs() < 1e-5);
        assert!((snapped.y - 0.0).abs() < 1e-5);
    }

    #[test]
    fn snap_grid_size_one_is_round() {
        let p = Vec2::new(3.7, -2.3);
        let snapped = snap_to_grid(p, 1.0);
        assert!((snapped.x - 4.0).abs() < 1e-5);
        assert!((snapped.y - (-2.0)).abs() < 1e-5);
    }

    // --- grid_to_world ---

    #[test]
    fn grid_to_world_centres_cell() {
        let w = grid_to_world(IVec2::new(0, 0), 8.0);
        assert!((w.x - 4.0).abs() < 1e-5);
        assert!((w.y - 4.0).abs() < 1e-5);
    }

    // --- integer_scale ---

    #[test]
    fn integer_scale_exact_4x() {
        let scale = integer_scale(UVec2::new(320, 180), UVec2::new(1280, 720));
        assert_eq!(scale, 4);
    }

    #[test]
    fn integer_scale_constrained_by_height() {
        // Width allows 4×, height allows 3× → result is 3.
        let scale = integer_scale(UVec2::new(320, 180), UVec2::new(1280, 540));
        assert_eq!(scale, 3);
    }

    #[test]
    fn integer_scale_window_smaller_than_virtual_returns_one() {
        let scale = integer_scale(UVec2::new(320, 180), UVec2::new(160, 90));
        assert_eq!(scale, 1);
    }

    #[test]
    fn integer_scale_exact_2x() {
        let scale = integer_scale(UVec2::new(320, 180), UVec2::new(640, 360));
        assert_eq!(scale, 2);
    }

    // --- virtual_to_window ---

    #[test]
    fn virtual_to_window_origin_at_canvas_offset() {
        // 4× scale, 1280×720 window, 320×180 virtual → canvas is 1280×720, offset is (0,0).
        let pos = virtual_to_window(IVec2::ZERO, 4, UVec2::new(1280, 720), UVec2::new(320, 180));
        assert!((pos.x - 0.0).abs() < 1e-5);
        assert!((pos.y - 0.0).abs() < 1e-5);
    }

    #[test]
    fn virtual_to_window_centred_canvas_has_correct_offset() {
        // 2× scale, 800×600 window, 320×240 virtual → canvas 640×480, offset (80, 60).
        let pos = virtual_to_window(IVec2::ZERO, 2, UVec2::new(800, 600), UVec2::new(320, 240));
        assert!((pos.x - 80.0).abs() < 1e-5);
        assert!((pos.y - 60.0).abs() < 1e-5);
    }
}
