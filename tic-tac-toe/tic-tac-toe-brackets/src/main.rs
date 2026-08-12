//! A mouse-driven [bracket-lib](https://github.com/amethyst/bracket-lib)
//! frontend for [`tic_tac_toe_lib`].
//!
//! Click a cell to place the current player's symbol. As with the terminal
//! frontend, this crate owns only input and rendering: legality, the winner,
//! and draw detection all come from the library, so the two frontends can
//! never disagree about the rules.

use bracket_lib::prelude::*;
use tic_tac_toe_lib::{Board, GameStatus, Player, TicTacToeGame};

const SCREEN_WIDTH: i32 = 80;
const SCREEN_HEIGHT: i32 = 50;
const DISPLAY_WIDTH: i32 = SCREEN_WIDTH / 2;
const DISPLAY_HEIGHT: i32 = SCREEN_HEIGHT / 2;
const BOARD_OFFSET_X: i32 = 10;
const BOARD_OFFSET_Y: i32 = 5;
const BACKGROUND_COLOR: (u8, u8, u8) = WHITE_SMOKE;
const FOREGROUND_COLOR: (u8, u8, u8) = SKY_BLUE;
/// `pretty_board` puts a `-+-` separator line between cell rows, so one board
/// cell spans two console cells on each axis.
const CELL_STRIDE: i32 = 2;

/// Converts a console cell to the board coordinate drawn there.
///
/// Pure so the coordinate mapping can be tested without a window. The result
/// may be negative or past the end of the board; callers pass it to
/// [`TicTacToeGame::take_turn`], which rejects anything off the board.
fn screen_to_board(screen_x: i32, screen_y: i32) -> (i32, i32) {
    (
        (screen_x - BOARD_OFFSET_X).div_euclid(CELL_STRIDE),
        (screen_y - BOARD_OFFSET_Y).div_euclid(CELL_STRIDE),
    )
}

/// Converts a board coordinate to the console cell that draws it.
fn board_to_screen(column: i32, row: i32) -> (i32, i32) {
    (
        BOARD_OFFSET_X + column * CELL_STRIDE,
        BOARD_OFFSET_Y + row * CELL_STRIDE,
    )
}

/// Whether a console cell falls inside the drawn board.
fn is_inside_board(game: &TicTacToeGame, screen_x: i32, screen_y: i32) -> bool {
    let width = game.width() as i32;
    let height = game.height() as i32;
    let (left, top) = board_to_screen(0, 0);
    let (right, bottom) = board_to_screen(width - 1, height - 1);
    (left..=right).contains(&screen_x) && (top..=bottom).contains(&screen_y)
}

/// The banner shown on the end screen.
fn outcome_message(game: &TicTacToeGame) -> String {
    match game.status() {
        GameStatus::Won(player) => format!("{} ({}) wins!", player.name(), player.symbol()),
        GameStatus::Draw => "It's a draw!".to_string(),
        GameStatus::InProgress => "Game abandoned.".to_string(),
    }
}

fn draw_game(game: &TicTacToeGame, ctx: &mut BTerm) {
    ctx.cls_bg(BACKGROUND_COLOR);
    ctx.print_color(5, 2, FOREGROUND_COLOR, BACKGROUND_COLOR, "Tic-Tac-Toe");
    let mut y = BOARD_OFFSET_Y;
    for line in game.pretty_board().lines() {
        ctx.print_color(BOARD_OFFSET_X, y, FOREGROUND_COLOR, BACKGROUND_COLOR, line);
        y += 1;
    }
    ctx.print_color(
        BOARD_OFFSET_X,
        y + 1,
        FOREGROUND_COLOR,
        BACKGROUND_COLOR,
        format!("{}'s turn", game.current_player().name()),
    );
}

enum GameMode {
    Menu,
    Playing,
    End,
}

struct State {
    mode: GameMode,
    game: TicTacToeGame,
    was_left_mouse_pressed: bool,
}

impl State {
    fn play(&mut self, ctx: &mut BTerm) {
        ctx.mouse_visible = true;

        draw_game(&self.game, ctx);

        let mouse_pos = INPUT.lock().mouse_tile(0);
        let Point { x, y } = mouse_pos;
        let is_left_pressed = INPUT.lock().is_mouse_button_pressed(0);

        let mut draw_batch = DrawBatch::new();

        if is_left_pressed {
            self.was_left_mouse_pressed = true;
            draw_batch.print_color(
                mouse_pos,
                " ",
                ColorPair::new(FOREGROUND_COLOR, FOREGROUND_COLOR),
            );
        } else if self.was_left_mouse_pressed {
            // Act on release, so a click-and-drag off the board is not a move.
            self.was_left_mouse_pressed = false;
            if is_inside_board(&self.game, x, y) {
                let (column, row) = screen_to_board(x, y);
                // The library validates the coordinate; an out-of-range click
                // is simply rejected and the turn is not consumed.
                if let (Ok(row), Ok(column)) = (usize::try_from(row), usize::try_from(column))
                    && self.game.take_turn(row, column).is_ok()
                    && self.game.is_game_over()
                {
                    self.mode = GameMode::End;
                }
            }
        }

        draw_batch.submit(0).expect("Batch error");
        render_draw_buffer(ctx).expect("Render error");
    }

    fn restart(&mut self) {
        self.game.reset();
        self.mode = GameMode::Playing;
    }

    fn main_menu(&mut self, ctx: &mut BTerm) {
        ctx.cls();
        ctx.print_centered(5, "Tic-Tac-Toe");
        ctx.print_centered(8, "(P) Play Game");
        ctx.print_centered(9, "(Q) Quit Game");
        self.handle_menu_keys(ctx);
    }

    fn end_game(&mut self, ctx: &mut BTerm) {
        ctx.cls();
        ctx.print_centered(5, "Game Over!");
        ctx.print_centered(6, outcome_message(&self.game));
        ctx.print_centered(8, "(P) Play Again");
        ctx.print_centered(9, "(Q) Quit Game");
        self.handle_menu_keys(ctx);
    }

    fn handle_menu_keys(&mut self, ctx: &mut BTerm) {
        match ctx.key {
            Some(VirtualKeyCode::P) => self.restart(),
            Some(VirtualKeyCode::Q) => ctx.quitting = true,
            _ => {}
        }
    }
}

impl GameState for State {
    fn tick(&mut self, ctx: &mut BTerm) {
        match self.mode {
            GameMode::Menu => self.main_menu(ctx),
            GameMode::End => self.end_game(ctx),
            GameMode::Playing => self.play(ctx),
        }
    }
}

fn main() -> BError {
    let game = TicTacToeGame::new(
        Board::new(3, 3),
        vec![
            Player::new(String::from("Xavier"), 'X'),
            Player::new(String::from("Olive"), 'O'),
        ],
        3,
    );
    let context = BTermBuilder::new()
        .with_title("Tic-Tac-Toe")
        .with_fps_cap(30.0)
        .with_dimensions(DISPLAY_WIDTH, DISPLAY_HEIGHT)
        .with_tile_dimensions(32, 32)
        .with_resource_path("resources/")
        .with_font("terminal8x8.png", 32, 32)
        .with_font("terminal8x8.png", 8, 8)
        .with_simple_console(DISPLAY_WIDTH, DISPLAY_HEIGHT, "terminal8x8.png")
        .with_simple_console_no_bg(DISPLAY_WIDTH, DISPLAY_HEIGHT, "terminal8x8.png")
        .with_simple_console_no_bg(SCREEN_WIDTH * 2, SCREEN_HEIGHT * 2, "terminal8x8.png")
        .build()?;

    main_loop(
        context,
        State {
            mode: GameMode::Menu,
            game,
            was_left_mouse_pressed: false,
        },
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    fn game() -> TicTacToeGame {
        TicTacToeGame::new(
            Board::new(3, 3),
            vec![
                Player::new("Xavier".to_string(), 'X'),
                Player::new("Olive".to_string(), 'O'),
            ],
            3,
        )
    }

    #[test]
    fn board_and_screen_coordinates_round_trip() {
        for row in 0..3 {
            for column in 0..3 {
                let (x, y) = board_to_screen(column, row);
                assert_eq!(screen_to_board(x, y), (column, row));
            }
        }
    }

    #[test]
    fn board_origin_maps_to_the_draw_offset() {
        assert_eq!(board_to_screen(0, 0), (BOARD_OFFSET_X, BOARD_OFFSET_Y));
        assert_eq!(screen_to_board(BOARD_OFFSET_X, BOARD_OFFSET_Y), (0, 0));
    }

    #[test]
    fn clicks_left_of_the_board_do_not_wrap_to_a_valid_cell() {
        // Truncating division would map x just left of the board back onto
        // column 0; euclidean division keeps it negative so it is rejected.
        let (column, _) = screen_to_board(BOARD_OFFSET_X - 1, BOARD_OFFSET_Y);
        assert!(column < 0, "expected a negative column, got {column}");
    }

    #[test]
    fn is_inside_board_covers_exactly_the_drawn_cells() {
        let game = game();
        let (left, top) = board_to_screen(0, 0);
        let (right, bottom) = board_to_screen(2, 2);

        assert!(is_inside_board(&game, left, top));
        assert!(is_inside_board(&game, right, bottom));
        assert!(!is_inside_board(&game, left - 1, top));
        assert!(!is_inside_board(&game, left, top - 1));
        assert!(!is_inside_board(&game, right + 1, bottom));
        assert!(!is_inside_board(&game, right, bottom + 1));
    }

    #[test]
    fn outcome_message_names_the_winner() {
        let mut game = game();
        for &(row, column) in &[(0, 0), (1, 0), (0, 1), (1, 1), (0, 2)] {
            game.take_turn(row, column).unwrap();
        }
        assert_eq!(outcome_message(&game), "Xavier (X) wins!");
    }

    #[test]
    fn outcome_message_reports_a_draw() {
        let mut game = game();
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
        assert_eq!(outcome_message(&game), "It's a draw!");
    }

    #[test]
    fn outcome_message_handles_an_unfinished_game() {
        assert_eq!(outcome_message(&game()), "Game abandoned.");
    }
}
