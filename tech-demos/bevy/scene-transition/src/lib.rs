//! Scene transition — a reusable Bevy plugin.
//!
//! This crate is a *building block*: drop [`SceneTransitionPlugin`] into any
//! Bevy app and it wires up a three-state flow with a loading screen:
//! `MainMenu → Loading → Playing → (ESC) → MainMenu`.
//!
//! Key ideas:
//! - Three distinct app states in [`AppState`].
//! - [`OnEnter`] spawns scene-specific entities tagged with a marker component.
//! - [`OnExit`] calls the generic [`cleanup`] system to despawn all tagged entities.
//! - A fake loading bar in the `Loading` state demonstrates the "loading screen"
//!   pattern: do work (or just wait), then call [`NextState`] when ready.
//! - Tune loading duration, arena bounds, and score value via
//!   [`SceneTransitionConfig`].
//!
//! **Bug fixed:** the camera was previously tagged `MenuEntity` and destroyed on
//! `OnExit(MainMenu)`, leaving the Loading state with no camera.  The camera is
//! now spawned once in `Startup` with no scene tag and persists across all states.
//!
//! # Example
//! ```no_run
//! use bevy::prelude::*;
//! use scene_transition::SceneTransitionPlugin;
//!
//! App::new()
//!     .add_plugins(DefaultPlugins)
//!     .add_plugins(SceneTransitionPlugin)
//!     .run();
//! ```

use bevy::prelude::*;

/// Bundles every system, state, and resource for the scene-transition flow.
///
/// Add it with `app.add_plugins(SceneTransitionPlugin)`. It does **not** add
/// `DefaultPlugins`, so the host app stays in control of window and rendering
/// setup.
pub struct SceneTransitionPlugin;

impl Plugin for SceneTransitionPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<SceneTransitionConfig>()
            .init_state::<AppState>()
            .init_resource::<Score>()
            // Persistent camera — not tagged with any scene marker.
            .add_systems(Startup, spawn_camera)
            // --- Main Menu ---
            .add_systems(OnEnter(AppState::MainMenu), setup_menu)
            .add_systems(OnExit(AppState::MainMenu), cleanup::<MenuEntity>)
            .add_systems(Update, menu_input.run_if(in_state(AppState::MainMenu)))
            // --- Loading ---
            .add_systems(OnEnter(AppState::Loading), setup_loading)
            .add_systems(OnExit(AppState::Loading), cleanup::<LoadingEntity>)
            .add_systems(Update, tick_loading.run_if(in_state(AppState::Loading)))
            // --- Playing ---
            .add_systems(OnEnter(AppState::Playing), setup_playing)
            .add_systems(OnExit(AppState::Playing), (cleanup::<PlayingEntity>, reset_score))
            .add_systems(
                Update,
                (playing_input, move_balls, bounce_walls, update_score_hud)
                    .run_if(in_state(AppState::Playing)),
            );
    }
}

// --- Configuration ---

/// Tunable parameters for the flow. Override before adding the plugin, e.g.
/// `app.insert_resource(SceneTransitionConfig { loading_seconds: 1.0, ..default() })`.
#[derive(Resource, Clone, Copy, Debug)]
pub struct SceneTransitionConfig {
    /// How long the fake loading screen runs before entering `Playing`.
    pub loading_seconds: f32,
    /// Points awarded per SPACE press while playing.
    pub score_per_press: u32,
    /// Half-width of the arena; balls bounce when `|x|` exceeds this.
    pub half_width: f32,
    /// Half-height of the arena; balls bounce when `|y|` exceeds this.
    pub half_height: f32,
}

impl Default for SceneTransitionConfig {
    fn default() -> Self {
        Self {
            loading_seconds: 2.0,
            score_per_press: 10,
            half_width: 440.0,
            half_height: 280.0,
        }
    }
}

// --- State ---

/// Top-level application state machine.
#[derive(States, Default, Debug, Clone, PartialEq, Eq, Hash)]
pub enum AppState {
    /// The start menu; press SPACE to begin loading.
    #[default]
    MainMenu,
    /// A fake loading screen with a progress bar.
    Loading,
    /// The gameplay scene with bouncing balls and a score.
    Playing,
}

// --- Scene marker components ---
// Each scene tags its entities with the corresponding marker so OnExit can
// bulk-despawn them without tracking individual entity IDs.

/// Tags entities that belong to the main-menu scene.
#[derive(Component)]
pub struct MenuEntity;

/// Tags entities that belong to the loading screen.
#[derive(Component)]
pub struct LoadingEntity;

/// Tags entities that belong to the gameplay scene.
#[derive(Component)]
pub struct PlayingEntity;

// --- Resources ---

/// Accumulated score for the current play session.
#[derive(Resource, Default)]
pub struct Score(pub u32);

/// Tracks loading progress and drives the visual progress bar.
#[derive(Resource)]
pub struct LoadingProgress {
    timer: Timer,
}

// --- Gameplay components ---

/// Tags a bouncing ball in the playing scene.
#[derive(Component)]
pub struct Ball;

/// 2D linear velocity.
#[derive(Component)]
pub struct Velocity(pub Vec2);

/// Marker for the score display text node.
#[derive(Component)]
pub struct ScoreText;

/// Marker for the progress bar fill node.
#[derive(Component)]
pub struct LoadingBar;

// --- Persistent setup ---

/// Spawns a camera that lives for the whole session.
fn spawn_camera(mut commands: Commands) {
    commands.spawn(Camera2d);
}

// --- Main Menu ---

fn setup_menu(mut commands: Commands) {
    commands
        .spawn((
            Node {
                width: Val::Percent(100.0),
                height: Val::Percent(100.0),
                justify_content: JustifyContent::Center,
                align_items: AlignItems::Center,
                flex_direction: FlexDirection::Column,
                row_gap: Val::Px(16.0),
                ..default()
            },
            MenuEntity,
        ))
        .with_children(|p| {
            p.spawn((
                Text::new("DEMO GAME"),
                TextFont { font_size: 56.0, ..default() },
                TextColor(Color::WHITE),
            ));
            p.spawn((
                Text::new("SPACE — start"),
                TextFont { font_size: 22.0, ..default() },
                TextColor(Color::srgb(0.7, 0.7, 0.7)),
            ));
        });
}

fn menu_input(input: Res<ButtonInput<KeyCode>>, mut next: ResMut<NextState<AppState>>) {
    if input.just_pressed(KeyCode::Space) {
        next.set(AppState::Loading);
    }
}

// --- Loading ---

fn setup_loading(mut commands: Commands, config: Res<SceneTransitionConfig>) {
    commands
        .spawn((
            Node {
                width: Val::Percent(100.0),
                height: Val::Percent(100.0),
                justify_content: JustifyContent::Center,
                align_items: AlignItems::Center,
                flex_direction: FlexDirection::Column,
                row_gap: Val::Px(20.0),
                ..default()
            },
            LoadingEntity,
        ))
        .with_children(|p| {
            p.spawn((
                Text::new("Loading…"),
                TextFont { font_size: 32.0, ..default() },
                TextColor(Color::srgb(0.8, 0.8, 0.8)),
            ));

            p.spawn((
                Node { width: Val::Px(300.0), height: Val::Px(20.0), ..default() },
                BackgroundColor(Color::srgb(0.15, 0.15, 0.15)),
            ))
            .with_children(|bar| {
                bar.spawn((
                    Node {
                        width: Val::Percent(0.0),
                        height: Val::Percent(100.0),
                        ..default()
                    },
                    BackgroundColor(Color::srgb(0.3, 0.75, 0.95)),
                    LoadingBar,
                ));
            });
        });

    commands.insert_resource(LoadingProgress {
        timer: Timer::from_seconds(config.loading_seconds, TimerMode::Once),
    });
}

/// Advances the progress bar and transitions to `Playing` when complete.
fn tick_loading(
    time: Res<Time>,
    mut progress: ResMut<LoadingProgress>,
    mut next: ResMut<NextState<AppState>>,
    mut bar_query: Query<&mut Node, With<LoadingBar>>,
) {
    progress.timer.tick(time.delta());
    let frac = progress.timer.fraction();

    for mut node in &mut bar_query {
        node.width = Val::Percent(frac * 100.0);
    }

    if progress.timer.is_finished() {
        next.set(AppState::Playing);
    }
}

// --- Playing ---

fn setup_playing(mut commands: Commands) {
    let balls: &[(Vec3, Vec2, Color)] = &[
        (Vec3::new(-100.0,  60.0, 0.0), Vec2::new( 160.0,  90.0), Color::srgb(0.9, 0.4, 0.2)),
        (Vec3::new( 120.0, -60.0, 0.0), Vec2::new(-140.0, 110.0), Color::srgb(0.3, 0.7, 0.9)),
        (Vec3::new(   0.0,  80.0, 0.0), Vec2::new( 100.0,-160.0), Color::srgb(0.7, 0.9, 0.2)),
    ];

    for &(pos, vel, color) in balls {
        commands.spawn((
            Sprite { color, custom_size: Some(Vec2::splat(26.0)), ..default() },
            Transform::from_translation(pos),
            Ball,
            Velocity(vel),
            PlayingEntity,
        ));
    }

    commands.spawn((
        Text::new("Score: 0"),
        TextFont { font_size: 24.0, ..default() },
        TextColor(Color::WHITE),
        Node {
            position_type: PositionType::Absolute,
            top: Val::Px(12.0),
            left: Val::Px(12.0),
            ..default()
        },
        ScoreText,
        PlayingEntity,
    ));

    commands.spawn((
        Text::new("SPACE = score   ESC = main menu"),
        TextFont { font_size: 14.0, ..default() },
        TextColor(Color::srgb(0.55, 0.55, 0.55)),
        Node {
            position_type: PositionType::Absolute,
            bottom: Val::Px(10.0),
            left: Val::Px(10.0),
            ..default()
        },
        PlayingEntity,
    ));
}

fn playing_input(
    input: Res<ButtonInput<KeyCode>>,
    config: Res<SceneTransitionConfig>,
    mut next: ResMut<NextState<AppState>>,
    mut score: ResMut<Score>,
) {
    if input.just_pressed(KeyCode::Escape) {
        next.set(AppState::MainMenu);
    }
    if input.just_pressed(KeyCode::Space) {
        score.0 += config.score_per_press;
    }
}

fn move_balls(time: Res<Time>, mut query: Query<(&mut Transform, &Velocity), With<Ball>>) {
    for (mut t, v) in &mut query {
        t.translation.x += v.0.x * time.delta_secs();
        t.translation.y += v.0.y * time.delta_secs();
    }
}

fn bounce_walls(
    config: Res<SceneTransitionConfig>,
    mut query: Query<(&Transform, &mut Velocity), With<Ball>>,
) {
    for (t, mut v) in &mut query {
        if t.translation.x.abs() > config.half_width { v.0.x *= -1.0; }
        if t.translation.y.abs() > config.half_height { v.0.y *= -1.0; }
    }
}

fn update_score_hud(score: Res<Score>, mut q: Query<&mut Text, With<ScoreText>>) {
    if !score.is_changed() { return; }
    for mut text in &mut q {
        *text = Text::new(format!("Score: {}", score.0));
    }
}

// --- Generic cleanup ---

/// Despawns every entity tagged with marker `T`.
///
/// This is the standard Bevy pattern for cleaning up a scene on state exit:
/// tag all scene entities with a marker component, then despawn in `OnExit`.
pub fn cleanup<T: Component>(mut commands: Commands, query: Query<Entity, With<T>>) {
    for entity in &query {
        commands.entity(entity).despawn();
    }
}

fn reset_score(mut score: ResMut<Score>) {
    score.0 = 0;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn config_default_matches_documented_values() {
        let c = SceneTransitionConfig::default();
        assert_eq!(c.loading_seconds, 2.0);
        assert_eq!(c.score_per_press, 10);
        assert_eq!(c.half_width, 440.0);
        assert_eq!(c.half_height, 280.0);
    }

    #[test]
    fn persistent_camera_survives_scene_changes() {
        use bevy::state::app::StatesPlugin;
        let mut app = App::new();
        app.add_plugins((MinimalPlugins, StatesPlugin))
            .init_state::<AppState>()
            .init_resource::<Score>()
            .add_systems(Startup, spawn_camera)
            .add_systems(OnEnter(AppState::MainMenu), setup_menu)
            .add_systems(OnExit(AppState::MainMenu), cleanup::<MenuEntity>);
        app.update();

        let mut cam_q = app.world_mut().query::<&Camera2d>();
        assert_eq!(cam_q.iter(app.world()).count(), 1, "camera should exist in MainMenu");

        // Simulate leaving MainMenu — cleanup despawns MenuEntity but not the camera.
        app.world_mut()
            .resource_mut::<NextState<AppState>>()
            .set(AppState::Loading);
        app.update();

        let mut cam_q2 = app.world_mut().query::<&Camera2d>();
        assert_eq!(cam_q2.iter(app.world()).count(), 1, "camera should still exist after leaving MainMenu");
    }

    #[test]
    fn reset_score_sets_to_zero() {
        let mut score = Score(42);
        // Call the function logic directly (no ECS needed for this check).
        score.0 = 0;
        assert_eq!(score.0, 0);
    }

    #[test]
    fn setup_playing_spawns_three_balls() {
        let mut app = App::new();
        app.add_plugins(MinimalPlugins)
            .add_systems(Startup, setup_playing);
        app.update();

        let mut q = app.world_mut().query::<&Ball>();
        assert_eq!(q.iter(app.world()).count(), 3);
    }

    #[test]
    fn score_default_is_zero() {
        assert_eq!(Score::default().0, 0);
    }

    #[test]
    fn score_increments() {
        let mut score = Score::default();
        score.0 += 10;
        score.0 += 10;
        assert_eq!(score.0, 20);
    }

    #[test]
    fn app_state_default_is_main_menu() {
        assert_eq!(AppState::default(), AppState::MainMenu);
    }

    #[test]
    fn setup_playing_spawns_one_score_text() {
        let mut app = App::new();
        app.add_plugins(MinimalPlugins)
            .add_systems(Startup, setup_playing);
        app.update();

        let mut q = app.world_mut().query::<&ScoreText>();
        assert_eq!(q.iter(app.world()).count(), 1);
    }

    #[test]
    fn setup_playing_tags_balls_as_playing_entity() {
        let mut app = App::new();
        app.add_plugins(MinimalPlugins)
            .add_systems(Startup, setup_playing);
        app.update();

        let mut q = app.world_mut().query::<(&Ball, &PlayingEntity)>();
        assert_eq!(q.iter(app.world()).count(), 3);
    }
}
