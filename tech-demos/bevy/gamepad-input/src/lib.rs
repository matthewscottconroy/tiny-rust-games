//! Gamepad input — a reusable Bevy plugin.
//!
//! This crate is a *building block*: drop [`GamepadInputPlugin`] into any Bevy
//! app with `app.add_plugins(GamepadInputPlugin)` and it spawns a square the
//! player drives with an analog stick (or WASD / arrow-key fallback), plus a
//! HUD that lights up the four face buttons. Tune it through the
//! [`GamepadInputConfig`] resource without editing the plugin's internals.
//!
//! Key ideas:
//! - Bevy 0.14+ exposes gamepads as ECS entities; `Query<&Gamepad>` iterates
//!   all connected pads.
//! - `gamepad.pressed(GamepadButton::South)` checks digital buttons.
//! - `gamepad.get(GamepadAxis::LeftStickX).unwrap_or(0.0)` reads analog axes.
//! - A dead-zone magnitude threshold ([`apply_deadzone`]) prevents stick drift
//!   from moving the sprite when the stick is near center.
//! - WASD / arrow keys provide an identical keyboard fallback ([`keyboard_axis`])
//!   so the demo is usable without a controller.
//!
//! **Controls (gamepad):** left stick to move; South/East/North/West to light
//! up the button HUD.
//! **Controls (keyboard):** WASD or arrow keys to move.
//!
//! # Example
//! ```no_run
//! use bevy::prelude::*;
//! use gamepad_input::GamepadInputPlugin;
//!
//! App::new()
//!     .add_plugins(DefaultPlugins)
//!     .add_plugins(GamepadInputPlugin)
//!     .run();
//! ```

use bevy::prelude::*;

/// Bundles every system and resource for the gamepad-input feature.
///
/// Add it with `app.add_plugins(GamepadInputPlugin)`. It does **not** add
/// `DefaultPlugins`, so the host app stays in control of window and rendering
/// setup.
pub struct GamepadInputPlugin;

impl Plugin for GamepadInputPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<GamepadInputConfig>()
            .add_systems(Startup, setup)
            .add_systems(Update, (move_player, update_button_hud));
    }
}

// --- Configuration ---

/// Tunable parameters for the feature. Override before adding the plugin, e.g.
/// `app.insert_resource(GamepadInputConfig { move_speed: 400.0, ..default() })`.
#[derive(Resource, Clone, Copy, Debug)]
pub struct GamepadInputConfig {
    /// Pixels per second for full-axis deflection.
    pub move_speed: f32,
    /// Analog stick dead zone (magnitude).
    pub dead_zone: f32,
    /// Horizontal clamp for the player's position (± this many pixels).
    pub half_width: f32,
    /// Vertical clamp for the player's position (± this many pixels).
    pub half_height: f32,
}

impl Default for GamepadInputConfig {
    fn default() -> Self {
        Self {
            move_speed: 220.0,
            dead_zone: 0.15,
            half_width: 380.0,
            half_height: 230.0,
        }
    }
}

// --- Components ---

/// Marker for the movable player square.
#[derive(Component)]
pub struct Player;

/// Marks each button-state label with the gamepad button it tracks.
#[derive(Component)]
pub struct ButtonLabel(pub GamepadButton);

/// Marks the connection-status label.
#[derive(Component)]
struct ConnectionLabel;

// --- Pure logic ---

/// Applies a radial dead zone: returns [`Vec2::ZERO`] when `raw`'s magnitude is
/// at or below `dead_zone`, otherwise passes `raw` through unchanged.
pub fn apply_deadzone(raw: Vec2, dead_zone: f32) -> Vec2 {
    if raw.length() > dead_zone {
        raw
    } else {
        Vec2::ZERO
    }
}

/// Builds a movement axis from digital directional input, normalizing diagonals
/// so they are not faster than cardinal movement.
pub fn keyboard_axis(left: bool, right: bool, up: bool, down: bool) -> Vec2 {
    let axis = Vec2::new(
        right as i8 as f32 - left as i8 as f32,
        up as i8 as f32 - down as i8 as f32,
    );
    if axis.length() > 1.0 {
        axis.normalize()
    } else {
        axis
    }
}

// --- Setup ---

/// Spawns the player sprite and the button HUD.
fn setup(mut commands: Commands) {
    commands.spawn(Camera2d);

    commands.spawn((
        Sprite {
            color: Color::srgb(0.3, 0.7, 1.0),
            custom_size: Some(Vec2::splat(40.0)),
            ..default()
        },
        Transform::default(),
        Player,
    ));

    // Button HUD — four face buttons.
    let buttons = [
        // Xbox letter / PlayStation shape. The shapes are spelled out because
        // Bevy's default font is ASCII-only and the glyphs render as tofu.
        (
            GamepadButton::South,
            "South (A/Cross)",
            Vec2::new(100.0, -120.0),
        ),
        (
            GamepadButton::East,
            "East  (B/Circle)",
            Vec2::new(220.0, -120.0),
        ),
        (
            GamepadButton::West,
            "West  (X/Square)",
            Vec2::new(100.0, -150.0),
        ),
        (
            GamepadButton::North,
            "North (Y/Triangle)",
            Vec2::new(220.0, -150.0),
        ),
    ];
    for (button, label_str, pos) in buttons {
        commands.spawn((
            Text::new(label_str),
            TextFont {
                font_size: 18.0,
                ..default()
            },
            TextColor(Color::srgba(0.6, 0.6, 0.6, 1.0)),
            Node {
                position_type: PositionType::Absolute,
                left: Val::Px(pos.x),
                top: Val::Px(-pos.y + 300.0),
                ..default()
            },
            ButtonLabel(button),
        ));
    }

    commands.spawn((
        Text::new("No gamepad connected - using keyboard (WASD / arrows)"),
        TextFont {
            font_size: 20.0,
            ..default()
        },
        TextColor(Color::srgb(1.0, 0.8, 0.3)),
        Node {
            position_type: PositionType::Absolute,
            top: Val::Px(12.0),
            left: Val::Px(12.0),
            ..default()
        },
        ConnectionLabel,
    ));
}

// --- Systems ---

/// Reads the first connected gamepad (or keyboard) and moves the player.
fn move_player(
    time: Res<Time>,
    config: Res<GamepadInputConfig>,
    keyboard: Res<ButtonInput<KeyCode>>,
    gamepads: Query<&Gamepad>,
    mut player_query: Query<&mut Transform, With<Player>>,
    mut conn_query: Query<&mut Text, With<ConnectionLabel>>,
) {
    let Ok(mut transform) = player_query.single_mut() else {
        return;
    };
    let dt = time.delta_secs();

    let mut axis = Vec2::ZERO;
    let mut has_gamepad = false;

    // Use the first connected gamepad, if any.
    if let Some(gamepad) = gamepads.iter().next() {
        has_gamepad = true;
        let raw = Vec2::new(
            gamepad.get(GamepadAxis::LeftStickX).unwrap_or(0.0),
            gamepad.get(GamepadAxis::LeftStickY).unwrap_or(0.0),
        );
        axis = apply_deadzone(raw, config.dead_zone);
    }

    // Keyboard fallback (also merged when no gamepad).
    if !has_gamepad || axis == Vec2::ZERO {
        let left = keyboard.pressed(KeyCode::KeyA) || keyboard.pressed(KeyCode::ArrowLeft);
        let right = keyboard.pressed(KeyCode::KeyD) || keyboard.pressed(KeyCode::ArrowRight);
        let up = keyboard.pressed(KeyCode::KeyW) || keyboard.pressed(KeyCode::ArrowUp);
        let down = keyboard.pressed(KeyCode::KeyS) || keyboard.pressed(KeyCode::ArrowDown);
        axis = keyboard_axis(left, right, up, down);
    }

    transform.translation.x = (transform.translation.x + axis.x * config.move_speed * dt)
        .clamp(-config.half_width, config.half_width);
    transform.translation.y = (transform.translation.y + axis.y * config.move_speed * dt)
        .clamp(-config.half_height, config.half_height);

    // Update connection label.
    if let Ok(mut text) = conn_query.single_mut() {
        if has_gamepad {
            text.0 = "Gamepad connected - left stick to move".to_string();
        } else {
            text.0 = "No gamepad connected - using keyboard (WASD / arrows)".to_string();
        }
    }
}

/// Highlights button labels when their corresponding button is held.
fn update_button_hud(gamepads: Query<&Gamepad>, mut labels: Query<(&mut TextColor, &ButtonLabel)>) {
    let gamepad = gamepads.iter().next();
    for (mut color, btn_label) in &mut labels {
        let pressed = gamepad.is_some_and(|gp| gp.pressed(btn_label.0));
        color.0 = if pressed {
            Color::srgb(1.0, 0.9, 0.2)
        } else {
            Color::srgba(0.6, 0.6, 0.6, 1.0)
        };
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn config_default_matches_documented_values() {
        let c = GamepadInputConfig::default();
        assert_eq!(c.move_speed, 220.0);
        assert_eq!(c.dead_zone, 0.15);
    }

    #[test]
    fn move_speed_is_positive() {
        assert!(GamepadInputConfig::default().move_speed > 0.0);
    }

    #[test]
    fn dead_zone_is_within_unit_interval() {
        let dz = GamepadInputConfig::default().dead_zone;
        assert!(dz > 0.0 && dz < 1.0);
    }

    #[test]
    fn keyboard_axis_normalised_on_diagonal() {
        let axis = keyboard_axis(false, true, true, false);
        assert!((axis.length() - 1.0).abs() < 1e-5);
    }

    #[test]
    fn keyboard_axis_cardinal_is_unit() {
        let axis = keyboard_axis(false, true, false, false);
        assert_eq!(axis, Vec2::new(1.0, 0.0));
    }

    #[test]
    fn keyboard_axis_opposing_cancels() {
        let axis = keyboard_axis(true, true, true, true);
        assert_eq!(axis, Vec2::ZERO);
    }

    #[test]
    fn dead_zone_filters_small_input() {
        let dz = GamepadInputConfig::default().dead_zone;
        let effective = apply_deadzone(Vec2::new(dz * 0.5, 0.0), dz);
        assert_eq!(effective, Vec2::ZERO);
    }

    #[test]
    fn above_dead_zone_passes_through() {
        let dz = GamepadInputConfig::default().dead_zone;
        let effective = apply_deadzone(Vec2::new(dz + 0.05, 0.0), dz);
        assert_ne!(effective, Vec2::ZERO);
    }

    // --- ECS setup tests ---

    #[test]
    fn setup_spawns_one_player() {
        let mut app = App::new();
        app.add_plugins(MinimalPlugins)
            .init_resource::<GamepadInputConfig>()
            .add_systems(Startup, setup);
        app.update();

        let mut q = app.world_mut().query::<&Player>();
        assert_eq!(q.iter(app.world()).count(), 1);
    }

    #[test]
    fn plugin_spawns_four_button_labels() {
        // Demonstrates the building-block path: the plugin composes onto a
        // headless app with minimal wiring.
        let mut app = App::new();
        app.add_plugins((MinimalPlugins, GamepadInputPlugin));
        // The plugin's Update systems read keyboard input; MinimalPlugins omits it.
        app.insert_resource(ButtonInput::<KeyCode>::default());
        app.update();

        let mut q = app.world_mut().query::<&ButtonLabel>();
        assert_eq!(q.iter(app.world()).count(), 4);
    }
}
