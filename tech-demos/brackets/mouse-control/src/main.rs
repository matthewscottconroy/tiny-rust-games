use bracket_terminal::prelude::*;

struct State {
    x: i32,
    y: i32,
}

impl GameState for State {
    fn tick(&mut self, ctx: &mut BTerm) {
        ctx.mouse_visible = false;

        let mut draw_batch = DrawBatch::new();
        draw_batch.cls();
        let mouse_pos = INPUT.lock().mouse_tile(0);
        let Point { x, y } = mouse_pos;

        let is_left_pressed = INPUT.lock().is_mouse_button_pressed(0);
        let is_right_pressed = INPUT.lock().is_mouse_button_pressed(1);
        let mut cursor_color = WHITE_SMOKE;

        if is_left_pressed {
            cursor_color = RED;
        }

        if is_right_pressed {
            cursor_color = BLUE;
        }

        draw_batch.print_color(mouse_pos, "X", ColorPair::new(cursor_color, cursor_color));
        draw_batch.print_color(
            Point { x: 1, y: 1 },
            format!("x: {}, y: {}", x, y),
            ColorPair::new(WHITE_SMOKE, BLACK),
        );

        draw_batch.submit(0).expect("Batch error");
        render_draw_buffer(ctx).expect("Render error");
    }
}

fn main() -> BError {
    let context = BTermBuilder::simple80x50()
        .with_title("Mouse Control")
        .build()?;
    let gs = State { x: 0, y: 0 };
    main_loop(context, gs)
}
