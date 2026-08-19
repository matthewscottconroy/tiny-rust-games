//! Breakout rendered with [Godot](https://godotengine.org/)'s scene tree.
//!
//! Teaches: running a fixed-timestep simulation on Godot's own
//! `_physics_process` clock, interpolating the drawn ball with
//! `Engine::get_physics_interpolation_fraction`, and declining the engine's
//! built-in physics in favour of rules the library already owns.
//!
//! This is the frontend that was supposed to break the pattern. Godot ships a
//! complete 2D physics engine — `CharacterBody2D`, collision shapes, a solver —
//! and the obvious way to write Breakout here is to use it. Doing so would put
//! the rules of the game inside the engine, which is exactly what
//! [`breakout_lib`] exists to prevent: the Bevy frontend and this one would
//! then be free to disagree about how a ball bounces, and neither could be
//! called the game.
//!
//! So this demo uses Godot for what Godot is for — a window, a scene tree, a
//! draw call, an input event — and nothing else. The comparison with
//! `breakout-bevy` is the point:
//!
//! | | Bevy | Godot |
//! |---|---|---|
//! | fixed step | `FixedUpdate` + `Time<Fixed>` | `_physics_process` + `physics_ticks_per_second` |
//! | interpolation | `Time<Fixed>::overstep_fraction()` | `Engine::get_physics_interpolation_fraction()` |
//! | y axis | up, centred — needs a flip | down, top-left — matches the library |
//!
//! Both engines already ship the fixed-timestep scheduler the library needs, so
//! neither frontend hand-rolls an accumulator, and neither one decides the
//! rate: the library's [`STEPS_PER_SECOND`] does, and each engine is configured
//! to match it.
//!
//! **Controls:** left/right or A/D to move   Space to launch   R to restart.

use godot::classes::{Engine, INode2D, InputEvent, InputEventKey, Label, Node2D};
use godot::global::Key;
use godot::prelude::*;

use breakout_lib::{BreakoutGame, GameStatus, PaddleInput, Rect as GameRect, Vec2 as GameVec2};

// ─── Extension entry point ────────────────────────────────────────────────────

struct BreakoutExt;

#[gdextension]
unsafe impl ExtensionLibrary for BreakoutExt {}

// ─── Pure functions ───────────────────────────────────────────────────────────

/// Converts a game-space point to the node's local space.
///
/// The library puts the origin at the top-left with `y` increasing downward,
/// which is also how Godot's 2D space works — so unlike the Bevy frontend, this
/// only has to centre the field, not flip it.
pub fn to_local(point: GameVec2, field: GameVec2) -> Vector2 {
    Vector2::new(point.x - field.x / 2.0, point.y - field.y / 2.0)
}

/// The Godot rectangle covering a game rectangle.
pub fn to_rect2(rect: GameRect, field: GameVec2) -> Rect2 {
    let top_left = to_local(
        GameVec2::new(rect.centre.x - rect.half.x, rect.centre.y - rect.half.y),
        field,
    );
    Rect2::new(top_left, Vector2::new(rect.half.x * 2.0, rect.half.y * 2.0))
}

/// Maps held keys to a paddle input.
///
/// Both directions at once cancel, matching the Bevy frontend — the rule is
/// trivial, but it lives in both frontends rather than the library because it
/// is about *keyboards*, not about Breakout.
pub fn paddle_input_from_keys(left: bool, right: bool) -> PaddleInput {
    match (left, right) {
        (true, false) => PaddleInput::Left,
        (false, true) => PaddleInput::Right,
        _ => PaddleInput::None,
    }
}

/// The status line for the current game state.
pub fn status_line(game: &BreakoutGame) -> String {
    match game.status() {
        GameStatus::Playing if game.ball_is_stuck() => format!(
            "Score {}   Lives {}  -  Space to launch",
            game.score(),
            game.lives()
        ),
        GameStatus::Playing => format!(
            "Score {}   Lives {}   Bricks {}",
            game.score(),
            game.lives(),
            game.bricks_remaining()
        ),
        GameStatus::Won => format!("Cleared! Score {}.  R to restart", game.score()),
        GameStatus::Lost => format!("Game over. Score {}.  R to restart", game.score()),
    }
}

/// Colour for a brick, by row and remaining hits.
pub fn brick_color(row: usize, hits: u8) -> Color {
    let (r, g, b) = match row % 5 {
        0 => (0.90, 0.35, 0.35),
        1 => (0.90, 0.60, 0.30),
        2 => (0.85, 0.85, 0.35),
        3 => (0.40, 0.80, 0.45),
        _ => (0.40, 0.65, 0.90),
    };
    // A damaged two-hit brick is drawn at half brightness, matching the Bevy
    // frontend exactly. The two must agree: a player who learns what a weakened
    // brick looks like in one should not have to relearn it in the other.
    let k = if hits > 1 { 1.0 } else { 0.5 };
    Color::from_rgb(r * k, g * k, b * k)
}

// ─── BreakoutBoard node ───────────────────────────────────────────────────────

/// A `Node2D` that runs and draws a game of Breakout.
///
/// Add it as the root of a scene and run; it creates its own status `Label`
/// child in `ready`, so the scene needs nothing else.
#[derive(GodotClass)]
#[class(base=Node2D)]
pub struct BreakoutBoard {
    game: BreakoutGame,
    status: Option<Gd<Label>>,
    base: Base<Node2D>,
}

#[godot_api]
impl INode2D for BreakoutBoard {
    fn init(base: Base<Node2D>) -> Self {
        Self {
            game: BreakoutGame::new(BreakoutGame::default_layout()),
            status: None,
            base,
        }
    }

    fn ready(&mut self) {
        let field = self.game.size();
        let mut label = Label::new_alloc();
        label.set_text(&status_line(&self.game));
        label.set_position(Vector2::new(-field.x / 2.0, field.y / 2.0 + 8.0));
        self.base_mut().add_child(&label);
        self.status = Some(label);
    }

    /// Advances the simulation exactly one step.
    ///
    /// Godot calls this at `physics_ticks_per_second`, which `project.godot`
    /// sets to the library's own rate — so there is no accumulator here, and no
    /// `delta` is consulted. Taking the frame's `delta` instead would make the
    /// physics frame-rate dependent and destroy reproducibility, which is the
    /// mistake the whole design exists to avoid.
    fn physics_process(&mut self, _delta: f64) {
        if self.game.is_over() {
            return;
        }
        let input = read_paddle_input();
        self.game.set_paddle_input(input);
        self.game.step();

        let text = status_line(&self.game);
        if let Some(label) = self.status.as_mut() {
            label.set_text(&text);
        }
    }

    /// Redraws every frame, interpolated between the last two fixed steps.
    fn process(&mut self, _delta: f64) {
        self.base_mut().queue_redraw();
    }

    fn input(&mut self, event: Gd<InputEvent>) {
        let Ok(key_event) = event.try_cast::<InputEventKey>() else {
            return;
        };
        if !key_event.is_pressed() {
            return;
        }
        match key_event.get_keycode() {
            Key::SPACE => {
                self.game.launch();
            }
            Key::R => {
                self.game = BreakoutGame::new(BreakoutGame::default_layout());
                let text = status_line(&self.game);
                if let Some(label) = self.status.as_mut() {
                    label.set_text(&text);
                }
            }
            _ => {}
        }
    }

    fn draw(&mut self) {
        let field = self.game.size();

        // How far this frame sits between the two most recent fixed steps.
        // Drawing `ball()` directly judders whenever the display rate is not a
        // multiple of 120 Hz; this is the engine's equivalent of the ticker
        // alpha the Bevy frontend passes.
        let alpha = Engine::singleton().get_physics_interpolation_fraction() as f32;
        let ball = to_local(self.game.ball_at(alpha), field);

        // Snapshot before drawing: every draw call needs `&mut self`.
        let bricks: Vec<(Rect2, Color)> = self
            .game
            .bricks()
            .iter()
            .filter(|b| b.alive())
            .map(|b| (to_rect2(b.rect, field), brick_color(b.row, b.hits)))
            .collect();
        let paddle = to_rect2(self.game.paddle_rect(), field);
        let radius = self.game.paddle_rect().half.y.max(7.0);

        let background = Rect2::new(
            Vector2::new(-field.x / 2.0, -field.y / 2.0),
            Vector2::new(field.x, field.y),
        );
        self.base_mut()
            .draw_rect(background, Color::from_rgb(0.08, 0.09, 0.11));

        for (rect, color) in bricks {
            self.base_mut().draw_rect(rect, color);
        }
        self.base_mut()
            .draw_rect(paddle, Color::from_rgb(0.85, 0.88, 0.92));
        self.base_mut()
            .draw_circle(ball, radius, Color::from_rgb(1.0, 0.95, 0.85));
    }
}

#[godot_api]
impl BreakoutBoard {
    /// The current score (callable from GDScript).
    #[func]
    pub fn score(&self) -> i32 {
        self.game.score() as i32
    }

    /// Whether the game has ended (callable from GDScript).
    #[func]
    pub fn is_over(&self) -> bool {
        self.game.is_over()
    }
}

/// Reads the arrow and WASD keys through Godot's global input state.
fn read_paddle_input() -> PaddleInput {
    let input = godot::classes::Input::singleton();
    paddle_input_from_keys(
        input.is_key_pressed(Key::LEFT) || input.is_key_pressed(Key::A),
        input.is_key_pressed(Key::RIGHT) || input.is_key_pressed(Key::D),
    )
}

// ─── Tests ────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    fn field() -> GameVec2 {
        BreakoutGame::new(BreakoutGame::default_layout()).size()
    }

    #[test]
    fn the_field_is_centred_on_the_origin() {
        let f = field();
        let top_left = to_local(GameVec2::new(0.0, 0.0), f);
        let bottom_right = to_local(GameVec2::new(f.x, f.y), f);
        assert!((top_left.x + bottom_right.x).abs() < 1e-3);
        assert!((top_left.y + bottom_right.y).abs() < 1e-3);
    }

    #[test]
    fn the_y_axis_is_not_flipped() {
        // Unlike Bevy, Godot's 2D y grows downward, the same as the library's.
        // A row lower in the game must be lower on screen.
        let f = field();
        let high = to_local(GameVec2::new(0.0, 10.0), f);
        let low = to_local(GameVec2::new(0.0, 200.0), f);
        assert!(low.y > high.y, "y must increase downward");
    }

    #[test]
    fn a_game_rect_maps_to_the_same_size() {
        let f = field();
        let rect = GameRect::new(GameVec2::new(100.0, 50.0), GameVec2::new(20.0, 8.0));
        let mapped = to_rect2(rect, f);
        assert!((mapped.size.x - 40.0).abs() < 1e-3);
        assert!((mapped.size.y - 16.0).abs() < 1e-3);
    }

    #[test]
    fn both_directions_at_once_cancel() {
        assert_eq!(paddle_input_from_keys(true, true), PaddleInput::None);
        assert_eq!(paddle_input_from_keys(false, false), PaddleInput::None);
        assert_eq!(paddle_input_from_keys(true, false), PaddleInput::Left);
        assert_eq!(paddle_input_from_keys(false, true), PaddleInput::Right);
    }

    #[test]
    fn status_line_prompts_a_launch_while_the_ball_rests() {
        let game = BreakoutGame::new(BreakoutGame::default_layout());
        assert!(status_line(&game).contains("Space to launch"));
    }

    #[test]
    fn status_line_reports_bricks_once_launched() {
        let mut game = BreakoutGame::new(BreakoutGame::default_layout());
        game.launch();
        let line = status_line(&game);
        assert!(line.contains("Bricks"), "{line}");
    }

    #[test]
    fn a_damaged_brick_is_dimmer_than_a_fresh_one() {
        let fresh = brick_color(0, 2);
        let damaged = brick_color(0, 1);
        assert!(damaged.r < fresh.r || damaged.g < fresh.g || damaged.b < fresh.b);
    }

    #[test]
    fn every_row_gets_a_colour() {
        // The match is on `row % 5`, so all five arms must be reachable.
        let colours: Vec<Color> = (0..5).map(|row| brick_color(row, 2)).collect();
        for (i, a) in colours.iter().enumerate() {
            for b in colours.iter().skip(i + 1) {
                assert_ne!((a.r, a.g, a.b), (b.r, b.g, b.b));
            }
        }
    }
}
