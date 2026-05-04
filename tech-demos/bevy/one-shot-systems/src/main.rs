//! # One-Shot Systems Demo
//!
//! Demonstrates Bevy's on-demand "one-shot systems" — systems that are registered
//! once at startup and then executed explicitly via `Commands::run_system` whenever
//! needed, rather than running every frame.
//!
//! This pattern is ideal for:
//! - Button callbacks that modify game state
//! - Inventory actions, crafting, ability activations
//! - Cutscene triggers and scripted events
//!
//! **What to try**: click the three coloured buttons on screen to heal the player,
//! deal damage, or restore mana. The HUD reflects changes immediately.
//!
//! **Controls**: Left-click a button sprite to run its one-shot system.

use bevy::ecs::system::SystemId;
use bevy::prelude::*;

// ---------------------------------------------------------------------------
// Constants
// ---------------------------------------------------------------------------

const HP_MAX: i32 = 100;
const MP_MAX: i32 = 60;
const BUTTON_W: f32 = 140.0;
const BUTTON_H: f32 = 50.0;

// ---------------------------------------------------------------------------
// Pure helper functions
// ---------------------------------------------------------------------------

/// Clamps `value` to `[min, max]`.
pub fn clamp_stat(value: i32, min: i32, max: i32) -> i32 {
    value.max(min).min(max)
}

/// Returns `true` if `cursor` lies inside the axis-aligned box centred at
/// `center` with half-extents `half_size`.
pub fn cursor_in_box(cursor: Vec2, center: Vec2, half_size: Vec2) -> bool {
    (cursor.x - center.x).abs() <= half_size.x && (cursor.y - center.y).abs() <= half_size.y
}

// ---------------------------------------------------------------------------
// Components
// ---------------------------------------------------------------------------

/// Player stats.
#[derive(Component)]
struct Health(i32);

/// Player mana.
#[derive(Component)]
struct Mana(i32);

/// Marks the player entity.
#[derive(Component)]
struct Player;

/// Identifies which action a button performs when clicked.
#[derive(Component, Clone, Copy, PartialEq)]
enum ButtonAction {
    Heal,
    Damage,
    RestoreMana,
}

/// Marker for the HUD text entity.
#[derive(Component)]
struct HudText;

// ---------------------------------------------------------------------------
// Resource — registered one-shot system IDs
// ---------------------------------------------------------------------------

/// Holds the [`SystemId`] handles returned by `world.register_system`.
#[derive(Resource)]
struct ActionSystems {
    /// System that adds 20 HP to the player.
    heal: SystemId,
    /// System that subtracts 15 HP from the player.
    damage: SystemId,
    /// System that adds 30 MP to the player.
    restore_mana: SystemId,
}

// ---------------------------------------------------------------------------
// One-shot system implementations
// ---------------------------------------------------------------------------

/// Adds 20 HP to the player (clamped to `HP_MAX`).
fn heal_player(mut query: Query<&mut Health, With<Player>>) {
    let Ok(mut hp) = query.single_mut() else {
        return;
    };
    hp.0 = clamp_stat(hp.0 + 20, 0, HP_MAX);
}

/// Subtracts 15 HP from the player (clamped to 0).
fn damage_player(mut query: Query<&mut Health, With<Player>>) {
    let Ok(mut hp) = query.single_mut() else {
        return;
    };
    hp.0 = clamp_stat(hp.0 - 15, 0, HP_MAX);
}

/// Adds 30 MP to the player (clamped to `MP_MAX`).
fn restore_mana_player(mut query: Query<&mut Mana, With<Player>>) {
    let Ok(mut mp) = query.single_mut() else {
        return;
    };
    mp.0 = clamp_stat(mp.0 + 30, 0, MP_MAX);
}

// ---------------------------------------------------------------------------
// Startup: register one-shot systems
// ---------------------------------------------------------------------------

/// Exclusive startup system — registers all one-shot systems and stores their IDs.
fn register_actions(world: &mut World) {
    let heal = world.register_system(heal_player);
    let damage = world.register_system(damage_player);
    let restore_mana = world.register_system(restore_mana_player);
    world.insert_resource(ActionSystems {
        heal,
        damage,
        restore_mana,
    });
}

// ---------------------------------------------------------------------------
// Setup scene
// ---------------------------------------------------------------------------

fn setup(mut commands: Commands) {
    commands.spawn(Camera2d);

    // Player entity (invisible; holds stats only)
    commands.spawn((Player, Health(HP_MAX / 2), Mana(MP_MAX / 2)));

    // Buttons
    let buttons = [
        (ButtonAction::Heal, "Heal +20 HP", Color::srgb(0.2, 0.7, 0.3), -200.0_f32),
        (ButtonAction::Damage, "Damage −15 HP", Color::srgb(0.8, 0.2, 0.2), 0.0_f32),
        (ButtonAction::RestoreMana, "Mana +30 MP", Color::srgb(0.2, 0.3, 0.8), 200.0_f32),
    ];

    for (action, label, color, x) in buttons {
        commands.spawn((
            Sprite {
                color,
                custom_size: Some(Vec2::new(BUTTON_W, BUTTON_H)),
                ..default()
            },
            Transform::from_xyz(x, -80.0, 0.0),
            action,
        ));

        // Label below button
        commands.spawn((
            Text::new(label),
            TextFont {
                font_size: 14.0,
                ..default()
            },
            TextColor(Color::WHITE),
            Node {
                position_type: PositionType::Absolute,
                top: Val::Px(290.0),
                left: Val::Px(x + 350.0 - 60.0),
                ..default()
            },
        ));
    }

    // HUD
    commands.spawn((
        Text::new("HP: 50 / 100    MP: 30 / 60"),
        TextFont {
            font_size: 20.0,
            ..default()
        },
        TextColor(Color::WHITE),
        HudText,
        Node {
            position_type: PositionType::Absolute,
            top: Val::Px(8.0),
            left: Val::Px(8.0),
            ..default()
        },
    ));

    // Instructions
    commands.spawn((
        Text::new("Click a button to run its one-shot system"),
        TextFont {
            font_size: 15.0,
            ..default()
        },
        TextColor(Color::srgba(0.8, 0.8, 0.8, 1.0)),
        Node {
            position_type: PositionType::Absolute,
            top: Val::Px(36.0),
            left: Val::Px(8.0),
            ..default()
        },
    ));
}

// ---------------------------------------------------------------------------
// Click handling
// ---------------------------------------------------------------------------

/// Checks whether the mouse cursor (in world space) overlaps any button and,
/// if so, runs the corresponding one-shot system.
fn handle_clicks(
    mouse: Res<ButtonInput<MouseButton>>,
    window: Query<&Window>,
    camera_q: Query<(&Camera, &GlobalTransform)>,
    buttons: Query<(&ButtonAction, &Transform)>,
    ids: Res<ActionSystems>,
    mut commands: Commands,
) {
    if !mouse.just_pressed(MouseButton::Left) {
        return;
    }

    let Ok(window) = window.single() else {
        return;
    };
    let Ok((camera, cam_transform)) = camera_q.single() else {
        return;
    };

    let cursor_screen = match window.cursor_position() {
        Some(p) => p,
        None => return,
    };
    let Ok(cursor_world) = camera.viewport_to_world_2d(cam_transform, cursor_screen) else {
        return;
    };

    let half = Vec2::new(BUTTON_W * 0.5, BUTTON_H * 0.5);

    for (action, transform) in &buttons {
        let center = transform.translation.truncate();
        if cursor_in_box(cursor_world, center, half) {
            let id = match action {
                ButtonAction::Heal => ids.heal,
                ButtonAction::Damage => ids.damage,
                ButtonAction::RestoreMana => ids.restore_mana,
            };
            commands.run_system(id);
            break;
        }
    }
}

// ---------------------------------------------------------------------------
// HUD update
// ---------------------------------------------------------------------------

/// Reads current player stats and refreshes the HUD text each frame.
fn update_hud(
    player: Query<(&Health, &Mana), With<Player>>,
    mut hud: Query<&mut Text, With<HudText>>,
) {
    let Ok((hp, mp)) = player.single() else {
        return;
    };
    let Ok(mut text) = hud.single_mut() else {
        return;
    };
    **text = format!("HP: {} / {}    MP: {} / {}", hp.0, HP_MAX, mp.0, MP_MAX);
}

// ---------------------------------------------------------------------------
// App entry point
// ---------------------------------------------------------------------------

fn main() {
    App::new()
        .add_plugins(DefaultPlugins.set(WindowPlugin {
            primary_window: Some(Window {
                title: "One-Shot Systems — Click Buttons".into(),
                resolution: (700u32, 450u32).into(),
                ..default()
            }),
            ..default()
        }))
        .add_systems(Startup, (register_actions, setup))
        .add_systems(Update, (handle_clicks, update_hud).chain())
        .run();
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    /// clamp_stat keeps value within bounds.
    #[test]
    fn clamp_stat_within_bounds() {
        assert_eq!(clamp_stat(50, 0, 100), 50);
    }

    /// clamp_stat clamps to min when below.
    #[test]
    fn clamp_stat_below_min() {
        assert_eq!(clamp_stat(-5, 0, 100), 0);
    }

    /// clamp_stat clamps to max when above.
    #[test]
    fn clamp_stat_above_max() {
        assert_eq!(clamp_stat(120, 0, 100), 100);
    }

    /// clamp_stat returns min when value equals min.
    #[test]
    fn clamp_stat_at_min() {
        assert_eq!(clamp_stat(0, 0, 100), 0);
    }

    /// clamp_stat returns max when value equals max.
    #[test]
    fn clamp_stat_at_max() {
        assert_eq!(clamp_stat(100, 0, 100), 100);
    }

    /// cursor_in_box returns true when cursor is exactly at centre.
    #[test]
    fn cursor_in_box_at_center() {
        assert!(cursor_in_box(
            Vec2::new(0.0, 0.0),
            Vec2::new(0.0, 0.0),
            Vec2::new(70.0, 25.0),
        ));
    }

    /// cursor_in_box returns true on the boundary edge.
    #[test]
    fn cursor_in_box_on_edge() {
        assert!(cursor_in_box(
            Vec2::new(70.0, 0.0),
            Vec2::new(0.0, 0.0),
            Vec2::new(70.0, 25.0),
        ));
    }

    /// cursor_in_box returns false when just outside the boundary.
    #[test]
    fn cursor_in_box_just_outside() {
        assert!(!cursor_in_box(
            Vec2::new(71.0, 0.0),
            Vec2::new(0.0, 0.0),
            Vec2::new(70.0, 25.0),
        ));
    }

    /// cursor_in_box works for a non-origin center.
    #[test]
    fn cursor_in_box_offset_center() {
        assert!(cursor_in_box(
            Vec2::new(200.0, -80.0),
            Vec2::new(200.0, -80.0),
            Vec2::new(70.0, 25.0),
        ));
    }

    /// Heal simulation: clamping prevents HP from exceeding max.
    #[test]
    fn heal_clamps_at_max() {
        let hp = clamp_stat(HP_MAX - 5 + 20, 0, HP_MAX);
        assert_eq!(hp, HP_MAX);
    }

    /// Damage simulation: clamping prevents HP from going below zero.
    #[test]
    fn damage_clamps_at_zero() {
        let hp = clamp_stat(10 - 15, 0, HP_MAX);
        assert_eq!(hp, 0);
    }
}
