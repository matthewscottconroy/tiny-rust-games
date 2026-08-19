//! Minimap demo — two `Camera2d` instances in one window, as a reusable plugin.
//!
//! This crate is a *building block*: drop [`MinimapPlugin`] into any Bevy app
//! with `app.add_plugins(MinimapPlugin)` and it renders a scrolling play area
//! with a follow camera plus a fixed, zoomed-out minimap in the corner. Tune it
//! through the [`MinimapConfig`] resource without editing the plugin's internals.
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
//!
//! # Example
//! ```no_run
//! use bevy::prelude::*;
//! use minimap::MinimapPlugin;
//!
//! App::new()
//!     .add_plugins(DefaultPlugins)
//!     .add_plugins(MinimapPlugin)
//!     .run();
//! ```
//!
//! Counterpart: tech-demos/godot/minimap — the same concept in Godot.

use bevy::{camera::Viewport, math::UVec2, prelude::*};

/// Bundles every system and resource for the minimap feature.
///
/// Add it with `app.add_plugins(MinimapPlugin)`. It does **not** add
/// `DefaultPlugins`, so the host app stays in control of window and rendering
/// setup.
pub struct MinimapPlugin;

impl Plugin for MinimapPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<MinimapConfig>()
            .add_systems(Startup, setup)
            .add_systems(Update, (move_player, follow_main_camera).chain());
    }
}

// ---------------------------------------------------------------------------
// Configuration
// ---------------------------------------------------------------------------

/// Width of the default window in physical pixels (used by `main`).
pub const WIN_W: u32 = 1280;
/// Height of the default window in physical pixels (used by `main`).
pub const WIN_H: u32 = 720;

/// Tunable parameters for the minimap feature. Override before adding the
/// plugin, e.g. `app.insert_resource(MinimapConfig { player_speed: 400.0, ..default() })`.
#[derive(Resource, Clone, Copy, Debug)]
pub struct MinimapConfig {
    /// Half-extent of the play area in world units (area is `2*map_half` square).
    pub map_half: f32,
    /// Tile size in world units.
    pub tile_size: f32,
    /// Player movement speed in world units per second.
    pub player_speed: f32,
    /// How quickly the main camera lerps toward the player (fraction of gap/sec).
    pub cam_lerp_speed: f32,
    /// Physical size of the minimap viewport in pixels.
    pub minimap_px: u32,
    /// Orthographic scale for the minimap camera.
    pub minimap_scale: f32,
    /// Window width in physical pixels (used to place the minimap viewport).
    pub win_w: u32,
}

impl Default for MinimapConfig {
    fn default() -> Self {
        Self {
            map_half: 2000.0,
            tile_size: 100.0,
            player_speed: 200.0,
            cam_lerp_speed: 5.0,
            minimap_px: 200,
            minimap_scale: 8.0,
            win_w: WIN_W,
        }
    }
}

// ---------------------------------------------------------------------------
// Components & markers
// ---------------------------------------------------------------------------

/// Marks the player entity.
#[derive(Component)]
pub struct Player;

/// Marks the main follow camera.
#[derive(Component)]
pub struct MainCamera;

/// Marks the minimap camera.
#[derive(Component)]
pub struct MinimapCamera;

// ---------------------------------------------------------------------------
// Setup
// ---------------------------------------------------------------------------

/// Spawns the two cameras, the tile grid, landmark sprites, the player, and the HUD.
fn setup(mut commands: Commands, config: Res<MinimapConfig>) {
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
                physical_position: UVec2::new(config.win_w - config.minimap_px - 10, 10),
                physical_size: UVec2::new(config.minimap_px, config.minimap_px),
                ..default()
            }),
            ..default()
        },
        Projection::Orthographic(OrthographicProjection {
            scale: config.minimap_scale,
            ..OrthographicProjection::default_2d()
        }),
        MinimapCamera,
    ));

    // --- Tile grid (every `tile_size` units) ---
    let tile_count = (config.map_half * 2.0 / config.tile_size) as i32; // 40
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
                    custom_size: Some(Vec2::splat(config.tile_size - 1.0)),
                    ..default()
                },
                Transform::from_xyz(
                    col as f32 * config.tile_size + config.tile_size / 2.0,
                    row as f32 * config.tile_size + config.tile_size / 2.0,
                    -1.0,
                ),
            ));
        }
    }

    // --- Landmark sprites (bright, scattered) ---
    let landmarks: &[(f32, f32, Color)] = &[
        (-800.0, 600.0, Color::srgb(0.9, 0.2, 0.2)),
        (500.0, 900.0, Color::srgb(0.2, 0.8, 0.3)),
        (1200.0, -400.0, Color::srgb(0.2, 0.4, 0.9)),
        (-1400.0, -700.0, Color::srgb(0.9, 0.7, 0.1)),
        (300.0, -1100.0, Color::srgb(0.8, 0.2, 0.8)),
        (-600.0, 1300.0, Color::srgb(0.1, 0.8, 0.8)),
        (1600.0, 800.0, Color::srgb(0.95, 0.5, 0.1)),
        (-1700.0, 200.0, Color::srgb(0.5, 0.9, 0.2)),
        (900.0, -1500.0, Color::srgb(0.2, 0.6, 0.95)),
        (-300.0, -800.0, Color::srgb(0.9, 0.3, 0.6)),
        (1800.0, -1600.0, Color::srgb(0.7, 0.9, 0.2)),
        (-1900.0, 1600.0, Color::srgb(0.95, 0.85, 0.3)),
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
            custom_size: Some(Vec2::splat(config.map_half * 2.0 + 40.0)),
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
        TextFont {
            font_size: 18.0,
            ..default()
        },
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

/// Reads WASD and moves the player at [`MinimapConfig::player_speed`] units/second.
fn move_player(
    time: Res<Time>,
    input: Res<ButtonInput<KeyCode>>,
    config: Res<MinimapConfig>,
    mut query: Query<&mut Transform, With<Player>>,
) {
    let Ok(mut transform) = query.single_mut() else {
        return;
    };

    let mut dir = Vec2::ZERO;
    if input.pressed(KeyCode::KeyW) {
        dir.y += 1.0;
    }
    if input.pressed(KeyCode::KeyS) {
        dir.y -= 1.0;
    }
    if input.pressed(KeyCode::KeyA) {
        dir.x -= 1.0;
    }
    if input.pressed(KeyCode::KeyD) {
        dir.x += 1.0;
    }

    if dir != Vec2::ZERO {
        let delta = dir.normalize() * config.player_speed * time.delta_secs();
        let new_pos = Vec2::new(
            transform.translation.x + delta.x,
            transform.translation.y + delta.y,
        );
        // Keep player within map bounds.
        let clamped = new_pos.clamp(
            Vec2::splat(-config.map_half + 16.0),
            Vec2::splat(config.map_half - 16.0),
        );
        transform.translation.x = clamped.x;
        transform.translation.y = clamped.y;
    }
}

/// Smoothly moves the main camera toward the player each frame.
fn follow_main_camera(
    time: Res<Time>,
    config: Res<MinimapConfig>,
    player_q: Query<&Transform, (With<Player>, Without<MainCamera>)>,
    mut cam_q: Query<&mut Transform, (With<MainCamera>, Without<Player>)>,
) {
    let Ok(player) = player_q.single() else {
        return;
    };
    let Ok(mut cam) = cam_q.single_mut() else {
        return;
    };

    let current = cam.translation.truncate();
    let target = player.translation.truncate();
    let new_pos = smooth_follow(current, target, config.cam_lerp_speed, time.delta_secs());
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
        let result = clamp_camera_to_map(
            Vec2::ZERO,
            Vec2::new(100.0, 100.0),
            Vec2::new(2000.0, 2000.0),
        );
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

    // --- config ---

    #[test]
    fn config_default_matches_documented_values() {
        let c = MinimapConfig::default();
        assert_eq!(c.player_speed, 200.0);
        assert_eq!(c.map_half, 2000.0);
    }
}
