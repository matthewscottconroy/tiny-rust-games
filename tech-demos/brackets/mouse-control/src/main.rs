//! Mouse control demo — reading mouse position and buttons in bracket-terminal.
//!
//! Teaches: polling the global `INPUT` lock for the mouse tile and button
//! states, drawing a cursor at the mouse position with `DrawBatch`, and
//! rendering a batch through `render_draw_buffer`.
//!
//! **Controls:** move the mouse; hold left (red) or right (blue) button.
//!
//! The button-to-colour rule is factored out as [`cursor_color`] so the part
//! worth testing does not need a window.

use bracket_terminal::prelude::*;

/// Cursor colour when no mouse button is held.
pub const IDLE_COLOR: (u8, u8, u8) = WHITE_SMOKE;
/// Cursor colour while the left button is held.
pub const LEFT_COLOR: (u8, u8, u8) = RED;
/// Cursor colour while the right button is held.
pub const RIGHT_COLOR: (u8, u8, u8) = BLUE;

/// Picks the cursor colour for the current button state.
///
/// Right-click wins when both buttons are held, matching the order the
/// original checks were written in.
pub fn cursor_color(left_pressed: bool, right_pressed: bool) -> (u8, u8, u8) {
    if right_pressed {
        RIGHT_COLOR
    } else if left_pressed {
        LEFT_COLOR
    } else {
        IDLE_COLOR
    }
}

/// Formats the readout shown in the top-left corner.
pub fn position_label(x: i32, y: i32) -> String {
    format!("x: {x}, y: {y}")
}

struct State {}

impl GameState for State {
    fn tick(&mut self, ctx: &mut BTerm) {
        // The demo draws its own cursor, so hide the OS one.
        ctx.mouse_visible = false;

        let mut draw_batch = DrawBatch::new();
        draw_batch.cls();

        let mouse_pos = INPUT.lock().mouse_tile(0);
        let Point { x, y } = mouse_pos;
        let is_left_pressed = INPUT.lock().is_mouse_button_pressed(0);
        let is_right_pressed = INPUT.lock().is_mouse_button_pressed(1);

        let color = cursor_color(is_left_pressed, is_right_pressed);
        draw_batch.print_color(mouse_pos, "X", ColorPair::new(color, color));
        draw_batch.print_color(
            Point { x: 1, y: 1 },
            position_label(x, y),
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
    let gs = State {};
    main_loop(context, gs)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn idle_cursor_is_white() {
        assert_eq!(cursor_color(false, false), IDLE_COLOR);
    }

    #[test]
    fn left_button_turns_the_cursor_red() {
        assert_eq!(cursor_color(true, false), LEFT_COLOR);
    }

    #[test]
    fn right_button_turns_the_cursor_blue() {
        assert_eq!(cursor_color(false, true), RIGHT_COLOR);
    }

    #[test]
    fn right_button_wins_when_both_are_held() {
        assert_eq!(cursor_color(true, true), RIGHT_COLOR);
    }

    #[test]
    fn position_label_shows_both_coordinates() {
        assert_eq!(position_label(12, 34), "x: 12, y: 34");
        assert_eq!(position_label(0, 0), "x: 0, y: 0");
        assert_eq!(position_label(-1, -2), "x: -1, y: -2");
    }
}
