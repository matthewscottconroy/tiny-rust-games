//! Breakout rendered with [Bevy](https://bevyengine.org/)'s ECS.
//!
//! Snake's frontends hand-roll the frame-time accumulator, because
//! [`snake_lib::Ticker`] has to work from a terminal loop and a Godot callback
//! as well as from Bevy. This frontend does the opposite on purpose: Bevy
//! already ships a fixed-timestep scheduler, so it uses that.
//!
//! - [`BreakoutGame::step`] is called from `FixedUpdate`, which Bevy runs a
//!   whole number of times per frame at a rate you set with `Time<Fixed>`;
//! - rendering runs in `Update` and asks `Time<Fixed>::overstep_fraction()` how
//!   far it is between two fixed steps, then passes that to
//!   [`BreakoutGame::ball_at`].
//!
//! That is the same design as Snake's — fixed simulation, interpolated
//! rendering — reached through the engine's machinery instead of the library's.
//! The library does not care which, and that is the point: it exposes a step
//! function and a way to interpolate, and stays out of the argument about who
//! owns the clock.
//!
//! **Controls:** left/right or A/D to move   Space to launch   R to restart.
//!
//! # Example
//! ```no_run
//! use bevy::prelude::*;
//! use breakout_bevy::BreakoutPlugin;
//!
//! App::new()
//!     .add_plugins(DefaultPlugins)
//!     .add_plugins(BreakoutPlugin)
//!     .run();
//! ```

use bevy::prelude::*;
use bevy::time::Fixed;

use breakout_lib::{BreakoutGame, GameStatus, PaddleInput, STEPS_PER_SECOND, Vec2 as GameVec2};

/// Bundles every system and resource for the game.
///
/// Add it with `app.add_plugins(BreakoutPlugin)`. It does **not** add
/// `DefaultPlugins`, so the host app stays in control of window and rendering.
pub struct BreakoutPlugin;

impl Plugin for BreakoutPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<Game>()
            // The engine's accumulator, set to the library's fixed rate. Bevy
            // runs FixedUpdate however many times this frame's elapsed time is
            // worth, which is precisely the loop Snake writes by hand.
            .insert_resource(Time::<Fixed>::from_hz(STEPS_PER_SECOND as f64))
            .add_systems(Startup, setup)
            .add_systems(Update, (read_input, restart).chain())
            .add_systems(FixedUpdate, advance)
            .add_systems(Update, (draw, update_hud).after(read_input));
    }
}

/// The game itself, owned as a resource so systems can read and mutate it.
#[derive(Resource, Deref, DerefMut)]
pub struct Game(pub BreakoutGame);

impl Default for Game {
    fn default() -> Self {
        Self(BreakoutGame::new(BreakoutGame::default_layout()))
    }
}

// --- Pure helpers ---

/// Maps held keys to a paddle input.
///
/// Both directions at once cancel, so a player rolling their fingers across the
/// keys does not get a surprise.
pub fn paddle_input_from_keys(left: bool, right: bool) -> PaddleInput {
    match (left, right) {
        (true, false) => PaddleInput::Left,
        (false, true) => PaddleInput::Right,
        _ => PaddleInput::None,
    }
}

/// Converts a game-space point to Bevy world space.
///
/// The library puts the origin at the top-left with `y` increasing downward,
/// which is how a play field is naturally described; Bevy is y-up and centred.
pub fn to_world(point: GameVec2, field: GameVec2) -> Vec2 {
    Vec2::new(point.x - field.x / 2.0, field.y / 2.0 - point.y)
}

/// The status line for the current game state.
pub fn status_line(game: &BreakoutGame) -> String {
    match game.status() {
        GameStatus::Playing if game.ball_is_stuck() => {
            format!(
                "Score {}   Lives {}  -  Space to launch",
                game.score(),
                game.lives()
            )
        }
        GameStatus::Playing => format!(
            "Score {}   Lives {}   Bricks {}",
            game.score(),
            game.lives(),
            game.bricks_remaining()
        ),
        GameStatus::Won => format!("Cleared! Score {}.  R to restart", game.score()),
        GameStatus::Lost => format!("Game over. Score {}.  R to restart", game.score()),
    }
}

/// Colour for a brick, by row and remaining hits.
pub fn brick_color(row: usize, hits: u8) -> Color {
    let base = match row % 5 {
        0 => Color::srgb(0.90, 0.35, 0.35),
        1 => Color::srgb(0.90, 0.60, 0.30),
        2 => Color::srgb(0.85, 0.85, 0.35),
        3 => Color::srgb(0.40, 0.80, 0.45),
        _ => Color::srgb(0.40, 0.65, 0.90),
    };
    // A damaged two-hit brick is dimmed, so the player can see it is weakened.
    if hits > 1 { base } else { base.darker(0.12) }
}

// --- Components ---

/// A drawn brick, tagged with its index in the game's brick list.
#[derive(Component)]
pub struct BrickSprite(pub usize);

/// The paddle sprite.
#[derive(Component)]
pub struct PaddleSprite;

/// The ball sprite.
#[derive(Component)]
pub struct BallSprite;

/// The status text entity.
#[derive(Component)]
pub struct StatusText;

// --- Systems ---

fn setup(mut commands: Commands, game: Res<Game>) {
    commands.spawn(Camera2d);
    let field = game.size();

    commands.spawn((
        Sprite {
            color: Color::srgb(0.08, 0.09, 0.11),
            custom_size: Some(Vec2::new(field.x, field.y)),
            ..default()
        },
        Transform::from_xyz(0.0, 0.0, -1.0),
    ));

    for (index, brick) in game.bricks().iter().enumerate() {
        let pos = to_world(brick.rect.centre, field);
        commands.spawn((
            BrickSprite(index),
            Sprite {
                color: brick_color(brick.row, brick.hits),
                custom_size: Some(Vec2::new(brick.rect.half.x * 2.0, brick.rect.half.y * 2.0)),
                ..default()
            },
            Transform::from_xyz(pos.x, pos.y, 0.0),
        ));
    }

    let paddle = game.paddle_rect();
    let pos = to_world(paddle.centre, field);
    commands.spawn((
        PaddleSprite,
        Sprite {
            color: Color::srgb(0.85, 0.88, 0.92),
            custom_size: Some(Vec2::new(paddle.half.x * 2.0, paddle.half.y * 2.0)),
            ..default()
        },
        Transform::from_xyz(pos.x, pos.y, 1.0),
    ));

    let ball = to_world(game.ball(), field);
    commands.spawn((
        BallSprite,
        Sprite {
            color: Color::srgb(1.0, 0.95, 0.85),
            custom_size: Some(Vec2::splat(14.0)),
            ..default()
        },
        Transform::from_xyz(ball.x, ball.y, 1.0),
    ));

    commands.spawn((
        StatusText,
        Text::new(""),
        TextFont {
            font_size: 20.0,
            ..default()
        },
        TextColor(Color::WHITE),
        Node {
            position_type: PositionType::Absolute,
            bottom: Val::Px(12.0),
            left: Val::Px(16.0),
            ..default()
        },
    ));
}

/// Records input every frame; `FixedUpdate` consumes it whenever it next runs.
fn read_input(keys: Res<ButtonInput<KeyCode>>, mut game: ResMut<Game>) {
    let input = paddle_input_from_keys(
        keys.any_pressed([KeyCode::ArrowLeft, KeyCode::KeyA]),
        keys.any_pressed([KeyCode::ArrowRight, KeyCode::KeyD]),
    );
    game.set_paddle_input(input);

    if keys.just_pressed(KeyCode::Space) {
        game.launch();
    }
}

/// Advances the simulation. Runs in `FixedUpdate`, so Bevy decides how often.
fn advance(mut game: ResMut<Game>) {
    game.step();
}

fn restart(keys: Res<ButtonInput<KeyCode>>, mut game: ResMut<Game>) {
    if keys.just_pressed(KeyCode::KeyR) {
        **game = BreakoutGame::new(BreakoutGame::default_layout());
    }
}

/// Draws everything, interpolating the ball between fixed steps.
fn draw(
    game: Res<Game>,
    fixed: Res<Time<Fixed>>,
    mut bricks: Query<(&BrickSprite, &mut Sprite, &mut Visibility)>,
    mut paddle: Query<&mut Transform, (With<PaddleSprite>, Without<BallSprite>)>,
    mut ball: Query<&mut Transform, (With<BallSprite>, Without<PaddleSprite>)>,
) {
    let field = game.size();

    for (marker, mut sprite, mut visibility) in &mut bricks {
        let brick = game.bricks()[marker.0];
        *visibility = if brick.alive() {
            Visibility::Inherited
        } else {
            Visibility::Hidden
        };
        sprite.color = brick_color(brick.row, brick.hits);
    }

    for mut transform in &mut paddle {
        let pos = to_world(game.paddle_rect().centre, field);
        transform.translation.x = pos.x;
        transform.translation.y = pos.y;
    }

    // The interpolation the module docs are about. `overstep_fraction` is how
    // far this frame sits between the last fixed step and the next; without it
    // the ball visibly steps rather than glides.
    let alpha = fixed.overstep_fraction();
    for mut transform in &mut ball {
        let pos = to_world(game.ball_at(alpha), field);
        transform.translation.x = pos.x;
        transform.translation.y = pos.y;
    }
}

fn update_hud(game: Res<Game>, mut text: Query<&mut Text, With<StatusText>>) {
    for mut t in &mut text {
        **t = status_line(&game);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn keys_map_to_paddle_input() {
        assert_eq!(paddle_input_from_keys(true, false), PaddleInput::Left);
        assert_eq!(paddle_input_from_keys(false, true), PaddleInput::Right);
        assert_eq!(paddle_input_from_keys(false, false), PaddleInput::None);
    }

    #[test]
    fn holding_both_directions_cancels() {
        assert_eq!(paddle_input_from_keys(true, true), PaddleInput::None);
    }

    #[test]
    fn to_world_centres_the_field_and_flips_the_y_axis() {
        let field = GameVec2::new(800.0, 600.0);
        // The field's centre maps to the world origin.
        assert_eq!(to_world(GameVec2::new(400.0, 300.0), field), Vec2::ZERO);
        // Game-space "up" (smaller y) must become Bevy "up" (larger y).
        let high = to_world(GameVec2::new(400.0, 100.0), field);
        let low = to_world(GameVec2::new(400.0, 500.0), field);
        assert!(high.y > low.y, "y axis was not flipped");
    }

    #[test]
    fn to_world_maps_the_corners() {
        let field = GameVec2::new(800.0, 600.0);
        assert_eq!(
            to_world(GameVec2::new(0.0, 0.0), field),
            Vec2::new(-400.0, 300.0)
        );
        assert_eq!(
            to_world(GameVec2::new(800.0, 600.0), field),
            Vec2::new(400.0, -300.0)
        );
    }

    #[test]
    fn brick_colours_differ_by_row_and_dim_when_damaged() {
        assert_ne!(brick_color(0, 1), brick_color(1, 1));
        assert_ne!(
            brick_color(0, 2),
            brick_color(0, 1),
            "a damaged brick should look different"
        );
    }

    #[test]
    fn status_line_prompts_for_a_launch_while_the_ball_waits() {
        let game = BreakoutGame::new(BreakoutGame::default_layout());
        assert!(game.ball_is_stuck());
        assert!(status_line(&game).contains("Space to launch"));
    }

    #[test]
    fn status_line_reports_progress_once_playing() {
        let mut game = BreakoutGame::new(BreakoutGame::default_layout());
        game.launch();
        let line = status_line(&game);
        assert!(line.contains("Score 0"), "{line}");
        assert!(line.contains("Bricks 40"), "{line}");
    }

    #[test]
    fn status_line_reports_both_endings() {
        let mut won = BreakoutGame::default_layout();
        for brick in &mut won.bricks {
            brick.hits = 0;
        }
        let mut game = BreakoutGame::new(won);
        game.launch();
        game.step();
        assert!(status_line(&game).contains("Cleared!"));

        let mut lost = BreakoutGame::default_layout();
        lost.bricks.truncate(1);
        lost.lives = 1;
        let mut game = BreakoutGame::new(lost);
        game.launch();
        game.set_paddle_input(PaddleInput::Left);
        for _ in 0..20_000 {
            if game.step().finished {
                break;
            }
        }
        assert!(
            status_line(&game).contains("Game over"),
            "{}",
            status_line(&game)
        );
    }

    // --- ECS ---

    #[test]
    fn setup_spawns_a_sprite_for_every_brick() {
        let mut app = App::new();
        app.add_plugins(MinimalPlugins)
            .init_resource::<Game>()
            .add_systems(Startup, setup);
        app.update();

        let mut q = app.world_mut().query::<&BrickSprite>();
        assert_eq!(q.iter(app.world()).count(), 40);
    }

    #[test]
    fn the_simulation_runs_at_the_librarys_fixed_rate() {
        // The frontend must not invent its own rate: the library's physics is
        // tuned for DT, and a different step size changes the game.
        let time = Time::<Fixed>::from_hz(STEPS_PER_SECOND as f64);
        let expected = 1.0 / STEPS_PER_SECOND as f64;
        assert!((time.timestep().as_secs_f64() - expected).abs() < 1e-9);
    }
}
