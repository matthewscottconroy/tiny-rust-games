//! Time-scale — a reusable Bevy plugin demonstrating the virtual-time API.
//!
//! This crate is a *building block*: drop [`TimeScalePlugin`] into any Bevy app
//! with `app.add_plugins(TimeScalePlugin)` and it spawns orbiting sprites whose
//! speed follows `Time<Virtual>`, plus a HUD that reflects the current scale.
//! Tune the selectable speeds through the [`TimeScaleConfig`] resource without
//! editing the plugin's internals.
//!
//! Key ideas:
//! - `Time<Virtual>` is the game clock that drives `time.delta_secs()`.
//!   `set_relative_speed(f)` multiplies every delta by `f`, instantly
//!   making all virtual-time–driven systems run faster or slower.
//! - `pause()` / `unpause()` freeze the virtual clock without affecting
//!   `Time<Real>` (wall time), so the window still responds to input.
//! - Orbiting sprites use the virtual clock so they obey the scale factor;
//!   the HUD uses a fixed string update rather than `Time<Virtual>` so the
//!   label always reflects the true current speed.
//!
//! **Controls:**
//! - **1** — 0.25× (quarter speed)
//! - **2** — 0.5× (half speed)
//! - **3** — 1.0× (normal speed)
//! - **4** — 2.0× (double speed)
//! - **SPACE** — toggle pause / resume
//!
//! # Example
//! ```no_run
//! use bevy::prelude::*;
//! use time_scale::TimeScalePlugin;
//!
//! App::new()
//!     .add_plugins(DefaultPlugins)
//!     .add_plugins(TimeScalePlugin)
//!     .run();
//! ```

use bevy::prelude::*;

/// Bundles every system and resource for the time-scale feature.
///
/// Add it with `app.add_plugins(TimeScalePlugin)`. It does **not** add
/// `DefaultPlugins`, so the host app stays in control of window and rendering
/// setup.
pub struct TimeScalePlugin;

impl Plugin for TimeScalePlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<TimeScaleConfig>()
            .add_systems(Startup, setup)
            .add_systems(Update, (handle_input, orbit_system, update_hud));
    }
}

// --- Configuration ---

/// Tunable parameters for the feature. Override before adding the plugin, e.g.
/// `app.insert_resource(TimeScaleConfig { speeds: [0.1, 1.0, 2.0, 4.0] })`.
#[derive(Resource, Clone, Copy, Debug)]
pub struct TimeScaleConfig {
    /// Speed presets selected by keys 1–4.
    pub speeds: [f32; 4],
}

impl Default for TimeScaleConfig {
    fn default() -> Self {
        Self {
            speeds: [0.25, 0.5, 1.0, 2.0],
        }
    }
}

/// Drives an orbiting sprite around the origin.
#[derive(Component)]
pub struct Orbiter {
    /// Distance from the origin in pixels.
    pub radius: f32,
    /// Current angle in radians.
    pub angle: f32,
    /// Angular velocity in radians per second at normal speed.
    pub angular_speed: f32,
}

/// Marks the text entity that shows the current time-scale.
#[derive(Component)]
pub struct SpeedLabel;

/// Spawns the camera, orbiting sprites, and the HUD.
fn setup(mut commands: Commands) {
    commands.spawn(Camera2d);

    // Three orbiters at different radii, speeds, and colors.
    let configs = [
        (80.0, 1.2, Color::srgb(0.9, 0.3, 0.3)),
        (150.0, 0.7, Color::srgb(0.3, 0.8, 0.4)),
        (220.0, 0.4, Color::srgb(0.3, 0.5, 1.0)),
    ];
    for (radius, angular_speed, color) in configs {
        commands.spawn((
            Sprite {
                color,
                custom_size: Some(Vec2::splat(24.0)),
                ..default()
            },
            Transform::from_translation(Vec3::new(radius, 0.0, 0.0)),
            Orbiter {
                radius,
                angle: 0.0,
                angular_speed,
            },
        ));
    }

    // Central reference dot.
    commands.spawn((
        Sprite {
            color: Color::WHITE,
            custom_size: Some(Vec2::splat(10.0)),
            ..default()
        },
        Transform::default(),
    ));

    commands.spawn((
        Text::new("Speed: 1.00×  [running]"),
        TextFont {
            font_size: 22.0,
            ..default()
        },
        TextColor(Color::WHITE),
        Node {
            position_type: PositionType::Absolute,
            top: Val::Px(12.0),
            left: Val::Px(12.0),
            ..default()
        },
        SpeedLabel,
    ));

    commands.spawn((
        Text::new("1 = 0.25×   2 = 0.5×   3 = 1×   4 = 2×   SPACE = pause"),
        TextFont {
            font_size: 18.0,
            ..default()
        },
        TextColor(Color::srgba(1.0, 1.0, 1.0, 0.7)),
        Node {
            position_type: PositionType::Absolute,
            bottom: Val::Px(12.0),
            left: Val::Px(12.0),
            ..default()
        },
    ));
}

/// Sets the virtual time speed from key presses.
fn handle_input(
    input: Res<ButtonInput<KeyCode>>,
    config: Res<TimeScaleConfig>,
    mut virtual_time: ResMut<Time<Virtual>>,
) {
    let speed_keys = [
        (KeyCode::Digit1, config.speeds[0]),
        (KeyCode::Digit2, config.speeds[1]),
        (KeyCode::Digit3, config.speeds[2]),
        (KeyCode::Digit4, config.speeds[3]),
    ];
    for (key, speed) in speed_keys {
        if input.just_pressed(key) {
            virtual_time.set_relative_speed(speed);
        }
    }
    if input.just_pressed(KeyCode::Space) {
        if virtual_time.is_paused() {
            virtual_time.unpause();
        } else {
            virtual_time.pause();
        }
    }
}

/// Advances each orbiter's angle using virtual time and updates its position.
fn orbit_system(time: Res<Time>, mut query: Query<(&mut Transform, &mut Orbiter)>) {
    let dt = time.delta_secs();
    for (mut transform, mut orbiter) in &mut query {
        orbiter.angle += orbiter.angular_speed * dt;
        transform.translation.x = orbiter.radius * orbiter.angle.cos();
        transform.translation.y = orbiter.radius * orbiter.angle.sin();
    }
}

/// Keeps the HUD label in sync with the current virtual time state.
fn update_hud(virtual_time: Res<Time<Virtual>>, mut query: Query<&mut Text, With<SpeedLabel>>) {
    let Ok(mut text) = query.single_mut() else {
        return;
    };
    let status = if virtual_time.is_paused() {
        "paused"
    } else {
        "running"
    };
    text.0 = format!("Speed: {:.2}×  [{}]", virtual_time.relative_speed(), status);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn speeds_array_has_four_entries() {
        assert_eq!(TimeScaleConfig::default().speeds.len(), 4);
    }

    #[test]
    fn speeds_are_positive_and_ascending() {
        let speeds = TimeScaleConfig::default().speeds;
        for speed in speeds {
            assert!(speed > 0.0);
        }
        for i in 1..speeds.len() {
            assert!(speeds[i] > speeds[i - 1]);
        }
    }

    #[test]
    fn orbit_angle_advances_with_time() {
        let mut orbiter = Orbiter {
            radius: 100.0,
            angle: 0.0,
            angular_speed: 1.0,
        };
        let dt = 0.5_f32;
        orbiter.angle += orbiter.angular_speed * dt;
        assert!((orbiter.angle - 0.5).abs() < 1e-5);
    }

    #[test]
    fn orbit_position_on_unit_circle() {
        let radius = 100.0_f32;
        let angle = std::f32::consts::FRAC_PI_2;
        let x = radius * angle.cos();
        let y = radius * angle.sin();
        assert!(x.abs() < 1e-4);
        assert!((y - 100.0).abs() < 1e-4);
    }

    #[test]
    fn setup_spawns_three_orbiters() {
        let mut app = App::new();
        app.add_plugins(MinimalPlugins).add_systems(Startup, setup);
        app.update();

        let mut q = app.world_mut().query::<&Orbiter>();
        assert_eq!(q.iter(app.world()).count(), 3);
    }

    #[test]
    fn setup_spawns_one_speed_label() {
        let mut app = App::new();
        app.add_plugins(MinimalPlugins).add_systems(Startup, setup);
        app.update();

        let mut q = app.world_mut().query::<&SpeedLabel>();
        assert_eq!(q.iter(app.world()).count(), 1);
    }

    #[test]
    fn plugin_spawns_orbiters() {
        // Building-block path: the plugin composes onto a headless app.
        // Time<Virtual> is provided by MinimalPlugins; input is not.
        let mut app = App::new();
        app.add_plugins((MinimalPlugins, TimeScalePlugin));
        app.insert_resource(ButtonInput::<KeyCode>::default());
        app.update();

        let mut q = app.world_mut().query::<&Orbiter>();
        assert_eq!(q.iter(app.world()).count(), 3);
    }
}
