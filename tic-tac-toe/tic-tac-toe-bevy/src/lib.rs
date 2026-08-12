//! Tic-tac-toe rendered with [Bevy](https://bevyengine.org/)'s ECS.
//!
//! This crate is the third frontend over [`tic_tac_toe_lib`], and the point of
//! it is the boundary: Bevy's architecture (entities, components, systems, an
//! engine-owned update loop) could not be less like the terminal frontend's
//! `read_line` loop, and yet **neither one contains a rule of the game**. Every
//! question about legality, whose turn it is, who won, or whether the board is
//! drawn is answered by the library.
//!
//! That is repository goal #4 demonstrated across genuinely different engines,
//! rather than across two variations on a terminal.
//!
//! What this crate *does* own is exactly what a frontend should:
//! - turning a mouse click into a board coordinate ([`world_to_cell`]);
//! - placing sprites on screen ([`cell_to_world`]);
//! - deciding what text to show ([`status_line`]).
//!
//! All three are pure functions, so the parts worth testing need no window.
//!
//! **Controls:** click a cell to play   R — restart.
//!
//! # Example
//! ```no_run
//! use bevy::prelude::*;
//! use tic_tac_toe_bevy::TicTacToePlugin;
//!
//! App::new()
//!     .add_plugins(DefaultPlugins)
//!     .add_plugins(TicTacToePlugin)
//!     .run();
//! ```

use bevy::prelude::*;
use tic_tac_toe_lib::{Board, GameStatus, Player, TicTacToeGame};

/// Bundles every system and resource for the game.
///
/// Add it with `app.add_plugins(TicTacToePlugin)`. It does **not** add
/// `DefaultPlugins`, so the host app stays in control of window and rendering.
pub struct TicTacToePlugin;

impl Plugin for TicTacToePlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<TicTacToeConfig>()
            .init_resource::<Game>()
            .add_systems(Startup, setup)
            .add_systems(Update, (handle_clicks, restart, sync_marks, update_status));
    }
}

// --- Configuration ---

/// Tunable parameters. Override before adding the plugin, e.g.
/// `app.insert_resource(TicTacToeConfig { cell_px: 140.0, ..default() })`.
#[derive(Resource, Clone, Copy, Debug)]
pub struct TicTacToeConfig {
    /// Board width in cells.
    pub cols: usize,
    /// Board height in cells.
    pub rows: usize,
    /// Symbols in a row needed to win.
    pub win_len: usize,
    /// World-space size of one cell, in pixels.
    pub cell_px: f32,
    /// Gap between cell tiles, in pixels.
    pub gap_px: f32,
}

impl Default for TicTacToeConfig {
    fn default() -> Self {
        Self {
            cols: 3,
            rows: 3,
            win_len: 3,
            cell_px: 120.0,
            gap_px: 6.0,
        }
    }
}

impl TicTacToeConfig {
    /// Builds a game matching this configuration.
    pub fn new_game(&self) -> TicTacToeGame {
        TicTacToeGame::new(
            Board::new(self.rows, self.cols),
            vec![
                Player::new("Xavier".to_string(), 'X'),
                Player::new("Olive".to_string(), 'O'),
            ],
            self.win_len,
        )
    }
}

/// The game itself, owned as a resource so systems can read and mutate it.
///
/// All rules live inside; this wrapper exists only so Bevy can store it.
#[derive(Resource, Deref, DerefMut)]
pub struct Game(pub TicTacToeGame);

impl Default for Game {
    fn default() -> Self {
        Self(TicTacToeConfig::default().new_game())
    }
}

// --- Pure frontend math ---

/// Converts a board cell to the world-space centre of its tile.
///
/// Row 0 is drawn at the top, matching how the board reads in the terminal
/// frontend, so the row axis is flipped relative to Bevy's y-up world.
pub fn cell_to_world(row: usize, column: usize, config: &TicTacToeConfig) -> Vec2 {
    let stride = config.cell_px + config.gap_px;
    let origin_x = -(config.cols as f32 - 1.0) * stride / 2.0;
    let origin_y = (config.rows as f32 - 1.0) * stride / 2.0;
    Vec2::new(
        origin_x + column as f32 * stride,
        origin_y - row as f32 * stride,
    )
}

/// Converts a world-space point to the cell containing it.
///
/// Returns `None` when the point falls outside the board or in the gap between
/// tiles, so a click on the background is never mistaken for a move.
pub fn world_to_cell(point: Vec2, config: &TicTacToeConfig) -> Option<(usize, usize)> {
    let stride = config.cell_px + config.gap_px;
    let origin_x = -(config.cols as f32 - 1.0) * stride / 2.0;
    let origin_y = (config.rows as f32 - 1.0) * stride / 2.0;

    let col_f = (point.x - origin_x) / stride;
    let row_f = (origin_y - point.y) / stride;
    let column = col_f.round();
    let row = row_f.round();

    if row < 0.0 || column < 0.0 || row >= config.rows as f32 || column >= config.cols as f32 {
        return None;
    }
    // Reject the gap: the click must land on the tile, not between tiles.
    let half = config.cell_px / 2.0;
    if (col_f - column).abs() * stride > half || (row_f - row).abs() * stride > half {
        return None;
    }
    Some((row as usize, column as usize))
}

/// The status text for the current game state.
pub fn status_line(game: &TicTacToeGame) -> String {
    match game.status() {
        GameStatus::Won(player) => format!(
            "{} ({}) wins!  R to restart",
            player.name(),
            player.symbol()
        ),
        GameStatus::Draw => "It's a draw!  R to restart".to_string(),
        GameStatus::InProgress => {
            let p = game.current_player();
            format!("{}'s turn ({})", p.name(), p.symbol())
        }
    }
}

/// Colour for a player's symbol, so X and O read differently at a glance.
pub fn symbol_color(symbol: char) -> Color {
    match symbol {
        'X' => Color::srgb(0.35, 0.75, 1.0),
        'O' => Color::srgb(1.0, 0.55, 0.35),
        _ => Color::WHITE,
    }
}

/// Converts a window-space cursor position to 2D world coordinates.
pub fn cursor_to_world(
    window: &Window,
    cam: &Camera,
    cam_transform: &GlobalTransform,
) -> Option<Vec2> {
    let cursor = window.cursor_position()?;
    cam.viewport_to_world_2d(cam_transform, cursor).ok()
}

// --- Components ---

/// A board tile, tagged with the cell it represents.
#[derive(Component, Clone, Copy)]
pub struct Cell {
    /// Row index, 0 at the top.
    pub row: usize,
    /// Column index, 0 at the left.
    pub column: usize,
}

/// A placed symbol. Despawned and respawned whenever the board changes.
#[derive(Component)]
pub struct Mark;

/// The status line entity.
#[derive(Component)]
pub struct StatusText;

// --- Systems ---

fn setup(mut commands: Commands, config: Res<TicTacToeConfig>) {
    commands.spawn(Camera2d);

    for row in 0..config.rows {
        for column in 0..config.cols {
            let pos = cell_to_world(row, column, &config);
            commands.spawn((
                Cell { row, column },
                Sprite {
                    color: Color::srgb(0.16, 0.16, 0.20),
                    custom_size: Some(Vec2::splat(config.cell_px)),
                    ..default()
                },
                Transform::from_xyz(pos.x, pos.y, 0.0),
            ));
        }
    }

    commands.spawn((
        StatusText,
        Text::new(""),
        TextFont {
            font_size: 28.0,
            ..default()
        },
        TextColor(Color::WHITE),
        Node {
            position_type: PositionType::Absolute,
            bottom: Val::Px(24.0),
            left: Val::Px(24.0),
            ..default()
        },
    ));
}

/// Turns a left-click into a move, letting the library reject illegal ones.
fn handle_clicks(
    buttons: Res<ButtonInput<MouseButton>>,
    windows: Query<&Window>,
    cameras: Query<(&Camera, &GlobalTransform), With<Camera2d>>,
    config: Res<TicTacToeConfig>,
    mut game: ResMut<Game>,
) {
    if !buttons.just_pressed(MouseButton::Left) {
        return;
    }
    let (Ok(window), Ok((cam, cam_transform))) = (windows.single(), cameras.single()) else {
        return;
    };
    let Some(world) = cursor_to_world(window, cam, cam_transform) else {
        return;
    };
    let Some((row, column)) = world_to_cell(world, &config) else {
        return;
    };

    // No pre-checking: `take_turn` is the single authority on legality, and a
    // rejected move leaves the game untouched.
    let _ = game.take_turn(row, column);
}

fn restart(input: Res<ButtonInput<KeyCode>>, mut game: ResMut<Game>) {
    if input.just_pressed(KeyCode::KeyR) {
        game.reset();
    }
}

/// Redraws the placed symbols whenever the board changes.
fn sync_marks(
    mut commands: Commands,
    game: Res<Game>,
    config: Res<TicTacToeConfig>,
    marks: Query<Entity, With<Mark>>,
) {
    if !game.is_changed() {
        return;
    }
    for entity in &marks {
        commands.entity(entity).despawn();
    }

    for row in 0..game.height() {
        for column in 0..game.width() {
            let Some(symbol) = game.get(row, column) else {
                continue;
            };
            if symbol == tic_tac_toe_lib::EMPTY_SYMBOL {
                continue;
            }
            let pos = cell_to_world(row, column, &config);
            commands.spawn((
                Mark,
                Text2d::new(symbol.to_string()),
                TextFont {
                    font_size: config.cell_px * 0.7,
                    ..default()
                },
                TextColor(symbol_color(symbol)),
                Transform::from_xyz(pos.x, pos.y, 1.0),
            ));
        }
    }
}

fn update_status(game: Res<Game>, mut text: Query<&mut Text, With<StatusText>>) {
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

    fn config() -> TicTacToeConfig {
        TicTacToeConfig::default()
    }

    #[test]
    fn cell_and_world_coordinates_round_trip() {
        let c = config();
        for row in 0..c.rows {
            for column in 0..c.cols {
                let world = cell_to_world(row, column, &c);
                assert_eq!(world_to_cell(world, &c), Some((row, column)));
            }
        }
    }

    #[test]
    fn board_is_centred_on_the_origin() {
        let c = config();
        // With an odd board the middle cell sits exactly at the origin.
        let middle = cell_to_world(1, 1, &c);
        assert!(middle.length() < 1e-5, "expected origin, got {middle:?}");
    }

    #[test]
    fn row_zero_is_drawn_at_the_top() {
        let c = config();
        assert!(cell_to_world(0, 0, &c).y > cell_to_world(2, 0, &c).y);
    }

    #[test]
    fn column_zero_is_drawn_on_the_left() {
        let c = config();
        assert!(cell_to_world(0, 0, &c).x < cell_to_world(0, 2, &c).x);
    }

    #[test]
    fn clicks_outside_the_board_hit_nothing() {
        let c = config();
        assert_eq!(world_to_cell(Vec2::new(10_000.0, 0.0), &c), None);
        assert_eq!(world_to_cell(Vec2::new(0.0, -10_000.0), &c), None);
    }

    #[test]
    fn clicks_in_the_gap_between_tiles_hit_nothing() {
        let c = config();
        // Midway between two tile centres is gap, not tile.
        let a = cell_to_world(0, 0, &c);
        let b = cell_to_world(0, 1, &c);
        assert_eq!(world_to_cell((a + b) / 2.0, &c), None);
    }

    #[test]
    fn a_click_anywhere_on_a_tile_selects_it() {
        let c = config();
        let centre = cell_to_world(2, 1, &c);
        let nudge = c.cell_px / 2.0 - 1.0;
        for offset in [
            Vec2::new(nudge, nudge),
            Vec2::new(-nudge, nudge),
            Vec2::new(nudge, -nudge),
            Vec2::new(-nudge, -nudge),
        ] {
            assert_eq!(world_to_cell(centre + offset, &c), Some((2, 1)));
        }
    }

    #[test]
    fn status_line_reports_whose_turn_it_is() {
        let game = config().new_game();
        assert!(status_line(&game).starts_with("Xavier's turn"));
    }

    #[test]
    fn status_line_reports_the_winner() {
        let mut game = config().new_game();
        for &(row, column) in &[(0, 0), (1, 0), (0, 1), (1, 1), (0, 2)] {
            game.take_turn(row, column).unwrap();
        }
        assert!(status_line(&game).contains("Xavier (X) wins!"));
    }

    #[test]
    fn status_line_reports_a_draw() {
        let mut game = config().new_game();
        let moves = [
            (0, 0),
            (0, 1),
            (0, 2),
            (1, 1),
            (1, 0),
            (1, 2),
            (2, 1),
            (2, 0),
            (2, 2),
        ];
        for &(row, column) in &moves {
            game.take_turn(row, column).unwrap();
        }
        assert!(status_line(&game).contains("draw"));
    }

    #[test]
    fn symbol_colors_differ_between_players() {
        assert_ne!(symbol_color('X'), symbol_color('O'));
    }

    #[test]
    fn config_builds_a_matching_game() {
        let c = TicTacToeConfig {
            cols: 5,
            rows: 4,
            win_len: 4,
            ..config()
        };
        let game = c.new_game();
        assert_eq!(game.width(), 5);
        assert_eq!(game.height(), 4);
        assert_eq!(game.how_many_to_win(), 4);
    }

    // --- ECS ---

    #[test]
    fn setup_spawns_one_tile_per_cell() {
        let mut app = App::new();
        app.add_plugins(MinimalPlugins)
            .init_resource::<TicTacToeConfig>()
            .add_systems(Startup, setup);
        app.update();

        let mut q = app.world_mut().query::<&Cell>();
        assert_eq!(q.iter(app.world()).count(), 9);
    }

    #[test]
    fn tiles_cover_every_distinct_cell() {
        let mut app = App::new();
        app.add_plugins(MinimalPlugins)
            .init_resource::<TicTacToeConfig>()
            .add_systems(Startup, setup);
        app.update();

        let mut q = app.world_mut().query::<&Cell>();
        let mut seen: Vec<(usize, usize)> =
            q.iter(app.world()).map(|c| (c.row, c.column)).collect();
        seen.sort_unstable();
        seen.dedup();
        assert_eq!(seen.len(), 9);
    }

    #[test]
    fn restart_clears_the_board() {
        let mut game = config().new_game();
        game.take_turn(0, 0).unwrap();
        assert_eq!(game.turn_count(), 1);
        game.reset();
        assert_eq!(game.turn_count(), 0);
        assert!(status_line(&game).starts_with("Xavier's turn"));
    }
}
