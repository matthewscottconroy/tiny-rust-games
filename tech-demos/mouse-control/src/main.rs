use bracket_terminal::prelude::*;

struct State {}

impl GameState for State {
    fn tick(&mut self, ctx: &mut BTerm) {
        let mut draw_batch = DrawBatch::new();
        draw_batch.cls();
        let mouse_pos = INPUT.lock().mouse_tile(0);

        draw_batch.print_color(mouse_pos, "X", ColorPair::new(WHITE_SMOKE, WHITE_SMOKE));

        draw_batch.submit(0).expect("Batch error");
        render_draw_buffer(ctx).expect("Render error");
    }
}

fn main() -> BError {
    let context = BTermBuilder::simple80x50()
        .with_title("Mouse Control")
        .build()?;
    let gs = State {};
    main_loop(context, gs)
}
