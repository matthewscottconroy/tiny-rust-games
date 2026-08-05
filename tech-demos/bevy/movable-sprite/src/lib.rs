//! Movable sprite — WASD movement applied to a sprite entity.
//!
//! This crate is a *building block*: drop [`MovableSpritePlugin`] into any Bevy
//! app with `app.add_plugins(MovableSpritePlugin)` and it spawns a sprite the
//! player can move with WASD. Tune it through [`MovableSpriteConfig`].
//!
//! Key idea: the movement system reads from `ButtonInput<KeyCode>` (held keys)
//! and applies a delta each frame using `time.delta_secs()` for frame-rate
//! independence.
//!
//! # Example
//! ```no_run
//! use bevy::prelude::*;
//! use movable_sprite::MovableSpritePlugin;
//!
//! App::new()
//!     .add_plugins(DefaultPlugins)
//!     .add_plugins(MovableSpritePlugin)
//!     .run();
//! ```

use bevy::prelude::*;

/// Bundles the setup and movement systems for a WASD-movable sprite.
///
/// Add it with `app.add_plugins(MovableSpritePlugin)`. It does **not** add
/// `DefaultPlugins`, so the host app stays in control of window and rendering
/// setup.
pub struct MovableSpritePlugin;

impl Plugin for MovableSpritePlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<MovableSpriteConfig>()
            .add_systems(Startup, setup)
            .add_systems(Update, player_movement_system);
    }
}

/// Tunable parameters for the movable sprite.
#[derive(Resource, Clone, Copy, Debug)]
pub struct MovableSpriteConfig {
    /// Top-speed of the player sprite in world units per second.
    pub speed: f32,
}

impl Default for MovableSpriteConfig {
    fn default() -> Self {
        Self { speed: 300.0 }
    }
}

/// Tags the player entity.
#[derive(Component)]
pub struct Player;

/// Returns a normalized movement direction from four boolean key states.
///
/// Opposing keys cancel; when no key is pressed returns `Vec2::ZERO`.
pub fn input_dir(up: bool, down: bool, left: bool, right: bool) -> Vec2 {
    let raw = Vec2::new(right as i8 as f32 - left as i8 as f32, up as i8 as f32 - down as i8 as f32);
    if raw != Vec2::ZERO { raw.normalize() } else { Vec2::ZERO }
}

/// Returns the per-frame translation delta given a unit direction, speed, and timestep.
pub fn frame_delta(dir: Vec2, speed: f32, dt: f32) -> Vec3 {
    Vec3::new(dir.x * speed * dt, dir.y * speed * dt, 0.0)
}

/// Spawns a camera and the player sprite from disk.
fn setup(mut commands: Commands, asset_server: Res<AssetServer>) {
    commands.spawn(Camera2d);
    commands.spawn((
        Sprite {
            image: asset_server.load("sprite.png"),
            ..default()
        },
        Player,
    ));
}

/// Reads WASD input and moves the player at [`MovableSpriteConfig::speed`].
fn player_movement_system(
    time: Res<Time>,
    keyboard_input: Res<ButtonInput<KeyCode>>,
    config: Res<MovableSpriteConfig>,
    mut player_query: Query<&mut Transform, With<Player>>,
) {
    for mut player_transform in player_query.iter_mut() {
        let dir = input_dir(
            keyboard_input.pressed(KeyCode::KeyW),
            keyboard_input.pressed(KeyCode::KeyS),
            keyboard_input.pressed(KeyCode::KeyA),
            keyboard_input.pressed(KeyCode::KeyD),
        );
        if dir != Vec2::ZERO {
            player_transform.translation += frame_delta(dir, config.speed, time.delta_secs());
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn config_default_speed_is_positive() {
        assert!(MovableSpriteConfig::default().speed > 0.0);
    }

    #[test]
    fn input_dir_no_keys_is_zero() {
        assert_eq!(input_dir(false, false, false, false), Vec2::ZERO);
    }

    #[test]
    fn input_dir_up_only_points_north() {
        let d = input_dir(true, false, false, false);
        assert!((d.y - 1.0).abs() < 1e-5);
        assert_eq!(d.x, 0.0);
    }

    #[test]
    fn input_dir_diagonal_is_unit_length() {
        let d = input_dir(true, false, false, true); // up + right
        assert!((d.length() - 1.0).abs() < 1e-5);
        assert!(d.x > 0.0 && d.y > 0.0);
    }

    #[test]
    fn input_dir_opposing_keys_cancel() {
        assert_eq!(input_dir(true, true, false, false), Vec2::ZERO);
        assert_eq!(input_dir(false, false, true, true), Vec2::ZERO);
    }

    #[test]
    fn frame_delta_scales_with_speed_and_dt() {
        let delta = frame_delta(Vec2::X, 300.0, 1.0);
        assert!((delta.x - 300.0).abs() < 1e-4);
        assert_eq!(delta.y, 0.0);
        assert_eq!(delta.z, 0.0);
    }
}
