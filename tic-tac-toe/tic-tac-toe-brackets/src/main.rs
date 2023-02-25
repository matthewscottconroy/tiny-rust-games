use bracket_lib::prelude::*;
use tic_tac_toe_lib::Board;

struct State {}

impl GameState for State {
    fn tick(&mut self, ctx: &mut BTerm) {
        ctx.cls();
        ctx.print(1, 1, "Hello, Bracket Terminal!");
    }
}

fn main() -> BError {
    let context = BTermBuilder::simple80x50()
        .with_title("Tic-Tac-Toe")
        .build()?;
    main_loop(context, State {})
}
