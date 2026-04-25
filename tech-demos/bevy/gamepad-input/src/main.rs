//! Gamepad-input demo — analog sticks, digital buttons, and keyboard fallback.
//!
//! Key ideas:
//! - Bevy 0.14+ exposes gamepads as ECS entities; `Query<&Gamepad>` iterates
//!   all connected pads.
//! - `gamepad.pressed(GamepadButton::South)` checks digital buttons.
//! - `gamepad.get(GamepadAxis::LeftStickX).unwrap_or(0.0)` reads analog axes.
//! - A dead-zone magnitude threshold prevents stick drift from moving the
//!   sprite when the stick is near center.
//! - WASD / arrow keys provide an identical keyboard fallback so the demo is
//!   usable without a controller.
//!
//! **Controls (gamepad):** left stick to move; South/East/North/West to light
//! up the button HUD.
//! **Controls (keyboard):** WASD or arrow keys to move.

use bevy::prelude::*;

/// Pixels per second for full-axis deflection.
const MOVE_SPEED: f32 = 220.0;
/// Analog stick dead zone (magnitude).
const DEAD_ZONE: f32 = 0.15;

fn main() {
    App::new()
        .add_plugins(DefaultPlugins.set(WindowPlugin {
            primary_window: Some(Window {
                title: "Gamepad Input Demo".to_string(),
                resolution: (800.0, 500.0).into(),
                ..default()
            }),
            ..default()
        }))
        .add_systems(Startup, setup)
        .add_systems(Update, (move_player, update_button_hud))
        .run();
}

/// Marker for the movable player square.
#[derive(Component)]
struct Player;

/// Marks each button-state label.
#[derive(Component)]
struct ButtonLabel(GamepadButton);

/// Marks the connection-status label.
#[derive(Component)]
struct ConnectionLabel;

/// Spawns the player sprite and the button HUD.
fn setup(mut commands: Commands) {
    commands.spawn(Camera2d);

    commands.spawn((
        Sprite { color: Color::srgb(0.3, 0.7, 1.0), custom_size: Some(Vec2::splat(40.0)), ..default() },
        Transform::default(),
        Player,
    ));

    // Button HUD — four face buttons.
    let buttons = [
        (GamepadButton::South,  "South (A/X)",   Vec2::new(100.0, -120.0)),
        (GamepadButton::East,   "East  (B/○)",   Vec2::new(220.0, -120.0)),
        (GamepadButton::West,   "West  (X/□)",   Vec2::new(100.0, -150.0)),
        (GamepadButton::North,  "North (Y/△)",   Vec2::new(220.0, -150.0)),
    ];
    for (button, label_str, pos) in buttons {
        commands.spawn((
            Text::new(label_str),
            TextFont { font_size: 18.0, ..default() },
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
        Text::new("No gamepad connected — using keyboard (WASD / arrows)"),
        TextFont { font_size: 20.0, ..default() },
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

/// Reads the first connected gamepad (or keyboard) and moves the player.
fn move_player(
    time: Res<Time>,
    keyboard: Res<ButtonInput<KeyCode>>,
    gamepads: Query<&Gamepad>,
    mut player_query: Query<&mut Transform, With<Player>>,
    mut conn_query: Query<&mut Text, With<ConnectionLabel>>,
) {
    let Ok(mut transform) = player_query.get_single_mut() else { return };
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
        if raw.length() > DEAD_ZONE {
            axis = raw;
        }
    }

    // Keyboard fallback (also merged when no gamepad).
    if !has_gamepad || axis == Vec2::ZERO {
        let left  = keyboard.pressed(KeyCode::KeyA) || keyboard.pressed(KeyCode::ArrowLeft);
        let right = keyboard.pressed(KeyCode::KeyD) || keyboard.pressed(KeyCode::ArrowRight);
        let up    = keyboard.pressed(KeyCode::KeyW) || keyboard.pressed(KeyCode::ArrowUp);
        let down  = keyboard.pressed(KeyCode::KeyS) || keyboard.pressed(KeyCode::ArrowDown);
        axis = Vec2::new(right as i8 as f32 - left as i8 as f32, up as i8 as f32 - down as i8 as f32);
        if axis.length() > 1.0 {
            axis = axis.normalize();
        }
    }

    transform.translation.x = (transform.translation.x + axis.x * MOVE_SPEED * dt).clamp(-380.0, 380.0);
    transform.translation.y = (transform.translation.y + axis.y * MOVE_SPEED * dt).clamp(-230.0, 230.0);

    // Update connection label.
    if let Ok(mut text) = conn_query.get_single_mut() {
        if has_gamepad {
            text.0 = "Gamepad connected — left stick to move".to_string();
        } else {
            text.0 = "No gamepad connected — using keyboard (WASD / arrows)".to_string();
        }
    }
}

/// Highlights button labels when their corresponding button is held.
fn update_button_hud(
    gamepads: Query<&Gamepad>,
    mut labels: Query<(&mut TextColor, &ButtonLabel)>,
) {
    let gamepad = gamepads.iter().next();
    for (mut color, btn_label) in &mut labels {
        let pressed = gamepad.map_or(false, |gp| gp.pressed(btn_label.0));
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
    fn move_speed_is_positive() {
        assert!(MOVE_SPEED > 0.0);
    }

    #[test]
    fn dead_zone_is_within_unit_interval() {
        assert!(DEAD_ZONE > 0.0 && DEAD_ZONE < 1.0);
    }

    #[test]
    fn keyboard_axis_normalised_on_diagonal() {
        // Simulate diagonal press.
        let raw = Vec2::new(1.0, 1.0);
        let normalised = if raw.length() > 1.0 { raw.normalize() } else { raw };
        assert!((normalised.length() - 1.0).abs() < 1e-5);
    }

    #[test]
    fn dead_zone_filters_small_input() {
        let raw = Vec2::new(DEAD_ZONE * 0.5, 0.0);
        let effective = if raw.length() > DEAD_ZONE { raw } else { Vec2::ZERO };
        assert_eq!(effective, Vec2::ZERO);
    }

    #[test]
    fn above_dead_zone_passes_through() {
        let raw = Vec2::new(DEAD_ZONE + 0.05, 0.0);
        let effective = if raw.length() > DEAD_ZONE { raw } else { Vec2::ZERO };
        assert_ne!(effective, Vec2::ZERO);
    }
}
