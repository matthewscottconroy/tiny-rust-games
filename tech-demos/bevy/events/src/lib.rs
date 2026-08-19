//! Events (Message API) — a reusable Bevy plugin.
//!
//! This crate is a *building block*: drop [`EventsPlugin`] into any Bevy app
//! with `app.add_plugins(EventsPlugin)` and it wires up a score that reacts to
//! keyboard input purely through messages.
//!
//! Key idea: systems communicate through messages rather than shared mutable
//! state.  Sender systems write messages; receiver systems read them
//! independently.  This keeps systems fully decoupled — the input system does
//! not know who handles the score, and multiple receivers can react to the
//! same message. Tune it through the [`EventsConfig`] resource without editing
//! the plugin's internals.
//!
//! Bevy 0.17+ uses `#[derive(Message)]` / [`MessageWriter`] / [`MessageReader`].
//!
//! # Example
//! ```no_run
//! use bevy::prelude::*;
//! use events::EventsPlugin;
//!
//! App::new()
//!     .add_plugins(DefaultPlugins)
//!     .add_plugins(EventsPlugin)
//!     .run();
//! ```
//!
//! Counterpart: tech-demos/godot/event-bus — the same concept in Godot.

use bevy::prelude::*;

/// Bundles every system, message, and resource for the events feature.
///
/// Add it with `app.add_plugins(EventsPlugin)`. It does **not** add
/// `DefaultPlugins`, so the host app stays in control of window and rendering
/// setup.
pub struct EventsPlugin;

impl Plugin for EventsPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<EventsConfig>()
            .add_message::<AddScore>()
            .add_message::<ResetScore>()
            .init_resource::<Score>()
            .add_systems(Startup, setup)
            .add_systems(
                Update,
                (handle_input, apply_add_score, apply_reset_score, update_hud).chain(),
            );
    }
}

// --- Configuration ---

/// Tunable parameters for the feature. Override before adding the plugin, e.g.
/// `app.insert_resource(EventsConfig { points_per_press: 25, ..default() })`.
#[derive(Resource, Clone, Copy, Debug)]
pub struct EventsConfig {
    /// Points added to the score per SPACE press.
    pub points_per_press: u32,
}

impl Default for EventsConfig {
    fn default() -> Self {
        Self {
            points_per_press: 10,
        }
    }
}

// --- Messages ---

/// Request to add `u32` points to the score.
#[derive(Message)]
pub struct AddScore(pub u32);

/// Request to reset the score to zero.
#[derive(Message)]
pub struct ResetScore;

// --- Resources ---

/// Accumulated player score for the session.
#[derive(Resource, Default)]
pub struct Score(pub u32);

// --- Marker components ---

/// Marks the score display text entity.
#[derive(Component)]
struct ScoreText;

/// Marks the "last action received" text entity.
#[derive(Component)]
struct LastActionText;

// --- Setup ---

/// Spawns a decorative sprite and all HUD text entities.
fn setup(mut commands: Commands) {
    commands.spawn(Camera2d);

    commands.spawn((
        Sprite {
            color: Color::srgb(0.25, 0.55, 0.9),
            custom_size: Some(Vec2::splat(60.0)),
            ..default()
        },
        Transform::default(),
    ));

    commands.spawn((
        Text::new("Score: 0"),
        TextFont {
            font_size: 32.0,
            ..default()
        },
        TextColor(Color::WHITE),
        Node {
            position_type: PositionType::Absolute,
            top: Val::Px(12.0),
            left: Val::Px(12.0),
            ..default()
        },
        ScoreText,
    ));

    commands.spawn((
        Text::new("(no action yet)"),
        TextFont {
            font_size: 16.0,
            ..default()
        },
        TextColor(Color::srgb(0.6, 0.85, 0.6)),
        Node {
            position_type: PositionType::Absolute,
            top: Val::Px(54.0),
            left: Val::Px(12.0),
            ..default()
        },
        LastActionText,
    ));

    commands.spawn((
        Text::new("SPACE = +10 score   R = reset score"),
        TextFont {
            font_size: 14.0,
            ..default()
        },
        TextColor(Color::srgb(0.6, 0.6, 0.6)),
        Node {
            position_type: PositionType::Absolute,
            bottom: Val::Px(12.0),
            left: Val::Px(12.0),
            ..default()
        },
    ));
}

// --- Systems ---

/// Reads keys and writes messages.  Touches no game state directly.
fn handle_input(
    input: Res<ButtonInput<KeyCode>>,
    config: Res<EventsConfig>,
    mut add_writer: MessageWriter<AddScore>,
    mut reset_writer: MessageWriter<ResetScore>,
) {
    if input.just_pressed(KeyCode::Space) {
        add_writer.write(AddScore(config.points_per_press));
    }
    if input.just_pressed(KeyCode::KeyR) {
        reset_writer.write(ResetScore);
    }
}

/// Receives [`AddScore`] messages and accumulates them into the [`Score`] resource.
fn apply_add_score(
    mut reader: MessageReader<AddScore>,
    mut score: ResMut<Score>,
    mut label_query: Query<&mut Text, With<LastActionText>>,
) {
    for msg in reader.read() {
        score.0 += msg.0;
        for mut text in &mut label_query {
            *text = Text::new(format!("AddScore({}) received", msg.0));
        }
    }
}

/// Receives [`ResetScore`] messages and zeroes the [`Score`] resource.
fn apply_reset_score(
    mut reader: MessageReader<ResetScore>,
    mut score: ResMut<Score>,
    mut label_query: Query<&mut Text, With<LastActionText>>,
) {
    for _ in reader.read() {
        score.0 = 0;
        for mut text in &mut label_query {
            *text = Text::new("ResetScore received".to_string());
        }
    }
}

/// Rewrites the score display whenever [`Score`] changes.
fn update_hud(score: Res<Score>, mut query: Query<&mut Text, With<ScoreText>>) {
    if !score.is_changed() {
        return;
    }
    for mut text in &mut query {
        *text = Text::new(format!("Score: {}", score.0));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn score_defaults_to_zero() {
        assert_eq!(Score::default().0, 0);
    }

    #[test]
    fn config_default_matches_documented_values() {
        let c = EventsConfig::default();
        assert_eq!(c.points_per_press, 10);
    }

    #[test]
    fn setup_spawns_one_score_text() {
        let mut app = App::new();
        app.add_plugins(MinimalPlugins)
            .add_message::<AddScore>()
            .add_message::<ResetScore>()
            .init_resource::<Score>()
            .add_systems(Startup, setup);
        app.update();

        let mut q = app.world_mut().query::<&ScoreText>();
        assert_eq!(q.iter(app.world()).count(), 1);
    }

    #[test]
    fn setup_spawns_one_last_action_text() {
        let mut app = App::new();
        app.add_plugins(MinimalPlugins)
            .add_message::<AddScore>()
            .add_message::<ResetScore>()
            .init_resource::<Score>()
            .add_systems(Startup, setup);
        app.update();

        let mut q = app.world_mut().query::<&LastActionText>();
        assert_eq!(q.iter(app.world()).count(), 1);
    }

    #[test]
    fn setup_spawns_one_camera() {
        let mut app = App::new();
        app.add_plugins(MinimalPlugins)
            .add_message::<AddScore>()
            .add_message::<ResetScore>()
            .init_resource::<Score>()
            .add_systems(Startup, setup);
        app.update();

        let mut q = app.world_mut().query::<&Camera2d>();
        assert_eq!(q.iter(app.world()).count(), 1);
    }

    #[test]
    fn plugin_spawns_one_score_text() {
        // Demonstrates the building-block path: the plugin composes onto a
        // headless app with no extra wiring.
        let mut app = App::new();
        app.add_plugins((MinimalPlugins, EventsPlugin));
        // The plugin's Update systems read input; MinimalPlugins omits it.
        app.insert_resource(ButtonInput::<KeyCode>::default());
        app.update();

        let mut q = app.world_mut().query::<&ScoreText>();
        assert_eq!(q.iter(app.world()).count(), 1);
    }

    #[test]
    fn score_accumulates_correctly() {
        let mut score = Score(0);
        score.0 += 10;
        score.0 += 10;
        assert_eq!(score.0, 20);
    }

    #[test]
    fn score_resets_to_zero() {
        let mut score = Score(50);
        score.0 = 0;
        assert_eq!(score.0, 0);
    }

    #[test]
    fn add_score_carries_correct_value() {
        let msg = AddScore(42);
        assert_eq!(msg.0, 42);
    }
}
