//! Tic-tac-toe with **no game engine at all**.
//!
//! The other frontends make the engine-agnostic argument by using different
//! engines: Bevy's ECS, Godot's scene tree, bracket-lib's console, a terminal
//! loop. This one makes it by using none. It is a canvas, a click handler and
//! about a hundred lines of drawing code, talking to exactly the same
//! [`tic_tac_toe_lib`] the other four use, with no change to that library and
//! no rule of the game in this file.
//!
//! That is the strongest form of goal #4 available. A rules crate that works
//! under four engines might just be a crate that abstracts over four engines.
//! One that also works with no engine, in a browser, against a raw 2D context,
//! is a crate that genuinely does not know what an engine is.
//!
//! # What is actually here
//!
//! - [`geometry`] — mapping the board onto pixels and a click back onto a cell.
//!   The only arithmetic in the frontend, kept free of `web-sys` so it is
//!   testable on the host, which is where all of this module's tests live.
//! - a `start` function that finds the canvas, draws, and installs one listener.
//!
//! There is no game loop, because a turn-based game does not need one: the
//! browser calls us when something is clicked, and nothing changes in between.
//! Snake and Breakout would need `requestAnimationFrame` and a `Ticker`; this
//! is the shape a turn-based game takes when nothing imposes a shape on it.
//!
//! # Building
//!
//! ```text
//! cargo build -p tic-tac-toe-web --target wasm32-unknown-unknown --release
//! wasm-bindgen --target web --out-dir <dir> <the .wasm>
//! ```
//!
//! `tools/build-web.sh` does this as part of `just web`.

pub mod geometry;

use tic_tac_toe_lib::{Board, GameStatus, Player, TicTacToeGame};
use wasm_bindgen::JsCast;
use wasm_bindgen::prelude::*;
use web_sys::{CanvasRenderingContext2d, HtmlCanvasElement, MouseEvent};

use geometry::Grid;

/// Canvas side length, in CSS pixels.
const CANVAS: f64 = 360.0;

/// The `id` of the canvas element this looks for.
const CANVAS_ID: &str = "board";

/// The `id` of the element the status line is written into.
const STATUS_ID: &str = "status";

/// Builds the standard three-by-three, two-player game.
///
/// Exactly the setup the CLI frontend uses; the board size and win length are
/// the library's business, not this file's.
pub fn new_game() -> TicTacToeGame {
    TicTacToeGame::new(
        Board::new(3, 3),
        vec![
            Player::new("X".to_string(), 'X'),
            Player::new("O".to_string(), 'O'),
        ],
        3,
    )
}

/// The status line for the current game state.
///
/// Pure, so it is tested below rather than by clicking through a browser.
pub fn status_line(game: &TicTacToeGame) -> String {
    match game.status() {
        GameStatus::Won(player) => format!("{} wins - click to play again", player.symbol()),
        GameStatus::Draw => "Draw - click to play again".to_string(),
        GameStatus::InProgress => format!("{} to play", game.current_symbol()),
    }
}

/// Entry point, called by the generated JavaScript when the module loads.
///
/// # Errors
/// Returns a `JsValue` if the page has no canvas with the expected `id`, or the
/// browser refuses a 2D context.
#[wasm_bindgen(start)]
pub fn start() -> Result<(), JsValue> {
    let document = web_sys::window()
        .ok_or_else(|| JsValue::from_str("no window"))?
        .document()
        .ok_or_else(|| JsValue::from_str("no document"))?;

    let canvas: HtmlCanvasElement = document
        .get_element_by_id(CANVAS_ID)
        .ok_or_else(|| JsValue::from_str("no #board canvas"))?
        .dyn_into()?;
    canvas.set_width(CANVAS as u32);
    canvas.set_height(CANVAS as u32);

    let context: CanvasRenderingContext2d = canvas
        .get_context("2d")?
        .ok_or_else(|| JsValue::from_str("no 2d context"))?
        .dyn_into()?;

    // The whole application state, owned by the closure below. No resource
    // system, no entity registry, no scene tree — one game and one canvas.
    let game = std::rc::Rc::new(std::cell::RefCell::new(new_game()));
    let grid = Grid::new(CANVAS, 3, 3);

    draw(&context, &game.borrow(), grid);
    set_status(&document, &status_line(&game.borrow()));

    let on_click = {
        let game = game.clone();
        let context = context.clone();
        let document = document.clone();
        Closure::<dyn FnMut(MouseEvent)>::new(move |event: MouseEvent| {
            let mut game = game.borrow_mut();
            if matches!(game.status(), GameStatus::Won(_) | GameStatus::Draw) {
                game.reset();
            } else if let Some((row, column)) =
                grid.cell_at(f64::from(event.offset_x()), f64::from(event.offset_y()))
            {
                // A rejected move is simply ignored: whether a square is
                // playable is the library's rule, not this frontend's.
                let _ = game.take_turn(row, column);
            }
            draw(&context, &game, grid);
            set_status(&document, &status_line(&game));
        })
    };
    canvas.set_onclick(Some(on_click.as_ref().unchecked_ref()));
    // Handing ownership to the JS runtime; the listener lives as long as the page.
    on_click.forget();

    Ok(())
}

/// Writes the status line into the page.
fn set_status(document: &web_sys::Document, text: &str) {
    if let Some(element) = document.get_element_by_id(STATUS_ID) {
        element.set_text_content(Some(text));
    }
}

/// Draws the whole board. Cheap enough to redraw entirely on every click.
fn draw(context: &CanvasRenderingContext2d, game: &TicTacToeGame, grid: Grid) {
    context.set_fill_style_str("#14161a");
    context.fill_rect(0.0, 0.0, grid.canvas, grid.canvas);

    context.set_stroke_style_str("#3a4048");
    context.set_line_width(2.0);
    for index in 1..grid.columns {
        let x = index as f64 * grid.cell_width();
        context.begin_path();
        context.move_to(x, 0.0);
        context.line_to(x, grid.canvas);
        context.stroke();
    }
    for index in 1..grid.rows {
        let y = index as f64 * grid.cell_height();
        context.begin_path();
        context.move_to(0.0, y);
        context.line_to(grid.canvas, y);
        context.stroke();
    }

    context.set_line_width(8.0);
    for row in 0..grid.rows {
        for column in 0..grid.columns {
            let Some(symbol) = game.board().get(row, column) else {
                continue;
            };
            let (cx, cy) = grid.cell_centre(row, column);
            let reach = grid.cell_rect(row, column).size * 0.28;
            match symbol {
                'X' => {
                    context.set_stroke_style_str("#e06c75");
                    context.begin_path();
                    context.move_to(cx - reach, cy - reach);
                    context.line_to(cx + reach, cy + reach);
                    context.move_to(cx + reach, cy - reach);
                    context.line_to(cx - reach, cy + reach);
                    context.stroke();
                }
                'O' => {
                    context.set_stroke_style_str("#61afef");
                    context.begin_path();
                    let _ = context.arc(cx, cy, reach, 0.0, std::f64::consts::TAU);
                    context.stroke();
                }
                _ => {}
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_new_game_asks_x_to_play() {
        assert_eq!(status_line(&new_game()), "X to play");
    }

    #[test]
    fn the_status_line_follows_the_turn() {
        let mut game = new_game();
        game.take_turn(0, 0).expect("legal");
        assert_eq!(status_line(&game), "O to play");
    }

    #[test]
    fn a_win_is_announced_with_a_way_to_restart() {
        let mut game = new_game();
        for (row, column) in [(0, 0), (1, 0), (0, 1), (1, 1), (0, 2)] {
            game.take_turn(row, column).expect("legal");
        }
        let line = status_line(&game);
        assert!(line.starts_with("X wins"), "{line}");
        assert!(line.contains("play again"), "{line}");
    }

    #[test]
    fn a_draw_is_announced() {
        let mut game = new_game();
        // A full board with no line of three.
        for (row, column) in [
            (0, 0),
            (0, 1),
            (0, 2),
            (1, 1),
            (1, 0),
            (1, 2),
            (2, 1),
            (2, 0),
            (2, 2),
        ] {
            game.take_turn(row, column).expect("legal");
        }
        assert_eq!(status_line(&game), "Draw - click to play again");
    }

    #[test]
    fn the_frontend_contains_no_rule_about_which_squares_are_playable() {
        // Taking an occupied square must be refused by the *library*. If this
        // ever succeeds, a rule has leaked out of the rules crate.
        let mut game = new_game();
        game.take_turn(1, 1).expect("legal");
        assert!(
            game.take_turn(1, 1).is_err(),
            "the library must refuse this"
        );
    }
}
