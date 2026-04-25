//! Events (Message API) demo.
//!
//! Key idea: systems communicate through messages rather than shared mutable
//! state.  Sender systems write messages; receiver systems read them
//! independently.  This keeps systems fully decoupled — the input system does
//! not know who handles the score, and multiple receivers can react to the
//! same message.
//!
//! Bevy 0.17+ uses `#[derive(Message)]` / [`MessageWriter`] / [`MessageReader`].

use bevy::prelude::*;

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .add_message::<AddScore>()
        .add_message::<ResetScore>()
        .init_resource::<Score>()
        .add_systems(Startup, setup)
        .add_systems(
            Update,
            (
                handle_input,
                apply_add_score,
                apply_reset_score,
                update_hud,
            )
                .chain(),
        )
        .run();
}

// --- Messages ---

/// Request to add `u32` points to the score.
#[derive(Message)]
struct AddScore(u32);

/// Request to reset the score to zero.
#[derive(Message)]
struct ResetScore;

// --- Resources ---

/// Accumulated player score for the session.
#[derive(Resource, Default)]
struct Score(u32);

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
        TextFont { font_size: 32.0, ..default() },
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
        TextFont { font_size: 16.0, ..default() },
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
        TextFont { font_size: 14.0, ..default() },
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
    mut add_writer: MessageWriter<AddScore>,
    mut reset_writer: MessageWriter<ResetScore>,
) {
    if input.just_pressed(KeyCode::Space) {
        add_writer.write(AddScore(10));
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
}
