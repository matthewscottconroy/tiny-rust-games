use bracket_lib::prelude::*;
use tic_tac_toe_lib::Board;
use tic_tac_toe_lib::Player;
use tic_tac_toe_lib::TicTacToeGame;

const SCREEN_WIDTH: i32 = 80;
const SCREEN_HEIGHT: i32 = 50;
const FONT_SIZE: i32 = 32;
const FRAME_DURATION: f32 = 75.0;
const BACKGROUND_COLOR: (u8, u8, u8) = WHITE_SMOKE;
const FOREGROUND_COLOR: (u8, u8, u8) = SKY_BLUE;
const DISPLAY_WIDTH: i32 = SCREEN_WIDTH / 2;
const DISPLAY_HEIGHT: i32 = SCREEN_HEIGHT / 2;
const BOARD_OFFSET_X: i32 = 10;
const BOARD_OFFSET_Y: i32 = 5;

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
        true
    }
}

impl WorldGrid for TicTacToeGame {
    fn screen_to_world(&self, screen_x: i32, screen_y: i32) -> (i32, i32) {
        let screen_x_distance = screen_x - BOARD_OFFSET_X;
        let screen_y_distance = screen_y - BOARD_OFFSET_Y;

        let world_x_location = screen_x_distance / 2;
        let world_y_location = screen_y_distance / 2;
        println!(
            "Inside screen to world: screen_x: {}, screen_y: {}, world_x: {}, world_y: {}",
            screen_x, screen_y, world_x_location, world_y_location
        );

        (world_x_location, world_y_location)
    }

    fn world_to_screen(&self, world_x: i32, world_y: i32) -> (i32, i32) {
        let screen_x = BOARD_OFFSET_X + world_x * 2;
        let screen_y = BOARD_OFFSET_Y + world_y * 2;
        println!(
            "Inside world to Screen: world_x: {}, world_y: {}, screen_x:{}, screen_y:{}",
            world_x, world_y, screen_x, screen_y
        );

        (screen_x, screen_y)
    }

    fn is_inside(&self, screen_x: i32, screen_y: i32) -> bool {
        let width: i32 = self.get_width().try_into().unwrap();
        let height: i32 = self.get_height().try_into().unwrap();

        let (upper_x, upper_y) = self.world_to_screen(0, 0);
        let (bottom_x, bottom_y) = self.world_to_screen(width - 1, height - 1);
        let result = !(screen_x < upper_x
            || screen_x > bottom_x
            || screen_y < upper_y
            || screen_y > bottom_y);
        println!("screen_x: {}, screen_y: {}", screen_x, screen_y);
        println!("upper_x: {}, upper_y: {}", upper_x, upper_y);
        println!("bottom_x: {}, bottom_y: {}", bottom_x, bottom_y);
        println!("is_inside: {}", result);
        result
    }
}

impl Render for TicTacToeGame {
    fn render(&mut self, row: usize, column: usize, ctx: &mut BTerm) {
        let board = self.get_board();
        let b_height = board.get_height();
        let b_width = board.get_width();

        ctx.cls_bg(BACKGROUND_COLOR);
        ctx.print_color(5, 2, FOREGROUND_COLOR, BACKGROUND_COLOR, "Tic-Tac-Toe");
        let mut i = BOARD_OFFSET_Y;

        for line in self.get_pretty_board().split('\n') {
            ctx.print_color(BOARD_OFFSET_X, i, FOREGROUND_COLOR, BACKGROUND_COLOR, line);
            i += 1;
        }
    }
}

enum GameMode {
    Menu,
    Playing,
    End,
}

struct State {
    mode: GameMode,
    frame_time: f32,
    game: TicTacToeGame,
    was_left_mouse_pressed: bool,
}

impl State {
    fn play(&mut self, ctx: &mut BTerm) {
        ctx.mouse_visible = true;
        ctx.cls_bg(WHITE_SMOKE);
        self.frame_time += ctx.frame_time_ms;

        if self.frame_time > FRAME_DURATION {
            self.frame_time = 0.0
        }

        self.game.render(10, 10, ctx);

        let mouse_pos = INPUT.lock().mouse_tile(0);
        let mut cursor_color = WHITE;
        let Point { x, y } = mouse_pos;

        let is_left_pressed = INPUT.lock().is_mouse_button_pressed(0);
        let mut draw_batch = DrawBatch::new();

        /*
                draw_batch.print_color(
                    mouse_pos,
                    &format!("Mouse position: {:?}", mouse_pos),
                    ColorPair::new(BACKGROUND_COLOR, FOREGROUND_COLOR),
                );
        */

        if is_left_pressed {
            self.was_left_mouse_pressed = true;
            cursor_color = FOREGROUND_COLOR;
            draw_batch.print_color(mouse_pos, " ", ColorPair::new(cursor_color, cursor_color));
        } else {
            if self.was_left_mouse_pressed {
                let (world_x, world_y) = self.game.screen_to_world(x, y);

                if self.game.is_inside(x, y) {
                    println!("I'm inside");
                    self.game
                        .take_turn(world_y.try_into().unwrap(), world_x.try_into().unwrap());
                }

                self.was_left_mouse_pressed = false;
            }
        }

        draw_batch.submit(0).expect("Batch error");
        render_draw_buffer(ctx).expect("Render error");

        //self.mode = GameMode::End;
    }

    fn restart(&mut self) {
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
        ctx.print_centered(5, "The Game has finished.");
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
    let how_many_to_win = 3;
    let context = BTermBuilder::new()
        .with_title("Dungeon Crawler")
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
            frame_time: 0.0,
            game: TicTacToeGame::new(b, players, how_many_to_win),
            was_left_mouse_pressed: false,
        },
    )
}
