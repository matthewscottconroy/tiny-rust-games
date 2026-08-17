//! Snake rendered with [Bevy](https://bevyengine.org/)'s ECS.
//!
//! This is the frontend that motivated [`snake_lib`]'s design. Bevy calls its
//! systems once per rendered frame — 60, 144, or whatever the monitor does —
//! but Snake must move at a fixed, slow rate or it is unplayable. Something has
//! to reconcile those two clocks.
//!
//! The library does not, deliberately: it exposes `step()` and nothing about
//! time. This crate owns the reconciliation, and even that is not hand-rolled —
//! [`snake_lib::Ticker`] does it, because every frontend needs the same thing:
//!
//! ```ignore
//! for _ in 0..ticker.accumulate(time.delta_secs()) {
//!     game.step();
//! }
//! ```
//!
//! That loop is the entire difference between this frontend and the terminal
//! one. Neither contains a rule of the game.
//!
//! **Controls:** arrows or WASD to steer   R to restart.
//!
//! # Example
//! ```no_run
//! use bevy::prelude::*;
//! use snake_bevy::SnakePlugin;
//!
//! App::new()
//!     .add_plugins(DefaultPlugins)
//!     .add_plugins(SnakePlugin)
//!     .run();
//! ```

use bevy::prelude::*;
use snake_lib::{Coord, Direction, GameStatus, SnakeGame, StepOutcome, Ticker};

/// Bundles every system and resource for the game.
///
/// Add it with `app.add_plugins(SnakePlugin)`. It does **not** add
/// `DefaultPlugins`, so the host app stays in control of window and rendering.
pub struct SnakePlugin;

impl Plugin for SnakePlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<SnakeConfig>()
            .init_resource::<Game>()
            .init_resource::<Tick>()
            .add_systems(Startup, setup)
            .add_systems(
                Update,
                (read_input, advance, restart, redraw, update_hud).chain(),
            );
    }
}

// --- Configuration ---

/// Tunable parameters. Override before adding the plugin, e.g.
/// `app.insert_resource(SnakeConfig { steps_per_second: 12.0, ..default() })`.
#[derive(Resource, Clone, Copy, Debug)]
pub struct SnakeConfig {
    /// Board width in cells.
    pub cols: i32,
    /// Board height in cells.
    pub rows: i32,
    /// World-space size of one cell, in pixels.
    pub cell_px: f32,
    /// How many simulation steps run per second, independent of frame rate.
    pub steps_per_second: f32,
    /// Seed for food placement; the same seed replays the same game.
    pub seed: u64,
}

impl Default for SnakeConfig {
    fn default() -> Self {
        Self {
            cols: 24,
            rows: 18,
            cell_px: 26.0,
            steps_per_second: 9.0,
            seed: 0x5EED,
        }
    }
}

impl SnakeConfig {
    /// Builds a game matching this configuration.
    pub fn new_game(&self) -> SnakeGame {
        SnakeGame::new(self.cols, self.rows, self.seed)
    }

    /// World-space centre of a board cell.
    ///
    /// Row 0 is the top of the board, so the y axis is flipped relative to
    /// Bevy's y-up world.
    pub fn cell_to_world(&self, c: Coord) -> Vec2 {
        let origin_x = -(self.cols as f32 - 1.0) * self.cell_px / 2.0;
        let origin_y = (self.rows as f32 - 1.0) * self.cell_px / 2.0;
        Vec2::new(
            origin_x + c.x as f32 * self.cell_px,
            origin_y - c.y as f32 * self.cell_px,
        )
    }
}

/// The game itself, owned as a resource so systems can read and mutate it.
#[derive(Resource, Deref, DerefMut)]
pub struct Game(pub SnakeGame);

impl Default for Game {
    fn default() -> Self {
        Self(SnakeConfig::default().new_game())
    }
}

/// Converts frame time into simulation steps.
#[derive(Resource, Deref, DerefMut)]
pub struct Tick(pub Ticker);

impl Default for Tick {
    fn default() -> Self {
        Self(Ticker::new(SnakeConfig::default().steps_per_second))
    }
}

// --- Pure helpers ---

/// Maps a set of held keys to a steering request.
///
/// Returns `None` when nothing relevant is pressed. Pure so the key mapping can
/// be tested without a window.
pub fn direction_from_keys(up: bool, down: bool, left: bool, right: bool) -> Option<Direction> {
    // Checked in a fixed order, so pressing two keys in one frame is resolved
    // deterministically rather than by whichever the ECS happened to see first.
    if up {
        Some(Direction::Up)
    } else if down {
        Some(Direction::Down)
    } else if left {
        Some(Direction::Left)
    } else if right {
        Some(Direction::Right)
    } else {
        None
    }
}

/// The status line for the current game state.
pub fn status_line(game: &SnakeGame) -> String {
    match game.status() {
        GameStatus::Running => format!("Score {}   Length {}", game.score(), game.len()),
        GameStatus::Dead(cause) => {
            format!("Score {} - died: {cause:?}.  R to restart", game.score())
        }
        GameStatus::Won => format!("Board full! Score {}.  R to restart", game.score()),
    }
}

// --- Components ---

/// A drawn snake segment or food sprite. Despawned and respawned each redraw.
#[derive(Component)]
pub struct Drawn;

/// The status text entity.
#[derive(Component)]
pub struct StatusText;

// --- Systems ---

fn setup(mut commands: Commands, config: Res<SnakeConfig>) {
    commands.spawn(Camera2d);

    // Board backdrop, so the play area is visible before anything moves.
    commands.spawn((
        Sprite {
            color: Color::srgb(0.10, 0.11, 0.13),
            custom_size: Some(Vec2::new(
                config.cols as f32 * config.cell_px,
                config.rows as f32 * config.cell_px,
            )),
            ..default()
        },
        Transform::from_xyz(0.0, 0.0, -1.0),
    ));

    commands.spawn((
        StatusText,
        Text::new(""),
        TextFont {
            font_size: 22.0,
            ..default()
        },
        TextColor(Color::WHITE),
        Node {
            position_type: PositionType::Absolute,
            bottom: Val::Px(16.0),
            left: Val::Px(16.0),
            ..default()
        },
    ));
}

/// Queues a steering request; the library decides whether it is legal.
fn read_input(keys: Res<ButtonInput<KeyCode>>, mut game: ResMut<Game>) {
    let direction = direction_from_keys(
        keys.any_just_pressed([KeyCode::ArrowUp, KeyCode::KeyW]),
        keys.any_just_pressed([KeyCode::ArrowDown, KeyCode::KeyS]),
        keys.any_just_pressed([KeyCode::ArrowLeft, KeyCode::KeyA]),
        keys.any_just_pressed([KeyCode::ArrowRight, KeyCode::KeyD]),
    );
    if let Some(direction) = direction {
        // A refused turn (a reversal) is simply ignored — the rule lives in the
        // library, so this frontend does not need to know what it is.
        game.queue_turn(direction);
    }
}

/// Runs however many simulation steps this frame's elapsed time is worth.
///
/// This is the whole of the frame-rate/simulation-rate reconciliation.
fn advance(time: Res<Time>, mut game: ResMut<Game>, mut tick: ResMut<Tick>) {
    if game.is_over() {
        return;
    }
    for _ in 0..tick.accumulate(time.delta_secs()) {
        match game.step() {
            StepOutcome::Died(_) | StepOutcome::Won => break,
            _ => {}
        }
    }
}

fn restart(
    keys: Res<ButtonInput<KeyCode>>,
    config: Res<SnakeConfig>,
    mut game: ResMut<Game>,
    mut tick: ResMut<Tick>,
) {
    if keys.just_pressed(KeyCode::KeyR) {
        // Vary the seed so a restart is a new game, not a replay of the last.
        let next_seed = config.seed.wrapping_add(game.ticks()).wrapping_add(1);
        game.reset(next_seed);
        **tick = Ticker::new(config.steps_per_second);
    }
}

/// Redraws the snake and food whenever the game changed.
fn redraw(
    mut commands: Commands,
    game: Res<Game>,
    config: Res<SnakeConfig>,
    drawn: Query<Entity, With<Drawn>>,
) {
    if !game.is_changed() {
        return;
    }
    for entity in &drawn {
        commands.entity(entity).despawn();
    }

    let head = game.head();
    for cell in game.body() {
        let pos = config.cell_to_world(cell);
        let color = if cell == head {
            Color::srgb(0.55, 0.95, 0.55)
        } else {
            Color::srgb(0.25, 0.70, 0.35)
        };
        commands.spawn((
            Drawn,
            Sprite {
                color,
                custom_size: Some(Vec2::splat(config.cell_px - 2.0)),
                ..default()
            },
            Transform::from_xyz(pos.x, pos.y, 1.0),
        ));
    }

    if let Some(food) = game.food() {
        let pos = config.cell_to_world(food);
        commands.spawn((
            Drawn,
            Sprite {
                color: Color::srgb(0.95, 0.45, 0.35),
                custom_size: Some(Vec2::splat(config.cell_px * 0.6)),
                ..default()
            },
            Transform::from_xyz(pos.x, pos.y, 1.0),
        ));
    }
}

fn update_hud(game: Res<Game>, mut text: Query<&mut Text, With<StatusText>>) {
    if !game.is_changed() {
        return;
    }
    for mut t in &mut text {
        **t = status_line(&game);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn config() -> SnakeConfig {
        SnakeConfig::default()
    }

    #[test]
    fn keys_map_to_directions() {
        assert_eq!(
            direction_from_keys(true, false, false, false),
            Some(Direction::Up)
        );
        assert_eq!(
            direction_from_keys(false, true, false, false),
            Some(Direction::Down)
        );
        assert_eq!(
            direction_from_keys(false, false, true, false),
            Some(Direction::Left)
        );
        assert_eq!(
            direction_from_keys(false, false, false, true),
            Some(Direction::Right)
        );
    }

    #[test]
    fn no_keys_means_no_steering_request() {
        assert_eq!(direction_from_keys(false, false, false, false), None);
    }

    #[test]
    fn simultaneous_keys_resolve_deterministically() {
        // Whatever the order rule is, it must not depend on ECS iteration order.
        assert_eq!(
            direction_from_keys(true, true, true, true),
            Some(Direction::Up)
        );
        assert_eq!(
            direction_from_keys(false, true, true, false),
            Some(Direction::Down)
        );
    }

    #[test]
    fn cell_to_world_places_row_zero_at_the_top() {
        let c = config();
        assert!(c.cell_to_world(Coord::new(0, 0)).y > c.cell_to_world(Coord::new(0, 5)).y);
    }

    #[test]
    fn cell_to_world_places_column_zero_on_the_left() {
        let c = config();
        assert!(c.cell_to_world(Coord::new(0, 0)).x < c.cell_to_world(Coord::new(5, 0)).x);
    }

    #[test]
    fn the_board_is_centred_on_the_origin() {
        // Opposite corners must be mirror images.
        let c = SnakeConfig {
            cols: 5,
            rows: 5,
            ..config()
        };
        let a = c.cell_to_world(Coord::new(0, 0));
        let b = c.cell_to_world(Coord::new(4, 4));
        assert!((a.x + b.x).abs() < 1e-4, "{a:?} {b:?}");
        assert!((a.y + b.y).abs() < 1e-4, "{a:?} {b:?}");
    }

    #[test]
    fn adjacent_cells_are_one_cell_apart() {
        let c = config();
        let a = c.cell_to_world(Coord::new(1, 1));
        let b = c.cell_to_world(Coord::new(2, 1));
        assert!((b.x - a.x - c.cell_px).abs() < 1e-4);
    }

    #[test]
    fn config_builds_a_matching_game() {
        let c = SnakeConfig {
            cols: 11,
            rows: 7,
            ..config()
        };
        let game = c.new_game();
        assert_eq!((game.width(), game.height()), (11, 7));
    }

    #[test]
    fn status_line_reports_score_while_running() {
        let game = config().new_game();
        assert!(status_line(&game).contains("Score 0"));
    }

    #[test]
    fn status_line_reports_death_and_how_to_restart() {
        let mut game = SnakeGame::new(4, 4, 1);
        while !game.is_over() {
            game.step();
        }
        let line = status_line(&game);
        assert!(line.contains("died"), "{line}");
        assert!(line.contains("R to restart"), "{line}");
    }

    // --- ECS ---

    #[test]
    fn setup_spawns_a_status_line() {
        let mut app = App::new();
        app.add_plugins(MinimalPlugins)
            .init_resource::<SnakeConfig>()
            .add_systems(Startup, setup);
        app.update();

        let mut q = app.world_mut().query::<&StatusText>();
        assert_eq!(q.iter(app.world()).count(), 1);
    }

    #[test]
    fn the_simulation_rate_is_independent_of_frame_rate() {
        // The point of the whole design: 60 tiny frames and 6 big ones covering
        // the same wall-clock second must advance the game equally.
        let steps_per_second = 10.0;
        let mut fast = Ticker::new(steps_per_second);
        let mut slow = Ticker::new(steps_per_second);

        let fast_steps: u32 = (0..60).map(|_| fast.accumulate(1.0 / 60.0)).sum();
        let slow_steps: u32 = (0..6).map(|_| slow.accumulate(1.0 / 6.0)).sum();

        assert_eq!(fast_steps, slow_steps);
        assert_eq!(fast_steps, 10);
    }
}
