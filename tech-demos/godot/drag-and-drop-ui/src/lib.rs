//! drag-and-drop-ui: Demonstrates GUI input events and custom drawing in Rust.
//! A colored rectangle can be clicked and dragged around the screen.
//!
//! Teaches: handling `Control::_gui_input` and distinguishing
//! `InputEventMouseButton` from `InputEventMouseMotion` while dragging.

use godot::classes::{Control, IControl, InputEvent, InputEventMouseButton, InputEventMouseMotion};
use godot::prelude::*;

struct DragAndDropUiExtension;
#[gdextension]
unsafe impl ExtensionLibrary for DragAndDropUiExtension {}

/// Size of the draggable rectangle, in pixels.
const RECT_W: f32 = 80.0;
const RECT_H: f32 = 60.0;

#[derive(GodotClass)]
#[class(base=Control)]
struct DragAndDropUi {
    dragging: bool,
    drag_offset_x: f32,
    drag_offset_y: f32,
    drag_pos_x: f32,
    drag_pos_y: f32,
    base: Base<Control>,
}

#[godot_api]
impl IControl for DragAndDropUi {
    fn init(base: Base<Control>) -> Self {
        Self {
            dragging: false,
            drag_offset_x: 0.0,
            drag_offset_y: 0.0,
            drag_pos_x: 100.0,
            drag_pos_y: 100.0,
            base,
        }
    }

    fn ready(&mut self) {
        godot_print!("DragAndDropUi ready");
    }

    fn gui_input(&mut self, event: Gd<InputEvent>) {
        if let Ok(mouse_btn) = event.clone().try_cast::<InputEventMouseButton>() {
            // Button index 1 == left mouse button
            if mouse_btn.get_button_index() == godot::global::MouseButton::LEFT {
                let pos = mouse_btn.get_position();
                if mouse_btn.is_pressed() {
                    // Check if click is inside the rect
                    if point_in_rect(
                        pos.x,
                        pos.y,
                        self.drag_pos_x,
                        self.drag_pos_y,
                        RECT_W,
                        RECT_H,
                    ) {
                        self.dragging = true;
                        self.drag_offset_x = pos.x - self.drag_pos_x;
                        self.drag_offset_y = pos.y - self.drag_pos_y;
                    }
                } else {
                    self.dragging = false;
                }
            }
        } else if let Ok(motion) = event.try_cast::<InputEventMouseMotion>()
            && self.dragging
        {
            let pos = motion.get_position();
            let (nx, ny) = apply_offset((pos.x, pos.y), (self.drag_offset_x, self.drag_offset_y));
            self.drag_pos_x = nx;
            self.drag_pos_y = ny;
            self.base_mut().queue_redraw();
        }
    }

    fn draw(&mut self) {
        // Copy fields to locals to avoid borrow conflict with base_mut().
        let dragging = self.dragging;
        let (px, py) = (self.drag_pos_x, self.drag_pos_y);
        let rect = Rect2::new(Vector2::new(px, py), Vector2::new(RECT_W, RECT_H));
        let color = if dragging {
            Color::from_rgb(0.2, 0.6, 1.0)
        } else {
            Color::from_rgb(0.8, 0.3, 0.1)
        };
        self.base_mut().draw_rect(rect, color);
    }
}

// ---------------------------------------------------------------------------
// Pure helpers
// ---------------------------------------------------------------------------

/// Returns true if point (px, py) is inside the rectangle defined by
/// top-left corner (rx, ry) with dimensions (rw × rh).
pub fn point_in_rect(px: f32, py: f32, rx: f32, ry: f32, rw: f32, rh: f32) -> bool {
    px >= rx && px <= rx + rw && py >= ry && py <= ry + rh
}

/// Subtracts an offset from a position, yielding the rectangle's new top-left.
pub fn apply_offset(pos: (f32, f32), offset: (f32, f32)) -> (f32, f32) {
    (pos.0 - offset.0, pos.1 - offset.1)
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn point_inside_rect() {
        assert!(point_in_rect(50.0, 50.0, 0.0, 0.0, 100.0, 100.0));
    }

    #[test]
    fn point_outside_rect_right() {
        assert!(!point_in_rect(110.0, 50.0, 0.0, 0.0, 100.0, 100.0));
    }

    #[test]
    fn point_outside_rect_below() {
        assert!(!point_in_rect(50.0, 110.0, 0.0, 0.0, 100.0, 100.0));
    }

    #[test]
    fn point_on_rect_edge() {
        assert!(point_in_rect(100.0, 100.0, 0.0, 0.0, 100.0, 100.0));
    }

    #[test]
    fn apply_offset_zero() {
        assert_eq!(apply_offset((200.0, 150.0), (0.0, 0.0)), (200.0, 150.0));
    }

    #[test]
    fn apply_offset_non_zero() {
        let (x, y) = apply_offset((300.0, 250.0), (50.0, 30.0));
        assert!((x - 250.0).abs() < 0.001);
        assert!((y - 220.0).abs() < 0.001);
    }

    #[test]
    fn point_not_in_offset_rect() {
        assert!(!point_in_rect(5.0, 5.0, 100.0, 100.0, 80.0, 60.0));
    }
}
