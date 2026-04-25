//! HUD and UI demo.
//!
//! Key ideas:
//! - Score and health are plain resources; UI text entities mirror them each frame.
//! - `is_changed()` guards the update systems so they only rewrite text when the
//!   underlying data actually changes.
//! - The health bar uses a nested UI node whose `width` shrinks as HP drops.

use bevy::prelude::*;

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .init_resource::<Score>()
        .init_resource::<PlayerHealth>()
        .add_systems(Startup, setup)
        .add_systems(Update, (handle_input, update_score_text, update_health_bar))
        .run();
}

// --- Resources ---

/// Accumulated player score.
#[derive(Resource, Default)]
struct Score(u32);

/// Player hit points.
#[derive(Resource)]
struct PlayerHealth {
    current: f32,
    max: f32,
}

impl Default for PlayerHealth {
    fn default() -> Self {
        Self { current: 100.0, max: 100.0 }
    }
}

// --- Marker components ---

/// Marks the score display text.
#[derive(Component)]
struct ScoreText;

/// Marks the health bar fill node.
#[derive(Component)]
struct HealthFill;

// --- Setup ---

/// Spawns a decorative sprite, score text, health bar, and instructions.
fn setup(mut commands: Commands) {
    commands.spawn(Camera2d);

    commands.spawn((
        Sprite {
            color: Color::srgb(0.3, 0.6, 0.9),
            custom_size: Some(Vec2::splat(60.0)),
            ..default()
        },
        Transform::default(),
    ));

    commands.spawn((
        Text::new("Score: 0"),
        TextFont { font_size: 28.0, ..default() },
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
        Text::new("Health"),
        TextFont { font_size: 18.0, ..default() },
        TextColor(Color::srgb(0.9, 0.9, 0.9)),
        Node {
            position_type: PositionType::Absolute,
            top: Val::Px(50.0),
            left: Val::Px(12.0),
            ..default()
        },
    ));

    commands
        .spawn((
            Node {
                position_type: PositionType::Absolute,
                top: Val::Px(74.0),
                left: Val::Px(12.0),
                width: Val::Px(200.0),
                height: Val::Px(18.0),
                ..default()
            },
            BackgroundColor(Color::srgb(0.15, 0.15, 0.15)),
        ))
        .with_children(|parent| {
            parent.spawn((
                Node {
                    width: Val::Percent(100.0),
                    height: Val::Percent(100.0),
                    ..default()
                },
                BackgroundColor(Color::srgb(0.8, 0.15, 0.15)),
                HealthFill,
            ));
        });

    commands.spawn((
        Text::new("SPACE = +10 score    D = -10 health"),
        TextFont { font_size: 14.0, ..default() },
        TextColor(Color::srgb(0.7, 0.7, 0.7)),
        Node {
            position_type: PositionType::Absolute,
            bottom: Val::Px(12.0),
            left: Val::Px(12.0),
            ..default()
        },
    ));
}

// --- Systems ---

/// Handles SPACE (+10 score) and D (-10 health) input.
fn handle_input(
    input: Res<ButtonInput<KeyCode>>,
    mut score: ResMut<Score>,
    mut health: ResMut<PlayerHealth>,
) {
    if input.just_pressed(KeyCode::Space) {
        score.0 += 10;
    }
    if input.just_pressed(KeyCode::KeyD) {
        health.current = (health.current - 10.0).max(0.0);
    }
}

/// Rewrites the score label when [`Score`] changes.
fn update_score_text(score: Res<Score>, mut query: Query<&mut Text, With<ScoreText>>) {
    if !score.is_changed() { return; }
    for mut text in &mut query {
        *text = Text::new(format!("Score: {}", score.0));
    }
}

/// Resizes the health-bar fill to match current HP percentage.
fn update_health_bar(
    health: Res<PlayerHealth>,
    mut fill_query: Query<&mut Node, With<HealthFill>>,
) {
    if !health.is_changed() { return; }
    let pct = (health.current / health.max * 100.0).clamp(0.0, 100.0);
    for mut node in &mut fill_query {
        node.width = Val::Percent(pct);
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
    fn player_health_defaults_to_full() {
        let h = PlayerHealth::default();
        assert_eq!(h.current, h.max);
    }

    #[test]
    fn player_health_fraction_at_full() {
        let h = PlayerHealth::default();
        assert!((h.current / h.max - 1.0).abs() < 1e-6);
    }

    #[test]
    fn setup_spawns_score_text() {
        let mut app = App::new();
        app.add_plugins(MinimalPlugins)
            .init_resource::<Score>()
            .init_resource::<PlayerHealth>()
            .add_systems(Startup, setup);
        app.update();

        let mut q = app.world_mut().query::<&ScoreText>();
        assert_eq!(q.iter(app.world()).count(), 1);
    }

    #[test]
    fn setup_spawns_health_fill() {
        let mut app = App::new();
        app.add_plugins(MinimalPlugins)
            .init_resource::<Score>()
            .init_resource::<PlayerHealth>()
            .add_systems(Startup, setup);
        app.update();

        let mut q = app.world_mut().query::<&HealthFill>();
        assert_eq!(q.iter(app.world()).count(), 1);
    }
}
