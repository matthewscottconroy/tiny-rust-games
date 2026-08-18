//! Camera-follow — a reusable Bevy plugin.
//!
//! This crate is a *building block*: add [`CameraFollowPlugin`] to any Bevy app
//! with `app.add_plugins(CameraFollowPlugin)` and it spawns a movable player the
//! camera smoothly lerp-follows. Tune it through the [`CameraFollowConfig`]
//! resource without editing the plugin's internals.
//!
//! Key idea: the camera smoothly lerps toward the player each frame.
//! Using disjoint query filters (`Without<>`) avoids borrow conflicts when
//! two systems both need to read or write `Transform` on different entity sets.
//!
//! # Example
//! ```no_run
//! use bevy::prelude::*;
//! use camera_follow::CameraFollowPlugin;
//!
//! App::new()
//!     .add_plugins(DefaultPlugins)
//!     .add_plugins(CameraFollowPlugin)
//!     .run();
//! ```

use bevy::prelude::*;

/// Bundles every system and resource for the camera-follow feature.
///
/// Add it with `app.add_plugins(CameraFollowPlugin)`. It does **not** add
/// `DefaultPlugins`, so the host app stays in control of window and rendering
/// setup.
pub struct CameraFollowPlugin;

impl Plugin for CameraFollowPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<CameraFollowConfig>()
            .add_systems(Startup, setup)
            .add_systems(Update, (move_player, follow_camera));
    }
}

// --- Configuration ---

/// Tunable parameters for the feature. Override before adding the plugin, e.g.
/// `app.insert_resource(CameraFollowConfig { speed: 400.0, ..default() })`.
#[derive(Resource, Clone, Copy, Debug)]
pub struct CameraFollowConfig {
    /// Top-speed of the player in world units per second.
    pub speed: f32,
    /// Fraction of the gap the camera closes each second (higher = snappier).
    pub lerp_speed: f32,
}

impl Default for CameraFollowConfig {
    fn default() -> Self {
        Self {
            speed: 220.0,
            lerp_speed: 6.0,
        }
    }
}

/// Marks the entity the camera should track.
#[derive(Component)]
pub struct Player;

/// Returns the `t` parameter for `Vec3::lerp` that closes the camera gap in one frame.
///
/// Clamped to `[0, 1]` so large timesteps don't overshoot.
pub fn lerp_factor(speed: f32, dt: f32) -> f32 {
    (speed * dt).clamp(0.0, 1.0)
}

/// Spawns a checkerboard background (makes camera motion visible), the player,
/// and an instruction label.
fn setup(mut commands: Commands) {
    commands.spawn(Camera2d);

    for row in -10..=10 {
        for col in -12..=12 {
            let dark = (row + col) % 2 == 0;
            let color = if dark {
                Color::srgb(0.13, 0.13, 0.13)
            } else {
                Color::srgb(0.22, 0.22, 0.22)
            };
            commands.spawn((
                Sprite {
                    color,
                    custom_size: Some(Vec2::splat(64.0)),
                    ..default()
                },
                Transform::from_xyz(col as f32 * 64.0, row as f32 * 64.0, -1.0),
            ));
        }
    }

    commands.spawn((
        Sprite {
            color: Color::srgb(0.25, 0.85, 0.35),
            custom_size: Some(Vec2::splat(32.0)),
            ..default()
        },
        Transform::default(),
        Player,
    ));

    commands.spawn((
        Text::new("WASD - move   (camera lerp-follows)"),
        TextFont {
            font_size: 16.0,
            ..default()
        },
        TextColor(Color::srgb(0.85, 0.85, 0.85)),
        Node {
            position_type: PositionType::Absolute,
            bottom: Val::Px(10.0),
            left: Val::Px(10.0),
            ..default()
        },
    ));
}

/// Reads WASD input and moves the player at [`CameraFollowConfig::speed`] units/s.
fn move_player(
    time: Res<Time>,
    input: Res<ButtonInput<KeyCode>>,
    config: Res<CameraFollowConfig>,
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
        let delta = dir.normalize() * config.speed * time.delta_secs();
        transform.translation.x += delta.x;
        transform.translation.y += delta.y;
    }
}

/// Smoothly moves the camera toward the player using exponential lerp.
///
/// `Without<Camera2d>` and `Without<Player>` make the two queries disjoint,
/// which is required when both touch `Transform`.
fn follow_camera(
    time: Res<Time>,
    config: Res<CameraFollowConfig>,
    player_query: Query<&Transform, (With<Player>, Without<Camera2d>)>,
    mut cam_query: Query<&mut Transform, (With<Camera2d>, Without<Player>)>,
) {
    let Ok(player) = player_query.single() else {
        return;
    };
    let Ok(mut cam) = cam_query.single_mut() else {
        return;
    };

    let target = player.translation;
    cam.translation = cam
        .translation
        .lerp(target, lerp_factor(config.lerp_speed, time.delta_secs()));
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn setup_spawns_one_player() {
        let mut app = App::new();
        app.add_plugins(MinimalPlugins).add_systems(Startup, setup);
        app.update();

        let mut q = app.world_mut().query::<&Player>();
        assert_eq!(q.iter(app.world()).count(), 1);
    }

    #[test]
    fn setup_spawns_one_camera() {
        let mut app = App::new();
        app.add_plugins(MinimalPlugins).add_systems(Startup, setup);
        app.update();

        let mut q = app.world_mut().query::<&Camera2d>();
        assert_eq!(q.iter(app.world()).count(), 1);
    }

    #[test]
    fn player_starts_at_origin() {
        let mut app = App::new();
        app.add_plugins(MinimalPlugins).add_systems(Startup, setup);
        app.update();

        let mut q = app.world_mut().query::<(&Player, &Transform)>();
        for (_, t) in q.iter(app.world()) {
            assert_eq!(t.translation, Vec3::ZERO);
        }
    }

    #[test]
    fn lerp_factor_zero_dt_is_zero() {
        assert_eq!(lerp_factor(6.0, 0.0), 0.0);
    }

    #[test]
    fn lerp_factor_normal_dt_is_proportional() {
        // At 60 fps (dt ≈ 0.0167) with speed 6: factor ≈ 0.1
        let f = lerp_factor(6.0, 1.0 / 60.0);
        assert!((f - 0.1).abs() < 1e-4);
    }

    #[test]
    fn lerp_factor_large_dt_clamped_to_one() {
        // Huge timestep should never overshoot
        assert_eq!(lerp_factor(6.0, 10.0), 1.0);
    }

    #[test]
    fn lerp_factor_scales_with_speed() {
        let slow = lerp_factor(1.0, 0.1);
        let fast = lerp_factor(5.0, 0.1);
        assert!(fast > slow);
    }

    #[test]
    fn config_default_matches_documented_values() {
        let c = CameraFollowConfig::default();
        assert_eq!(c.speed, 220.0);
        assert_eq!(c.lerp_speed, 6.0);
    }
}
