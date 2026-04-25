use bracket_lib::prelude::*;
use tic_tac_toe_lib::Board;
use tic_tac_toe_lib::Player;
use tic_tac_toe_lib::TicTacToeGame;

const SCREEN_WIDTH: i32 = 80;
const SCREEN_HEIGHT: i32 = 50;
const DISPLAY_WIDTH: i32 = SCREEN_WIDTH / 2;
const DISPLAY_HEIGHT: i32 = SCREEN_HEIGHT / 2;
const BOARD_OFFSET_X: i32 = 10;
const BOARD_OFFSET_Y: i32 = 5;
const BACKGROUND_COLOR: (u8, u8, u8) = WHITE_SMOKE;
const FOREGROUND_COLOR: (u8, u8, u8) = SKY_BLUE;

pub trait Render {
    fn render(&mut self, row: usize, column: usize, ctx: &mut BTerm);
}

pub trait WorldGrid {
    fn screen_to_world(&self, screen_x: i32, screen_y: i32) -> (i32, i32) {
        (screen_x, screen_y)
    }

    fn world_to_screen(&self, world_x: i32, world_y: i32) -> (i32, i32) {
        (world_x, world_y)
    }

    fn is_inside(&self, screen_x: i32, screen_y: i32) -> bool {
        let _ = (screen_x, screen_y);
        true
    }
}

impl WorldGrid for TicTacToeGame {
    fn screen_to_world(&self, screen_x: i32, screen_y: i32) -> (i32, i32) {
        let world_x = (screen_x - BOARD_OFFSET_X) / 2;
        let world_y = (screen_y - BOARD_OFFSET_Y) / 2;
        (world_x, world_y)
    }

    fn world_to_screen(&self, world_x: i32, world_y: i32) -> (i32, i32) {
        let screen_x = BOARD_OFFSET_X + world_x * 2;
        let screen_y = BOARD_OFFSET_Y + world_y * 2;
        (screen_x, screen_y)
    }

    fn is_inside(&self, screen_x: i32, screen_y: i32) -> bool {
        let width: i32 = self.get_width().try_into().unwrap();
        let height: i32 = self.get_height().try_into().unwrap();
        let (upper_x, upper_y) = self.world_to_screen(0, 0);
        let (bottom_x, bottom_y) = self.world_to_screen(width - 1, height - 1);
        !(screen_x < upper_x || screen_x > bottom_x || screen_y < upper_y || screen_y > bottom_y)
    }
}

impl Render for TicTacToeGame {
    fn render(&mut self, _row: usize, _column: usize, ctx: &mut BTerm) {
        ctx.cls_bg(BACKGROUND_COLOR);
        ctx.print_color(5, 2, FOREGROUND_COLOR, BACKGROUND_COLOR, "Tic-Tac-Toe");
        let mut i = BOARD_OFFSET_Y;
        for line in self.get_pretty_board().split('\n') {
            ctx.print_color(BOARD_OFFSET_X, i, FOREGROUND_COLOR, BACKGROUND_COLOR, line);
            i += 1;
        }
        ctx.print_color(
            BOARD_OFFSET_X,
            i + 1,
            FOREGROUND_COLOR,
            BACKGROUND_COLOR,
            &format!("{}'s turn", self.get_current_player_name()),
        );
    }
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

        self.game.render(0, 0, ctx);

        let mouse_pos = INPUT.lock().mouse_tile(0);
        let Point { x, y } = mouse_pos;
        let is_left_pressed = INPUT.lock().is_mouse_button_pressed(0);

        let mut draw_batch = DrawBatch::new();

        if is_left_pressed {
            self.was_left_mouse_pressed = true;
            draw_batch.print_color(mouse_pos, " ", ColorPair::new(FOREGROUND_COLOR, FOREGROUND_COLOR));
        } else if self.was_left_mouse_pressed {
            self.was_left_mouse_pressed = false;
            if self.game.is_inside(x, y) {
                let (world_x, world_y) = self.game.screen_to_world(x, y);
                let row = world_y as usize;
                let col = world_x as usize;
                if row < self.game.get_height()
                    && col < self.game.get_width()
                    && self.game.get_board().is_entry_empty(row, col)
                {
                    self.game.take_turn(row, col);
                    if self.game.is_game_over() {
                        self.mode = GameMode::End;
                    }
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

        if let Some(key) = ctx.key {
            match key {
                VirtualKeyCode::P => self.restart(),
                VirtualKeyCode::Q => ctx.quitting = true,
                _ => {}
            }
        }
    }

    fn end_game(&mut self, ctx: &mut BTerm) {
        ctx.cls();
        ctx.print_centered(5, "Game Over!");
        if self.game.do_full_win_check() {
            let winner_idx = (self.game.turn_count() - 1) % self.game.get_number_of_players();
            ctx.print_centered(6, &format!("Player {} wins!", winner_idx + 1));
        } else {
            ctx.print_centered(6, "It's a draw!");
        }
        ctx.print_centered(8, "(P) Play Again");
        ctx.print_centered(9, "(Q) Quit Game");

        if let Some(key) = ctx.key {
            match key {
                VirtualKeyCode::P => self.restart(),
                VirtualKeyCode::Q => ctx.quitting = true,
                _ => {}
            }
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
    let b = Board::new(3, 3);
    let p1 = Player::new(String::from("Matt"), 'X');
    let p2 = Player::new(String::from("John"), 'O');
    let players = vec![p1, p2];
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
            game: TicTacToeGame::new(b, players, 3),
            was_left_mouse_pressed: false,
        },
    )
}
