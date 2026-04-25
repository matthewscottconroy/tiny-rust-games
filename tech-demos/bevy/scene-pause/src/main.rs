//! Scene pause demo.
//!
//! Key ideas:
//! - [`GameState`] drives which systems run via `run_if(in_state(...))`.
//! - [`OnEnter`] / [`OnExit`] schedules handle one-shot overlay show/hide.
//! - Physics systems only run in [`GameState::Running`]; the pause overlay
//!   is only visible in [`GameState::Paused`].

use bevy::prelude::*;

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .init_state::<GameState>()
        .add_systems(Startup, setup)
        .add_systems(Update, toggle_pause)
        .add_systems(
            Update,
            (move_balls, bounce_walls).run_if(in_state(GameState::Running)),
        )
        .add_systems(OnEnter(GameState::Paused), show_overlay)
        .add_systems(OnExit(GameState::Paused),  hide_overlay)
        .run();
}

// --- State ---

/// Top-level app state toggled by pressing `P`.
#[derive(States, Default, Debug, Clone, PartialEq, Eq, Hash)]
enum GameState {
    #[default]
    Running,
    Paused,
}

// --- Components ---

/// Tags a bouncing ball in the playing scene.
#[derive(Component)]
struct Ball;

/// 2D linear velocity in world units/second.
#[derive(Component)]
struct Velocity(Vec2);

/// Tags the full-screen pause overlay node.
#[derive(Component)]
struct PauseOverlay;

// --- Setup ---

/// Spawns three balls, the pause overlay (initially hidden), and a HUD label.
fn setup(mut commands: Commands) {
    commands.spawn(Camera2d);

    let balls = [
        (Vec3::new(-120.0,  80.0, 0.0), Vec2::new( 180.0,  130.0), Color::srgb(0.9, 0.4, 0.2)),
        (Vec3::new( 100.0, -60.0, 0.0), Vec2::new(-220.0,  100.0), Color::srgb(0.2, 0.6, 0.9)),
        (Vec3::new(   0.0, 140.0, 0.0), Vec2::new( 140.0, -190.0), Color::srgb(0.7, 0.9, 0.2)),
    ];

    for (pos, vel, color) in balls {
        commands.spawn((
            Sprite { color, custom_size: Some(Vec2::splat(28.0)), ..default() },
            Transform::from_translation(pos),
            Ball,
            Velocity(vel),
        ));
    }

    commands
        .spawn((
            Node {
                position_type: PositionType::Absolute,
                width: Val::Percent(100.0),
                height: Val::Percent(100.0),
                justify_content: JustifyContent::Center,
                align_items: AlignItems::Center,
                ..default()
            },
            BackgroundColor(Color::srgba(0.0, 0.0, 0.0, 0.65)),
            Visibility::Hidden,
            PauseOverlay,
        ))
        .with_children(|parent| {
            parent.spawn((
                Text::new("PAUSED"),
                TextFont { font_size: 64.0, ..default() },
                TextColor(Color::WHITE),
            ));
        });

    commands.spawn((
        Text::new("P — toggle pause"),
        TextFont { font_size: 16.0, ..default() },
        TextColor(Color::srgb(0.75, 0.75, 0.75)),
        Node {
            position_type: PositionType::Absolute,
            bottom: Val::Px(10.0),
            left: Val::Px(10.0),
            ..default()
        },
    ));
}

// --- Systems ---

/// Toggles between [`GameState::Running`] and [`GameState::Paused`] on `P`.
fn toggle_pause(
    input: Res<ButtonInput<KeyCode>>,
    state: Res<State<GameState>>,
    mut next: ResMut<NextState<GameState>>,
) {
    if input.just_pressed(KeyCode::KeyP) {
        let new_state = match state.get() {
            GameState::Running => GameState::Paused,
            GameState::Paused  => GameState::Running,
        };
        next.set(new_state);
    }
}

/// Advances each ball by its velocity; runs only in [`GameState::Running`].
fn move_balls(time: Res<Time>, mut query: Query<(&mut Transform, &Velocity), With<Ball>>) {
    for (mut transform, vel) in &mut query {
        transform.translation.x += vel.0.x * time.delta_secs();
        transform.translation.y += vel.0.y * time.delta_secs();
    }
}

/// Reflects each ball's velocity when it reaches the arena boundary.
fn bounce_walls(mut query: Query<(&Transform, &mut Velocity), With<Ball>>) {
    for (transform, mut vel) in &mut query {
        let p = transform.translation;
        if p.x.abs() > 430.0 { vel.0.x *= -1.0; }
        if p.y.abs() > 280.0 { vel.0.y *= -1.0; }
    }
}

/// Makes the [`PauseOverlay`] visible on entering the Paused state.
fn show_overlay(mut query: Query<&mut Visibility, With<PauseOverlay>>) {
    for mut vis in &mut query {
        *vis = Visibility::Visible;
    }
}

/// Hides the [`PauseOverlay`] on leaving the Paused state.
fn hide_overlay(mut query: Query<&mut Visibility, With<PauseOverlay>>) {
    for mut vis in &mut query {
        *vis = Visibility::Hidden;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn game_state_default_is_running() {
        assert_eq!(GameState::default(), GameState::Running);
    }

    #[test]
    fn setup_spawns_three_balls() {
        let mut app = App::new();
        app.add_plugins(MinimalPlugins)
            .add_systems(Startup, setup);
        app.update();

        let mut q = app.world_mut().query::<&Ball>();
        assert_eq!(q.iter(app.world()).count(), 3);
    }

    #[test]
    fn setup_spawns_one_pause_overlay() {
        let mut app = App::new();
        app.add_plugins(MinimalPlugins)
            .add_systems(Startup, setup);
        app.update();

        let mut q = app.world_mut().query::<&PauseOverlay>();
        assert_eq!(q.iter(app.world()).count(), 1);
    }

    #[test]
    fn pause_overlay_starts_hidden() {
        let mut app = App::new();
        app.add_plugins(MinimalPlugins)
            .add_systems(Startup, setup);
        app.update();

        let mut q = app.world_mut().query::<(&PauseOverlay, &Visibility)>();
        for (_, vis) in q.iter(app.world()) {
            assert_eq!(*vis, Visibility::Hidden);
        }
    }
}
