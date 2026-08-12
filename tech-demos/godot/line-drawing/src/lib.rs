//! Line Drawing demo — captures mouse left-click positions and draws a
//! colour-cycling polyline using Node2D's _draw() hook.
//!
//! Teaches: procedural polyline drawing in `_draw()`, plus undo and clear over the
//! stored point list.

use godot::classes::{INode2D, InputEvent, InputEventMouseButton, Node2D};
use godot::prelude::*;

struct LineDrawingExtension;
#[gdextension]
unsafe impl ExtensionLibrary for LineDrawingExtension {}

#[derive(GodotClass)]
#[class(base=Node2D)]
struct LineDrawing {
    #[export]
    line_width: f32,
    #[export]
    max_points: u32,

    points: Vec<(f32, f32)>,
    base: Base<Node2D>,
}

#[godot_api]
impl INode2D for LineDrawing {
    fn init(base: Base<Node2D>) -> Self {
        Self {
            line_width: 3.0,
            max_points: 100,
            points: Vec::new(),
            base,
        }
    }

    fn input(&mut self, event: Gd<InputEvent>) {
        if let Ok(mouse_event) = event.try_cast::<InputEventMouseButton>() {
            // Button index 1 is left mouse button
            if mouse_event.get_button_index() == godot::global::MouseButton::LEFT
                && mouse_event.is_pressed()
            {
                let pos = mouse_event.get_position();
                let max = self.max_points as usize;
                if self.points.len() < max {
                    self.points.push((pos.x, pos.y));
                    self.base_mut().queue_redraw();
                }
            }
        }
    }

    fn draw(&mut self) {
        let points = self.points.clone();
        let width = self.line_width;

        if points.len() < 2 {
            return;
        }

        for i in 0..points.len() - 1 {
            let (ax, ay) = points[i];
            let (bx, by) = points[i + 1];
            let (r, g, b) = cycle_color(i);
            let color = Color::from_rgb(r, g, b);
            self.base_mut()
                .draw_line_ex(Vector2::new(ax, ay), Vector2::new(bx, by), color)
                .width(width)
                .done();
        }
    }
}

#[godot_api]
impl LineDrawing {
    #[func]
    pub fn clear_points(&mut self) {
        self.points.clear();
        self.base_mut().queue_redraw();
    }

    #[func]
    pub fn undo_last(&mut self) {
        if !self.points.is_empty() {
            self.points.pop();
            self.base_mut().queue_redraw();
        }
    }
}

// ---------------------------------------------------------------------------
// Pure helper functions
// ---------------------------------------------------------------------------

/// Cycle through a palette of colours based on segment index.
pub fn cycle_color(index: usize) -> (f32, f32, f32) {
    const PALETTE: &[(f32, f32, f32)] = &[
        (1.0, 0.2, 0.2),
        (0.2, 1.0, 0.2),
        (0.2, 0.2, 1.0),
        (1.0, 1.0, 0.2),
        (1.0, 0.2, 1.0),
        (0.2, 1.0, 1.0),
    ];
    PALETTE[index % PALETTE.len()]
}

/// Euclidean length of a line segment.
pub fn segment_length(ax: f32, ay: f32, bx: f32, by: f32) -> f32 {
    let dx = bx - ax;
    let dy = by - ay;
    (dx * dx + dy * dy).sqrt()
}

/// Sum of all segment lengths for the given point list.
pub fn total_length(points: &[(f32, f32)]) -> f32 {
    if points.len() < 2 {
        return 0.0;
    }
    points
        .windows(2)
        .map(|w| segment_length(w[0].0, w[0].1, w[1].0, w[1].1))
        .sum()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn segment_length_horizontal() {
        assert!((segment_length(0.0, 0.0, 5.0, 0.0) - 5.0).abs() < 1e-6);
    }

    #[test]
    fn segment_length_vertical() {
        assert!((segment_length(0.0, 0.0, 0.0, 3.0) - 3.0).abs() < 1e-6);
    }

    #[test]
    fn segment_length_diagonal() {
        // 3-4-5 right triangle
        assert!((segment_length(0.0, 0.0, 3.0, 4.0) - 5.0).abs() < 1e-5);
    }

    #[test]
    fn total_length_no_points() {
        assert!((total_length(&[]) - 0.0).abs() < 1e-6);
    }

    #[test]
    fn total_length_one_point() {
        assert!((total_length(&[(1.0, 2.0)]) - 0.0).abs() < 1e-6);
    }

    #[test]
    fn total_length_two_points() {
        let pts = [(0.0f32, 0.0f32), (10.0, 0.0)];
        assert!((total_length(&pts) - 10.0).abs() < 1e-6);
    }

    #[test]
    fn total_length_three_points() {
        let pts = [(0.0f32, 0.0f32), (3.0, 4.0), (6.0, 8.0)];
        assert!((total_length(&pts) - 10.0).abs() < 1e-5);
    }

    #[test]
    fn cycle_color_wraps() {
        assert_eq!(cycle_color(0), cycle_color(6));
    }

    #[test]
    fn cycle_color_distinct() {
        assert_ne!(cycle_color(0), cycle_color(1));
    }

    #[test]
    fn cycle_color_in_range() {
        for i in 0..12 {
            let (r, g, b) = cycle_color(i);
            assert!((0.0..=1.0).contains(&r));
            assert!((0.0..=1.0).contains(&g));
            assert!((0.0..=1.0).contains(&b));
        }
    }
}
