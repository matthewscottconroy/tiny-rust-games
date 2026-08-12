//! One-shot systems — a reusable Bevy plugin.
//!
//! This crate is a *building block*: drop [`OneShotSystemsPlugin`] into any
//! Bevy app with `app.add_plugins(OneShotSystemsPlugin)` and it demonstrates
//! Bevy's on-demand "one-shot systems" — systems registered once at startup and
//! then executed explicitly via `Commands::run_system` whenever needed, rather
//! than running every frame.
//!
//! This pattern is ideal for:
//! - Button callbacks that modify game state
//! - Inventory actions, crafting, ability activations
//! - Cutscene triggers and scripted events
//!
//! Tune it through the [`OneShotSystemsConfig`] resource without editing the
//! plugin's internals.
//!
//! **What to try**: click the three coloured buttons on screen to heal the
//! player, deal damage, or restore mana. The HUD reflects changes immediately.
//!
//! # Example
//! ```no_run
//! use bevy::prelude::*;
//! use one_shot_systems::OneShotSystemsPlugin;
//!
//! App::new()
//!     .add_plugins(DefaultPlugins)
//!     .add_plugins(OneShotSystemsPlugin)
//!     .run();
//! ```

use bevy::ecs::system::SystemId;
use bevy::prelude::*;

/// Bundles every system, resource, and the one-shot-system registration for the
/// feature.
///
/// Add it with `app.add_plugins(OneShotSystemsPlugin)`. It does **not** add
/// `DefaultPlugins`, so the host app stays in control of window and rendering
/// setup.
pub struct OneShotSystemsPlugin;

impl Plugin for OneShotSystemsPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<OneShotSystemsConfig>()
            .add_systems(Startup, (register_actions, setup))
            .add_systems(Update, (handle_clicks, update_hud).chain());
    }
}

// ---------------------------------------------------------------------------
// Configuration
// ---------------------------------------------------------------------------

/// Tunable parameters for the feature. Override before adding the plugin, e.g.
/// `app.insert_resource(OneShotSystemsConfig { heal_amount: 30, ..default() })`.
#[derive(Resource, Clone, Copy, Debug)]
pub struct OneShotSystemsConfig {
    /// Maximum hit points.
    pub hp_max: i32,
    /// Maximum mana points.
    pub mp_max: i32,
    /// Width of each button sprite.
    pub button_w: f32,
    /// Height of each button sprite.
    pub button_h: f32,
    /// Hit points added by the heal action.
    pub heal_amount: i32,
    /// Hit points removed by the damage action.
    pub damage_amount: i32,
    /// Mana points added by the restore-mana action.
    pub restore_mana_amount: i32,
}

impl Default for OneShotSystemsConfig {
    fn default() -> Self {
        Self {
            hp_max: 100,
            mp_max: 60,
            button_w: 140.0,
            button_h: 50.0,
            heal_amount: 20,
            damage_amount: 15,
            restore_mana_amount: 30,
        }
    }
}

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

/// Player health.
#[derive(Component)]
pub struct Health(pub i32);

/// Player mana.
#[derive(Component)]
pub struct Mana(pub i32);

/// Marks the player entity.
#[derive(Component)]
pub struct Player;

/// Identifies which action a button performs when clicked.
#[derive(Component, Clone, Copy, PartialEq)]
pub enum ButtonAction {
    /// Heals the player.
    Heal,
    /// Damages the player.
    Damage,
    /// Restores the player's mana.
    RestoreMana,
}

/// Marker for the HUD text entity.
#[derive(Component)]
pub struct HudText;

// ---------------------------------------------------------------------------
// Resource — registered one-shot system IDs
// ---------------------------------------------------------------------------

/// Holds the [`SystemId`] handles returned by `world.register_system`.
#[derive(Resource)]
pub struct ActionSystems {
    /// System that heals the player.
    pub heal: SystemId,
    /// System that damages the player.
    pub damage: SystemId,
    /// System that restores the player's mana.
    pub restore_mana: SystemId,
}

// ---------------------------------------------------------------------------
// One-shot system implementations
// ---------------------------------------------------------------------------

/// Adds HP to the player (clamped to the configured max).
fn heal_player(mut query: Query<&mut Health, With<Player>>, config: Res<OneShotSystemsConfig>) {
    let Ok(mut hp) = query.single_mut() else {
        return;
    };
    hp.0 = clamp_stat(hp.0 + config.heal_amount, 0, config.hp_max);
}

/// Subtracts HP from the player (clamped to 0).
fn damage_player(mut query: Query<&mut Health, With<Player>>, config: Res<OneShotSystemsConfig>) {
    let Ok(mut hp) = query.single_mut() else {
        return;
    };
    hp.0 = clamp_stat(hp.0 - config.damage_amount, 0, config.hp_max);
}

/// Adds MP to the player (clamped to the configured max).
fn restore_mana_player(
    mut query: Query<&mut Mana, With<Player>>,
    config: Res<OneShotSystemsConfig>,
) {
    let Ok(mut mp) = query.single_mut() else {
        return;
    };
    mp.0 = clamp_stat(mp.0 + config.restore_mana_amount, 0, config.mp_max);
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

fn setup(mut commands: Commands, config: Res<OneShotSystemsConfig>) {
    commands.spawn(Camera2d);

    // Player entity (invisible; holds stats only)
    commands.spawn((Player, Health(config.hp_max / 2), Mana(config.mp_max / 2)));

    // Buttons
    let buttons = [
        (
            ButtonAction::Heal,
            "Heal +20 HP",
            Color::srgb(0.2, 0.7, 0.3),
            -200.0_f32,
        ),
        (
            ButtonAction::Damage,
            "Damage −15 HP",
            Color::srgb(0.8, 0.2, 0.2),
            0.0_f32,
        ),
        (
            ButtonAction::RestoreMana,
            "Mana +30 MP",
            Color::srgb(0.2, 0.3, 0.8),
            200.0_f32,
        ),
    ];

    for (action, label, color, x) in buttons {
        commands.spawn((
            Sprite {
                color,
                custom_size: Some(Vec2::new(config.button_w, config.button_h)),
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
        Text::new(format!(
            "HP: {} / {}    MP: {} / {}",
            config.hp_max / 2,
            config.hp_max,
            config.mp_max / 2,
            config.mp_max
        )),
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
    config: Res<OneShotSystemsConfig>,
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

    let half = Vec2::new(config.button_w * 0.5, config.button_h * 0.5);

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
    config: Res<OneShotSystemsConfig>,
) {
    let Ok((hp, mp)) = player.single() else {
        return;
    };
    let Ok(mut text) = hud.single_mut() else {
        return;
    };
    **text = format!(
        "HP: {} / {}    MP: {} / {}",
        hp.0, config.hp_max, mp.0, config.mp_max
    );
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
        let c = OneShotSystemsConfig::default();
        let hp = clamp_stat(c.hp_max - 5 + c.heal_amount, 0, c.hp_max);
        assert_eq!(hp, c.hp_max);
    }

    /// Damage simulation: clamping prevents HP from going below zero.
    #[test]
    fn damage_clamps_at_zero() {
        let c = OneShotSystemsConfig::default();
        let hp = clamp_stat(10 - c.damage_amount, 0, c.hp_max);
        assert_eq!(hp, 0);
    }

    /// Config default matches the documented values.
    #[test]
    fn config_default_matches_documented_values() {
        let c = OneShotSystemsConfig::default();
        assert_eq!(c.hp_max, 100);
        assert_eq!(c.mp_max, 60);
        assert_eq!(c.heal_amount, 20);
    }

    /// setup spawns exactly one player entity.
    #[test]
    fn setup_spawns_one_player() {
        let mut app = App::new();
        app.add_plugins(MinimalPlugins)
            .init_resource::<OneShotSystemsConfig>()
            .add_systems(Startup, setup);
        app.update();

        let mut q = app.world_mut().query::<&Player>();
        assert_eq!(q.iter(app.world()).count(), 1);
    }

    /// setup spawns three clickable buttons.
    #[test]
    fn setup_spawns_three_buttons() {
        let mut app = App::new();
        app.add_plugins(MinimalPlugins)
            .init_resource::<OneShotSystemsConfig>()
            .add_systems(Startup, setup);
        app.update();

        let mut q = app.world_mut().query::<&ButtonAction>();
        assert_eq!(q.iter(app.world()).count(), 3);
    }

    /// register_actions inserts the ActionSystems resource.
    #[test]
    fn register_actions_inserts_resource() {
        let mut app = App::new();
        app.add_plugins(MinimalPlugins)
            .init_resource::<OneShotSystemsConfig>()
            .add_systems(Startup, register_actions);
        app.update();

        assert!(app.world().get_resource::<ActionSystems>().is_some());
    }
}
