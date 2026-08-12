//! Tic-tac-toe rendered with [Godot](https://godotengine.org/)'s scene tree.
//!
//! This is the fourth frontend over [`tic_tac_toe_lib`], and — together with
//! the Bevy one — the point of it is the contrast. Godot drives the game
//! through node lifecycle callbacks (`ready`, `draw`, `input`) with the engine
//! owning the scene graph, which is nothing like Bevy's ECS or the terminal
//! frontend's `read_line` loop. **None of the three contains a rule of the
//! game.** Legality, turn order, the winner, and draw detection all come from
//! the library.
//!
//! That is repository goal #4 demonstrated across genuinely different engine
//! architectures, which is what goal #2 asks for.
//!
//! What this crate owns is only presentation and input:
//! - mapping a click to a board cell ([`world_to_cell`]) and back
//!   ([`cell_to_world`]);
//! - deciding what the status line says ([`status_line`]).
//!
//! Both are free functions over plain numbers, so `cargo test` exercises them
//! without starting Godot.
//!
//! **Controls:** left-click a cell to play   R — restart.

use godot::classes::{INode2D, Label, Node2D};
use godot::classes::{InputEvent, InputEventKey, InputEventMouseButton};
use godot::global::Key;
use godot::global::MouseButton;
use godot::prelude::*;

use tic_tac_toe_lib::{Board, EMPTY_SYMBOL, GameStatus, Player, TicTacToeGame};

// ─── Extension entry point ────────────────────────────────────────────────────

struct TicTacToeExt;

#[gdextension]
unsafe impl ExtensionLibrary for TicTacToeExt {}

// ─── Layout constants ─────────────────────────────────────────────────────────

/// Side length of one board cell, in pixels.
pub const CELL_PX: f32 = 120.0;
/// Gap between cells, in pixels.
pub const GAP_PX: f32 = 6.0;
/// Board width in cells.
pub const COLS: usize = 3;
/// Board height in cells.
pub const ROWS: usize = 3;
/// Symbols in a row needed to win.
pub const WIN_LEN: usize = 3;

/// Distance between the centres of adjacent cells.
pub const STRIDE: f32 = CELL_PX + GAP_PX;

// ─── Pure frontend math ───────────────────────────────────────────────────────

/// Converts a board cell to the local-space centre of its tile.
///
/// Row 0 is drawn at the top, matching how the board reads in the terminal
/// frontend.
pub fn cell_to_world(row: usize, column: usize) -> (f32, f32) {
    let origin_x = -(COLS as f32 - 1.0) * STRIDE / 2.0;
    let origin_y = -(ROWS as f32 - 1.0) * STRIDE / 2.0;
    (
        origin_x + column as f32 * STRIDE,
        origin_y + row as f32 * STRIDE,
    )
}

/// Converts a local-space point to the cell containing it.
///
/// Returns `None` when the point lands outside the board or in the gap between
/// tiles, so a stray click is never mistaken for a move.
pub fn world_to_cell(x: f32, y: f32) -> Option<(usize, usize)> {
    let origin_x = -(COLS as f32 - 1.0) * STRIDE / 2.0;
    let origin_y = -(ROWS as f32 - 1.0) * STRIDE / 2.0;

    let col_f = (x - origin_x) / STRIDE;
    let row_f = (y - origin_y) / STRIDE;
    let column = col_f.round();
    let row = row_f.round();

    if row < 0.0 || column < 0.0 || row >= ROWS as f32 || column >= COLS as f32 {
        return None;
    }
    let half = CELL_PX / 2.0;
    if (col_f - column).abs() * STRIDE > half || (row_f - row).abs() * STRIDE > half {
        return None;
    }
    Some((row as usize, column as usize))
}

/// Builds the game this demo plays.
pub fn new_game() -> TicTacToeGame {
    TicTacToeGame::new(
        Board::new(ROWS, COLS),
        vec![
            Player::new("Xavier".to_string(), 'X'),
            Player::new("Olive".to_string(), 'O'),
        ],
        WIN_LEN,
    )
}

/// The status text for the current game state.
pub fn status_line(game: &TicTacToeGame) -> String {
    match game.status() {
        GameStatus::Won(player) => {
            format!(
                "{} ({}) wins!  R to restart",
                player.name(),
                player.symbol()
            )
        }
        GameStatus::Draw => "It's a draw!  R to restart".to_string(),
        GameStatus::InProgress => {
            let p = game.current_player();
            format!("{}'s turn ({})", p.name(), p.symbol())
        }
    }
}

// ─── TicTacToeBoard node ──────────────────────────────────────────────────────

/// A `Node2D` that draws the board and turns clicks into moves.
///
/// Add it as the root of a scene and run. It creates its own status `Label`
/// child in `ready`, so the scene needs nothing else.
#[derive(GodotClass)]
#[class(base=Node2D)]
pub struct TicTacToeBoard {
    game: TicTacToeGame,
    status: Option<Gd<Label>>,
    base: Base<Node2D>,
}

#[godot_api]
impl INode2D for TicTacToeBoard {
    fn init(base: Base<Node2D>) -> Self {
        // Defaults only — the scene tree does not exist yet.
        Self {
            game: new_game(),
            status: None,
            base,
        }
    }

    fn ready(&mut self) {
        let mut label = Label::new_alloc();
        label.set_text(&status_line(&self.game));
        label.set_position(Vector2::new(-220.0, 220.0));
        self.base_mut().add_child(&label);
        self.status = Some(label);
    }

    fn input(&mut self, event: Gd<InputEvent>) {
        if let Ok(mb) = event.clone().try_cast::<InputEventMouseButton>()
            && mb.get_button_index() == MouseButton::LEFT
            && mb.is_pressed()
        {
            let local = self.base().to_local(mb.get_position());
            if let Some((row, column)) = world_to_cell(local.x, local.y) {
                // No pre-checking: the library is the single authority on
                // legality, and a rejected move leaves the game untouched.
                let _ = self.game.take_turn(row, column);
                self.refresh();
            }
        }

        if let Ok(key) = event.try_cast::<InputEventKey>()
            && key.is_pressed()
            && key.get_keycode() == Key::R
        {
            self.game.reset();
            self.refresh();
        }
    }

    fn draw(&mut self) {
        // Snapshot the board first: the draw calls below need `&mut self`, so
        // the borrow of `self.game` has to end before the loop starts.
        let cells: Vec<(usize, usize, char)> = (0..ROWS)
            .flat_map(|row| (0..COLS).map(move |column| (row, column)))
            .map(|(row, column)| {
                let symbol = self.game.get(row, column).unwrap_or(EMPTY_SYMBOL);
                (row, column, symbol)
            })
            .collect();

        for (row, column, symbol) in cells {
            let (cx, cy) = cell_to_world(row, column);
            let rect = Rect2::new(
                Vector2::new(cx - CELL_PX / 2.0, cy - CELL_PX / 2.0),
                Vector2::new(CELL_PX, CELL_PX),
            );
            self.base_mut()
                .draw_rect(rect, Color::from_rgb(0.16, 0.16, 0.20));

            match symbol {
                'X' => self.draw_cross(cx, cy),
                'O' => self.draw_nought(cx, cy),
                _ => {}
            }
        }
    }
}

#[godot_api]
impl TicTacToeBoard {
    /// The current status text (callable from GDScript).
    #[func]
    pub fn status_text(&self) -> GString {
        GString::from(status_line(&self.game).as_str())
    }

    /// Restarts the game (callable from GDScript).
    #[func]
    pub fn restart(&mut self) {
        self.game.reset();
        self.refresh();
    }

    /// Repaints the board and updates the status label.
    fn refresh(&mut self) {
        let text = status_line(&self.game);
        if let Some(label) = self.status.as_mut() {
            label.set_text(&text);
        }
        self.base_mut().queue_redraw();
    }

    fn draw_cross(&mut self, cx: f32, cy: f32) {
        let arm = CELL_PX * 0.28;
        let color = Color::from_rgb(0.35, 0.75, 1.0);
        self.base_mut()
            .draw_line_ex(
                Vector2::new(cx - arm, cy - arm),
                Vector2::new(cx + arm, cy + arm),
                color,
            )
            .width(8.0)
            .done();
        self.base_mut()
            .draw_line_ex(
                Vector2::new(cx + arm, cy - arm),
                Vector2::new(cx - arm, cy + arm),
                color,
            )
            .width(8.0)
            .done();
    }

    fn draw_nought(&mut self, cx: f32, cy: f32) {
        // draw_arc takes 6 args in gdext 0.5: centre, radius, start, end, points, colour.
        self.base_mut().draw_arc(
            Vector2::new(cx, cy),
            CELL_PX * 0.3,
            0.0,
            std::f32::consts::TAU,
            48,
            Color::from_rgb(1.0, 0.55, 0.35),
        );
    }
}

// ─── Tests ────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cell_and_world_coordinates_round_trip() {
        for row in 0..ROWS {
            for column in 0..COLS {
                let (x, y) = cell_to_world(row, column);
                assert_eq!(world_to_cell(x, y), Some((row, column)));
            }
        }
    }

    #[test]
    fn board_is_centred_on_the_origin() {
        let (x, y) = cell_to_world(1, 1);
        assert!(
            x.abs() < 1e-5 && y.abs() < 1e-5,
            "expected origin, got ({x}, {y})"
        );
    }

    #[test]
    fn row_zero_is_drawn_at_the_top() {
        // Godot's y axis points down, so the top row has the smaller y.
        assert!(cell_to_world(0, 0).1 < cell_to_world(2, 0).1);
    }

    #[test]
    fn column_zero_is_drawn_on_the_left() {
        assert!(cell_to_world(0, 0).0 < cell_to_world(0, 2).0);
    }

    #[test]
    fn clicks_outside_the_board_hit_nothing() {
        assert_eq!(world_to_cell(10_000.0, 0.0), None);
        assert_eq!(world_to_cell(0.0, -10_000.0), None);
    }

    #[test]
    fn clicks_in_the_gap_between_tiles_hit_nothing() {
        let (ax, ay) = cell_to_world(0, 0);
        let (bx, by) = cell_to_world(0, 1);
        assert_eq!(world_to_cell((ax + bx) / 2.0, (ay + by) / 2.0), None);
    }

    #[test]
    fn a_click_anywhere_on_a_tile_selects_it() {
        let (cx, cy) = cell_to_world(2, 1);
        let nudge = CELL_PX / 2.0 - 1.0;
        for (dx, dy) in [
            (nudge, nudge),
            (-nudge, nudge),
            (nudge, -nudge),
            (-nudge, -nudge),
        ] {
            assert_eq!(world_to_cell(cx + dx, cy + dy), Some((2, 1)));
        }
    }

    #[test]
    fn status_line_reports_whose_turn_it_is() {
        assert!(status_line(&new_game()).starts_with("Xavier's turn"));
    }

    #[test]
    fn status_line_reports_the_winner() {
        let mut game = new_game();
        for &(row, column) in &[(0, 0), (1, 0), (0, 1), (1, 1), (0, 2)] {
            game.take_turn(row, column).unwrap();
        }
        assert!(status_line(&game).contains("Xavier (X) wins!"));
    }

    #[test]
    fn status_line_reports_a_draw() {
        let mut game = new_game();
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
    fn new_game_matches_the_declared_layout() {
        let game = new_game();
        assert_eq!(game.width(), COLS);
        assert_eq!(game.height(), ROWS);
        assert_eq!(game.how_many_to_win(), WIN_LEN);
    }

    #[test]
    fn the_library_rejects_illegal_moves_so_the_frontend_need_not_check() {
        let mut game = new_game();
        game.take_turn(0, 0).unwrap();
        assert!(game.take_turn(0, 0).is_err(), "occupied cell");
        assert!(game.take_turn(9, 9).is_err(), "off the board");
        assert_eq!(game.turn_count(), 1);
    }
}
