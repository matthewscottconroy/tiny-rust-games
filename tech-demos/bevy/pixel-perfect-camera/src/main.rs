//! Pixel-perfect camera demo — virtual 320×180 canvas at integer scale.
//!
//! Key ideas:
//! - The camera uses `ScalingMode::Fixed { width: 320, height: 180 }`, which
//!   makes the orthographic projection treat the window as exactly 320×180
//!   virtual pixels regardless of the real resolution.
//! - The window is 1280×720 — exactly 4× the virtual resolution — so every
//!   virtual pixel maps to a 4×4 block of real pixels with no sub-pixel blur.
//! - All entity positions are snapped to an 8-unit virtual grid before
//!   rendering so sprites never land between pixel boundaries.
//! - Player movement uses `just_pressed` so each keypress advances exactly one
//!   grid cell, giving a clean turn-feel without auto-repeat drift.
//!
//! **Controls:** WASD (one grid step per press).

use bevy::{
    camera::ScalingMode,
    math::{IVec2, UVec2},
    prelude::*,
    window::WindowResolution,
};

// ---------------------------------------------------------------------------
// Constants
// ---------------------------------------------------------------------------

/// Real window width in physical pixels.
const WIN_W: u32 = 1280;
/// Real window height in physical pixels.
const WIN_H: u32 = 720;

/// Virtual canvas width in virtual pixels.
const VIRT_W: u32 = 320;
/// Virtual canvas height in virtual pixels.
const VIRT_H: u32 = 180;

/// Size of one grid cell in virtual units (also the player sprite size).
const GRID: f32 = 8.0;

/// Number of grid cells across the canvas width.
const COLS: i32 = (VIRT_W as i32) / (GRID as i32); // 40
/// Number of grid cells across the canvas height.
const ROWS: i32 = (VIRT_H as i32) / (GRID as i32); // 22

// ---------------------------------------------------------------------------
// Components
// ---------------------------------------------------------------------------

/// Marks the player entity.
#[derive(Component)]
struct Player;

/// Stores the player's current virtual grid position (column, row).
#[derive(Component)]
struct GridPos(IVec2);

/// Marks the HUD label that shows the current player position.
#[derive(Component)]
struct PosLabel;

// ---------------------------------------------------------------------------
// main
// ---------------------------------------------------------------------------

fn main() {
    App::new()
        .add_plugins(DefaultPlugins.set(WindowPlugin {
            primary_window: Some(Window {
                title: "Pixel-Perfect Camera — WASD to move (grid-snapped)".to_string(),
                resolution: WindowResolution::new(WIN_W, WIN_H),
                ..default()
            }),
            ..default()
        }))
        .add_systems(Startup, setup)
        .add_systems(Update, (move_player, sync_player_transform, update_hud).chain())
        .run();
}

// ---------------------------------------------------------------------------
// Setup
// ---------------------------------------------------------------------------

/// Spawns the pixel-perfect camera, checkerboard floor, block obstacles, player, and HUD.
fn setup(mut commands: Commands) {
    // Camera with a fixed virtual resolution — 320×180 virtual pixels fills the 1280×720 window at 4×.
    commands.spawn((
        Camera2d,
        Projection::Orthographic(OrthographicProjection {
            scaling_mode: ScalingMode::Fixed {
                width: VIRT_W as f32,
                height: VIRT_H as f32,
            },
            ..OrthographicProjection::default_2d()
        }),
    ));

    // --- Checkerboard floor ---
    // Origin of the virtual canvas is (0,0) at centre. Tiles fill the whole area.
    let half_cols = COLS / 2;
    let half_rows = ROWS / 2;
    for row in -half_rows..half_rows {
        for col in -half_cols..half_cols {
            let dark = (row + col) % 2 == 0;
            let color = if dark {
                Color::srgb(0.18, 0.20, 0.24)
            } else {
                Color::srgb(0.22, 0.24, 0.30)
            };
            let world_pos = grid_to_world(IVec2::new(col, row));
            commands.spawn((
                Sprite {
                    color,
                    custom_size: Some(Vec2::splat(GRID)),
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
        let world_pos = grid_to_world(IVec2::new(col, row));
        commands.spawn((
            Sprite {
                color,
                custom_size: Some(Vec2::splat(GRID)),
                ..default()
            },
            Transform::from_xyz(world_pos.x, world_pos.y, 0.0),
        ));
    }

    // --- Player (starts at grid origin) ---
    let start = IVec2::ZERO;
    let start_world = grid_to_world(start);
    commands.spawn((
        Sprite {
            color: Color::srgb(1.0, 0.9, 0.2),
            custom_size: Some(Vec2::splat(GRID)),
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

// ---------------------------------------------------------------------------
// Systems
// ---------------------------------------------------------------------------

/// Reads WASD `just_pressed` and advances the player's [`GridPos`] by one cell.
fn move_player(
    input: Res<ButtonInput<KeyCode>>,
    mut query: Query<&mut GridPos, With<Player>>,
) {
    let Ok(mut gp) = query.single_mut() else { return; };

    let half_cols = COLS / 2 - 1;
    let half_rows = ROWS / 2 - 1;

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
    mut query: Query<(&GridPos, &mut Transform), With<Player>>,
) {
    let Ok((gp, mut transform)) = query.single_mut() else { return; };
    let world = grid_to_world(gp.0);
    let snapped = snap_to_grid(world, GRID);
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

// ---------------------------------------------------------------------------
// Pure helpers
// ---------------------------------------------------------------------------

/// Converts a grid cell coordinate to the world-space centre of that cell.
fn grid_to_world(cell: IVec2) -> Vec2 {
    Vec2::new(
        cell.x as f32 * GRID + GRID / 2.0,
        cell.y as f32 * GRID + GRID / 2.0,
    )
}

// ---------------------------------------------------------------------------
// Pure functions (testable without a running app)
// ---------------------------------------------------------------------------

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
/// The virtual canvas is centred in the window.  The returned `Vec2` gives the
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

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

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
