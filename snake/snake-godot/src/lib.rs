//! Snake rendered with [Godot](https://godotengine.org/)'s scene tree.
//!
//! Teaches: driving a fixed-rate simulation from Godot's variable-rate
//! `process(delta)` callback, using [`snake_lib::Ticker`] to convert frame time
//! into whole simulation steps, and drawing the result with `draw_rect`.
//!
//! Together with the Bevy frontend this is the point of `snake-lib`. Godot
//! hands you a `delta` and expects you to do something sensible with it; Bevy
//! hands you `Time::delta` from a system. Both reduce to the same three lines:
//!
//! ```text
//! for _ in 0..ticker.accumulate(delta) {
//!     game.step();
//! }
//! ```
//!
//! Neither frontend contains a rule of the game, and neither can disagree with
//! the other about how fast the snake moves, because neither one decides.
//!
//! **Controls:** arrows or WASD to steer   R to restart.

use godot::classes::{INode2D, InputEvent, InputEventKey, Label, Node2D};
use godot::global::Key;
use godot::prelude::*;

use snake_lib::{Coord, Direction, GameStatus, SnakeGame, Ticker};

// ─── Extension entry point ────────────────────────────────────────────────────

struct SnakeExt;

#[gdextension]
unsafe impl ExtensionLibrary for SnakeExt {}

// ─── Layout ───────────────────────────────────────────────────────────────────

/// Board width in cells.
pub const COLS: i32 = 24;
/// Board height in cells.
pub const ROWS: i32 = 18;
/// Side length of one cell, in pixels.
pub const CELL_PX: f32 = 26.0;
/// Simulation steps per second, independent of frame rate.
pub const STEPS_PER_SECOND: f32 = 9.0;

// ─── Pure functions ───────────────────────────────────────────────────────────

/// Top-left corner of a board cell, in local space.
///
/// The board is centred on the node's origin, and row 0 is the top.
pub fn cell_origin(c: Coord) -> (f32, f32) {
    let left = -(COLS as f32) * CELL_PX / 2.0;
    let top = -(ROWS as f32) * CELL_PX / 2.0;
    (left + c.x as f32 * CELL_PX, top + c.y as f32 * CELL_PX)
}

/// Maps a Godot keycode to a steering direction, if it is one.
pub fn direction_from_key(key: Key) -> Option<Direction> {
    match key {
        Key::UP | Key::W => Some(Direction::Up),
        Key::DOWN | Key::S => Some(Direction::Down),
        Key::LEFT | Key::A => Some(Direction::Left),
        Key::RIGHT | Key::D => Some(Direction::Right),
        _ => None,
    }
}

/// The status text for the current game state.
pub fn status_line(game: &SnakeGame) -> String {
    match game.status() {
        GameStatus::Running => format!("Score {}   Length {}", game.score(), game.len()),
        GameStatus::Dead(cause) => {
            format!("Score {} - died: {cause:?}.  R to restart", game.score())
        }
        GameStatus::Won => format!("Board full! Score {}.  R to restart", game.score()),
    }
}

// ─── SnakeBoard node ──────────────────────────────────────────────────────────

/// A `Node2D` that runs and draws a game of Snake.
///
/// Add it as the root of a scene and run; it creates its own status `Label`
/// child in `ready`, so the scene needs nothing else.
#[derive(GodotClass)]
#[class(base=Node2D)]
pub struct SnakeBoard {
    game: SnakeGame,
    ticker: Ticker,
    status: Option<Gd<Label>>,
    seed: u64,
    base: Base<Node2D>,
}

#[godot_api]
impl INode2D for SnakeBoard {
    fn init(base: Base<Node2D>) -> Self {
        // Defaults only — the scene tree does not exist yet.
        Self {
            game: SnakeGame::new(COLS, ROWS, 0x5EED),
            ticker: Ticker::new(STEPS_PER_SECOND),
            status: None,
            seed: 0x5EED,
            base,
        }
    }

    fn ready(&mut self) {
        let mut label = Label::new_alloc();
        label.set_text(&status_line(&self.game));
        label.set_position(Vector2::new(
            -(COLS as f32) * CELL_PX / 2.0,
            ROWS as f32 * CELL_PX / 2.0 + 8.0,
        ));
        self.base_mut().add_child(&label);
        self.status = Some(label);
    }

    fn process(&mut self, delta: f64) {
        if self.game.is_over() {
            return;
        }
        // The whole frame-rate reconciliation. Godot's delta varies; the
        // simulation rate does not.
        let steps = self.ticker.accumulate(delta as f32);
        if steps == 0 {
            return;
        }
        for _ in 0..steps {
            self.game.step();
            if self.game.is_over() {
                break;
            }
        }
        self.refresh();
    }

    fn input(&mut self, event: Gd<InputEvent>) {
        let Ok(key_event) = event.try_cast::<InputEventKey>() else {
            return;
        };
        if !key_event.is_pressed() {
            return;
        }
        let key = key_event.get_keycode();

        if key == Key::R {
            self.seed = self.seed.wrapping_add(self.game.ticks()).wrapping_add(1);
            let seed = self.seed;
            self.game.reset(seed);
            self.ticker = Ticker::new(STEPS_PER_SECOND);
            self.refresh();
            return;
        }
        if let Some(direction) = direction_from_key(key) {
            // A refused turn is simply ignored; the rule lives in the library.
            self.game.queue_turn(direction);
        }
    }

    fn draw(&mut self) {
        // Snapshot first: the draw calls below need `&mut self`.
        let body: Vec<Coord> = self.game.body().collect();
        let head = self.game.head();
        let food = self.game.food();

        let board = Rect2::new(
            Vector2::new(
                -(COLS as f32) * CELL_PX / 2.0,
                -(ROWS as f32) * CELL_PX / 2.0,
            ),
            Vector2::new(COLS as f32 * CELL_PX, ROWS as f32 * CELL_PX),
        );
        self.base_mut()
            .draw_rect(board, Color::from_rgb(0.10, 0.11, 0.13));

        for cell in body {
            let (x, y) = cell_origin(cell);
            let rect = Rect2::new(
                Vector2::new(x + 1.0, y + 1.0),
                Vector2::new(CELL_PX - 2.0, CELL_PX - 2.0),
            );
            let color = if cell == head {
                Color::from_rgb(0.55, 0.95, 0.55)
            } else {
                Color::from_rgb(0.25, 0.70, 0.35)
            };
            self.base_mut().draw_rect(rect, color);
        }

        if let Some(food) = food {
            let (x, y) = cell_origin(food);
            let inset = CELL_PX * 0.2;
            let rect = Rect2::new(
                Vector2::new(x + inset, y + inset),
                Vector2::new(CELL_PX - inset * 2.0, CELL_PX - inset * 2.0),
            );
            self.base_mut()
                .draw_rect(rect, Color::from_rgb(0.95, 0.45, 0.35));
        }
    }
}

#[godot_api]
impl SnakeBoard {
    /// The current score (callable from GDScript).
    #[func]
    pub fn score(&self) -> i32 {
        self.game.score() as i32
    }

    /// Whether the game has ended (callable from GDScript).
    #[func]
    pub fn is_over(&self) -> bool {
        self.game.is_over()
    }

    /// Repaints the board and updates the status label.
    fn refresh(&mut self) {
        let text = status_line(&self.game);
        if let Some(label) = self.status.as_mut() {
            label.set_text(&text);
        }
        self.base_mut().queue_redraw();
    }
}

// ─── Tests ────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cell_origin_places_row_zero_at_the_top() {
        assert!(cell_origin(Coord::new(0, 0)).1 < cell_origin(Coord::new(0, 5)).1);
    }

    #[test]
    fn cell_origin_places_column_zero_on_the_left() {
        assert!(cell_origin(Coord::new(0, 0)).0 < cell_origin(Coord::new(5, 0)).0);
    }

    #[test]
    fn adjacent_cells_are_one_cell_apart() {
        let a = cell_origin(Coord::new(1, 1));
        let b = cell_origin(Coord::new(2, 1));
        assert!((b.0 - a.0 - CELL_PX).abs() < 1e-4);
    }

    #[test]
    fn the_board_is_centred_on_the_origin() {
        let first = cell_origin(Coord::new(0, 0));
        let last = cell_origin(Coord::new(COLS - 1, ROWS - 1));
        // Left edge and the far cell's right edge are mirror images.
        assert!((first.0 + (last.0 + CELL_PX)).abs() < 1e-3);
        assert!((first.1 + (last.1 + CELL_PX)).abs() < 1e-3);
    }

    #[test]
    fn arrow_and_wasd_keys_map_to_the_same_directions() {
        assert_eq!(direction_from_key(Key::UP), Some(Direction::Up));
        assert_eq!(direction_from_key(Key::W), Some(Direction::Up));
        assert_eq!(direction_from_key(Key::DOWN), Some(Direction::Down));
        assert_eq!(direction_from_key(Key::S), Some(Direction::Down));
        assert_eq!(direction_from_key(Key::LEFT), Some(Direction::Left));
        assert_eq!(direction_from_key(Key::A), Some(Direction::Left));
        assert_eq!(direction_from_key(Key::RIGHT), Some(Direction::Right));
        assert_eq!(direction_from_key(Key::D), Some(Direction::Right));
    }

    #[test]
    fn other_keys_are_not_steering() {
        for key in [Key::R, Key::ESCAPE, Key::SPACE, Key::KEY_1] {
            assert_eq!(direction_from_key(key), None, "{key:?}");
        }
    }

    #[test]
    fn status_line_reports_score_while_running() {
        let game = SnakeGame::new(COLS, ROWS, 1);
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

    #[test]
    fn the_simulation_rate_is_independent_of_frame_rate() {
        // Godot's `delta` varies frame to frame; the game must not.
        let mut jittery = Ticker::new(10.0);
        let mut steady = Ticker::new(10.0);

        // One second delivered as uneven frames, and as even ones.
        let uneven = [0.004, 0.030, 0.016, 0.100, 0.050, 0.300, 0.200, 0.300];
        let jittery_steps: u32 = uneven.iter().map(|d| jittery.accumulate(*d)).sum();
        let steady_steps: u32 = (0..100).map(|_| steady.accumulate(0.01)).sum();

        // Within one step, not identical: f32 frame deltas are mostly
        // unrepresentable, so a hundred 0.01s sum to slightly under a second.
        assert!(
            jittery_steps.abs_diff(steady_steps) <= 1,
            "{jittery_steps} vs {steady_steps}"
        );
        assert!((9..=10).contains(&jittery_steps), "got {jittery_steps}");
    }
}
