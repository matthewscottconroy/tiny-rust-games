//! Minimap demo — two `Camera2d` instances in one window.
//!
//! Key ideas:
//! - A **main camera** (render order 0) follows the player with an exponential
//!   lerp, keeping them centred on screen.
//! - A **minimap camera** (render order 1) is fixed at the world centre with a
//!   high orthographic scale so the entire play area fits in a small viewport
//!   rendered into the top-right corner of the window.
//! - [`Camera::viewport`] lets each camera draw into an independent rectangular
//!   region of the same window surface — no render targets required.
//!
//! **Controls:** WASD to move the player.

use bevy::{
    camera::Viewport,
    math::UVec2,
    prelude::*,
    window::WindowResolution,
};

// ---------------------------------------------------------------------------
// Constants
// ---------------------------------------------------------------------------

/// Width of the window in physical pixels.
const WIN_W: u32 = 1280;
/// Height of the window in physical pixels.
const WIN_H: u32 = 720;

/// Half-extent of the play area in world units (area is 4000×4000).
const MAP_HALF: f32 = 2000.0;

/// Tile size in world units.
const TILE_SIZE: f32 = 100.0;

/// Player movement speed in world units per second.
const PLAYER_SPEED: f32 = 200.0;

/// How quickly the main camera lerps toward the player (fraction of gap per second).
const CAM_LERP_SPEED: f32 = 5.0;

/// Physical size of the minimap viewport in pixels.
const MINIMAP_PX: u32 = 200;

/// Orthographic scale for the minimap camera (world units per half-viewport-height).
const MINIMAP_SCALE: f32 = 8.0;

// ---------------------------------------------------------------------------
// Components & markers
// ---------------------------------------------------------------------------

/// Marks the player entity.
#[derive(Component)]
struct Player;

/// Marks the main follow camera.
#[derive(Component)]
struct MainCamera;

/// Marks the minimap camera.
#[derive(Component)]
struct MinimapCamera;

// ---------------------------------------------------------------------------
// main
// ---------------------------------------------------------------------------

fn main() {
    App::new()
        .add_plugins(DefaultPlugins.set(WindowPlugin {
            primary_window: Some(Window {
                title: "Minimap — WASD to move | Minimap: top-right".to_string(),
                resolution: WindowResolution::new(WIN_W, WIN_H),
                ..default()
            }),
            ..default()
        }))
        .add_systems(Startup, setup)
        .add_systems(Update, (move_player, follow_main_camera).chain())
        .run();
}

// ---------------------------------------------------------------------------
// Setup
// ---------------------------------------------------------------------------

/// Spawns the two cameras, the tile grid, landmark sprites, the player, and the HUD.
fn setup(mut commands: Commands) {
    // --- Main camera (follows player) ---
    commands.spawn((
        Camera2d,
        Camera {
            order: 0,
            ..default()
        },
        MainCamera,
    ));

    // --- Minimap camera (fixed, zoomed out, corner viewport) ---
    commands.spawn((
        Camera2d,
        Camera {
            order: 1,
            viewport: Some(Viewport {
                physical_position: UVec2::new(WIN_W - MINIMAP_PX - 10, 10),
                physical_size: UVec2::new(MINIMAP_PX, MINIMAP_PX),
                ..default()
            }),
            ..default()
        },
        Projection::Orthographic(OrthographicProjection {
            scale: MINIMAP_SCALE,
            ..OrthographicProjection::default_2d()
        }),
        MinimapCamera,
    ));

    // --- Tile grid (40×40 tiles every 100 units) ---
    let tile_count = (MAP_HALF * 2.0 / TILE_SIZE) as i32; // 40
    let half_tiles = tile_count / 2;
    for row in -half_tiles..half_tiles {
        for col in -half_tiles..half_tiles {
            let dark = (row + col) % 2 == 0;
            let color = if dark {
                Color::srgb(0.12, 0.14, 0.18)
            } else {
                Color::srgb(0.16, 0.18, 0.24)
            };
            commands.spawn((
                Sprite {
                    color,
                    custom_size: Some(Vec2::splat(TILE_SIZE - 1.0)),
                    ..default()
                },
                Transform::from_xyz(
                    col as f32 * TILE_SIZE + TILE_SIZE / 2.0,
                    row as f32 * TILE_SIZE + TILE_SIZE / 2.0,
                    -1.0,
                ),
            ));
        }
    }

    // --- Landmark sprites (bright, scattered) ---
    let landmarks: &[(f32, f32, Color)] = &[
        (-800.0,  600.0, Color::srgb(0.9, 0.2, 0.2)),
        ( 500.0,  900.0, Color::srgb(0.2, 0.8, 0.3)),
        ( 1200.0, -400.0, Color::srgb(0.2, 0.4, 0.9)),
        (-1400.0, -700.0, Color::srgb(0.9, 0.7, 0.1)),
        ( 300.0, -1100.0, Color::srgb(0.8, 0.2, 0.8)),
        (-600.0,  1300.0, Color::srgb(0.1, 0.8, 0.8)),
        ( 1600.0,  800.0, Color::srgb(0.95, 0.5, 0.1)),
        (-1700.0,  200.0, Color::srgb(0.5, 0.9, 0.2)),
        ( 900.0, -1500.0, Color::srgb(0.2, 0.6, 0.95)),
        (-300.0, -800.0,  Color::srgb(0.9, 0.3, 0.6)),
        ( 1800.0, -1600.0, Color::srgb(0.7, 0.9, 0.2)),
        (-1900.0,  1600.0, Color::srgb(0.95, 0.85, 0.3)),
    ];

    for &(x, y, color) in landmarks {
        commands.spawn((
            Sprite {
                color,
                custom_size: Some(Vec2::splat(60.0)),
                ..default()
            },
            Transform::from_xyz(x, y, 0.0),
        ));
    }

    // --- White border visible in minimap (slightly larger than minimap view) ---
    // Spawned at z=10 so it renders on top of tiles.
    commands.spawn((
        Sprite {
            color: Color::WHITE,
            custom_size: Some(Vec2::splat(MAP_HALF * 2.0 + 40.0)),
            ..default()
        },
        Transform::from_xyz(0.0, 0.0, -2.0),
    ));

    // --- Player ---
    commands.spawn((
        Sprite {
            color: Color::srgb(1.0, 0.85, 0.1),
            custom_size: Some(Vec2::splat(32.0)),
            ..default()
        },
        Transform::from_xyz(0.0, 0.0, 1.0),
        Player,
    ));

    // --- HUD label ---
    commands.spawn((
        Text::new("WASD to move  |  Minimap: top-right"),
        TextFont { font_size: 18.0, ..default() },
        TextColor(Color::WHITE),
        Node {
            position_type: PositionType::Absolute,
            top: Val::Px(8.0),
            left: Val::Px(8.0),
            ..default()
        },
    ));
}

// ---------------------------------------------------------------------------
// Systems
// ---------------------------------------------------------------------------

/// Reads WASD and moves the player at [`PLAYER_SPEED`] units/second.
fn move_player(
    time: Res<Time>,
    input: Res<ButtonInput<KeyCode>>,
    mut query: Query<&mut Transform, With<Player>>,
) {
    let Ok(mut transform) = query.single_mut() else { return; };

    let mut dir = Vec2::ZERO;
    if input.pressed(KeyCode::KeyW) { dir.y += 1.0; }
    if input.pressed(KeyCode::KeyS) { dir.y -= 1.0; }
    if input.pressed(KeyCode::KeyA) { dir.x -= 1.0; }
    if input.pressed(KeyCode::KeyD) { dir.x += 1.0; }

    if dir != Vec2::ZERO {
        let delta = dir.normalize() * PLAYER_SPEED * time.delta_secs();
        let new_pos = Vec2::new(
            transform.translation.x + delta.x,
            transform.translation.y + delta.y,
        );
        // Keep player within map bounds.
        let clamped = new_pos.clamp(Vec2::splat(-MAP_HALF + 16.0), Vec2::splat(MAP_HALF - 16.0));
        transform.translation.x = clamped.x;
        transform.translation.y = clamped.y;
    }
}

/// Smoothly moves the main camera toward the player each frame.
fn follow_main_camera(
    time: Res<Time>,
    player_q: Query<&Transform, (With<Player>, Without<MainCamera>)>,
    mut cam_q: Query<&mut Transform, (With<MainCamera>, Without<Player>)>,
) {
    let Ok(player) = player_q.single() else { return; };
    let Ok(mut cam) = cam_q.single_mut() else { return; };

    let current = cam.translation.truncate();
    let target = player.translation.truncate();
    let new_pos = smooth_follow(current, target, CAM_LERP_SPEED, time.delta_secs());
    cam.translation.x = new_pos.x;
    cam.translation.y = new_pos.y;
}

// ---------------------------------------------------------------------------
// Pure functions (testable without a running app)
// ---------------------------------------------------------------------------

/// Lerps `current` toward `target` by `speed * dt`, returning the new position.
///
/// Uses an exponential-decay formulation: each second closes `speed` fraction of
/// the remaining gap, giving frame-rate-independent smoothing.
pub fn smooth_follow(current: Vec2, target: Vec2, speed: f32, dt: f32) -> Vec2 {
    current.lerp(target, (speed * dt).min(1.0))
}

/// Maps a world position to a normalised `[0, 1]` minimap UV coordinate.
///
/// Returns `(0.5, 0.5)` for the world origin and `(1.0, 1.0)` for the
/// `+x, +y` corner.  Values outside `[-map_half_size, map_half_size]` are
/// clamped by the caller if desired.
pub fn world_to_minimap_uv(world_pos: Vec2, map_half_size: Vec2) -> Vec2 {
    Vec2::new(
        (world_pos.x / map_half_size.x) * 0.5 + 0.5,
        (world_pos.y / map_half_size.y) * 0.5 + 0.5,
    )
}

/// Clamps `pos` so a camera centred there keeps its view within the map.
///
/// `half_view` is the half-extent of the camera frustum in world units.
/// `map_half_size` is the half-extent of the play area.
pub fn clamp_camera_to_map(pos: Vec2, half_view: Vec2, map_half_size: Vec2) -> Vec2 {
    let min = half_view - map_half_size;
    let max = map_half_size - half_view;
    pos.clamp(min, max)
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    // --- smooth_follow ---

    #[test]
    fn smooth_follow_at_zero_dt_returns_current() {
        let current = Vec2::new(10.0, -5.0);
        let target = Vec2::new(100.0, 200.0);
        let result = smooth_follow(current, target, 5.0, 0.0);
        assert!((result - current).length() < 1e-5);
    }

    #[test]
    fn smooth_follow_large_dt_clamps_to_target() {
        // speed * dt > 1 → lerp factor clamped to 1 → should reach target exactly.
        let current = Vec2::new(0.0, 0.0);
        let target = Vec2::new(500.0, 300.0);
        let result = smooth_follow(current, target, 10.0, 1.0);
        assert!((result - target).length() < 1e-5);
    }

    #[test]
    fn smooth_follow_moves_toward_target() {
        let current = Vec2::new(0.0, 0.0);
        let target = Vec2::new(100.0, 0.0);
        let result = smooth_follow(current, target, 5.0, 0.016);
        // Should have moved closer to target, not away.
        assert!(result.x > current.x);
        assert!(result.x < target.x);
    }

    #[test]
    fn smooth_follow_already_at_target_stays() {
        let pos = Vec2::new(42.0, -17.0);
        let result = smooth_follow(pos, pos, 5.0, 0.016);
        assert!((result - pos).length() < 1e-5);
    }

    // --- world_to_minimap_uv ---

    #[test]
    fn minimap_uv_origin_is_centre() {
        let uv = world_to_minimap_uv(Vec2::ZERO, Vec2::new(2000.0, 2000.0));
        assert!((uv.x - 0.5).abs() < 1e-5);
        assert!((uv.y - 0.5).abs() < 1e-5);
    }

    #[test]
    fn minimap_uv_positive_corner_is_one() {
        let half = Vec2::new(2000.0, 2000.0);
        let uv = world_to_minimap_uv(half, half);
        assert!((uv.x - 1.0).abs() < 1e-5);
        assert!((uv.y - 1.0).abs() < 1e-5);
    }

    #[test]
    fn minimap_uv_negative_corner_is_zero() {
        let half = Vec2::new(2000.0, 2000.0);
        let uv = world_to_minimap_uv(-half, half);
        assert!(uv.x.abs() < 1e-5);
        assert!(uv.y.abs() < 1e-5);
    }

    #[test]
    fn minimap_uv_non_square_map() {
        let half = Vec2::new(1000.0, 500.0);
        let uv = world_to_minimap_uv(Vec2::new(500.0, 250.0), half);
        assert!((uv.x - 0.75).abs() < 1e-5);
        assert!((uv.y - 0.75).abs() < 1e-5);
    }

    // --- clamp_camera_to_map ---

    #[test]
    fn clamp_camera_origin_within_large_map() {
        let result = clamp_camera_to_map(Vec2::ZERO, Vec2::new(100.0, 100.0), Vec2::new(2000.0, 2000.0));
        assert!((result - Vec2::ZERO).length() < 1e-5);
    }

    #[test]
    fn clamp_camera_far_right_is_pulled_back() {
        // Camera at x=2000 with half_view=200 and map_half=2000 → max_x = 1800.
        let pos = Vec2::new(2000.0, 0.0);
        let result = clamp_camera_to_map(pos, Vec2::new(200.0, 200.0), Vec2::new(2000.0, 2000.0));
        assert!((result.x - 1800.0).abs() < 1e-5);
    }

    #[test]
    fn clamp_camera_far_left_is_pulled_back() {
        let pos = Vec2::new(-3000.0, 0.0);
        let result = clamp_camera_to_map(pos, Vec2::new(200.0, 200.0), Vec2::new(2000.0, 2000.0));
        assert!((result.x - (-1800.0)).abs() < 1e-5);
    }
}
