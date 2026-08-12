//! Engine-agnostic Breakout rules — continuous physics, fixed timestep.
//!
//! This is the repository's third demonstration of goal #4, and it was chosen
//! because it looked like the one that would break the pattern. Tic-tac-toe is
//! turn-based and Snake moves on a grid; both have state that is exactly
//! representable and advances in whole units. Breakout has a ball at a
//! floating-point position moving at a floating-point velocity, bouncing off
//! things. Engines ship physics; the temptation is to use theirs.
//!
//! # The pattern holds, on one condition
//!
//! [`BreakoutGame::step`] advances the world by a **fixed** [`DT`] — never by a
//! frame's elapsed time. That is the whole trick, and it is the same rule Snake
//! follows for the same reason: a library that never reads a clock stays
//! engine-agnostic *and* deterministic.
//!
//! Taking `dt` as an argument would have been the obvious API and is a trap. It
//! makes the simulation frame-rate dependent — a ball that crosses a brick in
//! one 8 ms step can tunnel straight through it in one 40 ms step — and it
//! destroys reproducibility, because the physics then depends on how busy the
//! machine was.
//!
//! So the frontend's job is unchanged from Snake: use [`snake_lib::Ticker`]-style
//! accumulation to convert frame time into a whole number of fixed steps.
//!
//! # What continuous motion *does* add
//!
//! One thing Snake did not need: **interpolation**. A grid game can draw the
//! simulation state directly, because a snake is either in a cell or it is not.
//! A ball at 120 steps per second drawn on a 144 Hz monitor judders visibly if
//! you draw the last simulated position.
//!
//! So this library keeps the previous position alongside the current one and
//! exposes [`BreakoutGame::ball_at`], which blends between them. The frontend
//! passes the fraction of the way to the next step — exactly what a ticker's
//! `alpha` reports. Rendering interpolates; the simulation never does.
//!
//! That is the real boundary this game found. Not "physics cannot be
//! engine-agnostic", but "continuous state needs a rendering-side concept that
//! discrete state does not".
//!
//! # Example
//! ```
//! use breakout_lib::{BreakoutGame, PaddleInput};
//!
//! let mut game = BreakoutGame::new(BreakoutGame::default_layout());
//! game.set_paddle_input(PaddleInput::Right);
//! for _ in 0..60 {
//!     game.step();
//! }
//! // Rendering asks for a position between the last two steps.
//! let drawn = game.ball_at(0.5);
//! assert!(drawn.x > 0.0);
//! ```

/// A point or vector in game space, with no dependency on any engine's maths.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Vec2 {
    /// Horizontal component, increasing rightwards.
    pub x: f32,
    /// Vertical component, increasing downwards.
    pub y: f32,
}

impl Vec2 {
    /// Creates a vector.
    pub const fn new(x: f32, y: f32) -> Self {
        Self { x, y }
    }

    /// Blends linearly toward `other`, with `t` clamped to `0.0..=1.0`.
    pub fn lerp(self, other: Self, t: f32) -> Self {
        let t = t.clamp(0.0, 1.0);
        Self::new(
            self.x + (other.x - self.x) * t,
            self.y + (other.y - self.y) * t,
        )
    }
}

/// Seconds one simulation step covers.
///
/// 120 Hz: fast enough that the ball never crosses a brick in a single step at
/// the speeds this game uses, and an exact multiple of common frame rates.
pub const DT: f32 = 1.0 / 120.0;

/// Simulation steps per second, the reciprocal of [`DT`].
pub const STEPS_PER_SECOND: f32 = 120.0;

/// Which way the player is pushing the paddle this step.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum PaddleInput {
    /// No input; the paddle holds position.
    #[default]
    None,
    /// Move left.
    Left,
    /// Move right.
    Right,
}

/// An axis-aligned rectangle, used for bricks and the paddle.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Rect {
    /// Centre of the rectangle.
    pub centre: Vec2,
    /// Distance from the centre to each edge.
    pub half: Vec2,
}

impl Rect {
    /// Creates a rectangle from its centre and half-extents.
    pub const fn new(centre: Vec2, half: Vec2) -> Self {
        Self { centre, half }
    }

    /// Left edge.
    pub fn left(&self) -> f32 {
        self.centre.x - self.half.x
    }
    /// Right edge.
    pub fn right(&self) -> f32 {
        self.centre.x + self.half.x
    }
    /// Top edge (smaller `y`).
    pub fn top(&self) -> f32 {
        self.centre.y - self.half.y
    }
    /// Bottom edge (larger `y`).
    pub fn bottom(&self) -> f32 {
        self.centre.y + self.half.y
    }

    /// Whether a circle overlaps this rectangle.
    pub fn overlaps_circle(&self, centre: Vec2, radius: f32) -> bool {
        let nearest_x = centre.x.clamp(self.left(), self.right());
        let nearest_y = centre.y.clamp(self.top(), self.bottom());
        let dx = centre.x - nearest_x;
        let dy = centre.y - nearest_y;
        dx * dx + dy * dy <= radius * radius
    }
}

/// One destructible brick.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Brick {
    /// Where the brick sits.
    pub rect: Rect,
    /// Hits remaining; the brick is gone at zero.
    pub hits: u8,
    /// Row index, so a frontend can colour by row.
    pub row: usize,
}

impl Brick {
    /// Whether the brick is still in play.
    pub fn alive(&self) -> bool {
        self.hits > 0
    }
}

/// Where a game stands.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GameStatus {
    /// Still playing.
    Playing,
    /// Every brick destroyed.
    Won,
    /// No lives left.
    Lost,
}

/// What one [`step`](BreakoutGame::step) did, so a frontend can react.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct StepOutcome {
    /// The ball bounced off a wall or the ceiling.
    pub hit_wall: bool,
    /// The ball bounced off the paddle.
    pub hit_paddle: bool,
    /// Index of the brick hit, if any.
    pub hit_brick: Option<usize>,
    /// A brick was destroyed outright this step.
    pub broke_brick: bool,
    /// The ball was lost and a life spent.
    pub lost_life: bool,
    /// The game ended this step.
    pub finished: bool,
}

/// The dimensions and contents a game starts with.
#[derive(Debug, Clone, PartialEq)]
pub struct Layout {
    /// Play-field width.
    pub width: f32,
    /// Play-field height.
    pub height: f32,
    /// Half-width of the paddle.
    pub paddle_half_width: f32,
    /// How fast the paddle moves, in units per second.
    pub paddle_speed: f32,
    /// Ball radius.
    pub ball_radius: f32,
    /// Ball speed, in units per second.
    pub ball_speed: f32,
    /// Lives the player starts with.
    pub lives: u32,
    /// The bricks.
    pub bricks: Vec<Brick>,
}

/// A game of Breakout.
#[derive(Debug, Clone, PartialEq)]
pub struct BreakoutGame {
    layout: Layout,
    paddle_x: f32,
    paddle_input: PaddleInput,
    ball: Vec2,
    ball_prev: Vec2,
    ball_vel: Vec2,
    /// Set while the ball rides the paddle, before launch.
    stuck: bool,
    lives: u32,
    score: u32,
    ticks: u64,
    status: GameStatus,
}

impl BreakoutGame {
    /// Starts a game with the given layout.
    ///
    /// # Panics
    /// Panics if the play field has no area or there are no lives.
    pub fn new(layout: Layout) -> Self {
        assert!(
            layout.width > 0.0 && layout.height > 0.0,
            "play field must have a positive size"
        );
        assert!(layout.lives > 0, "a game needs at least one life");

        let paddle_x = layout.width / 2.0;
        let ball = Self::ball_rest_position(&layout, paddle_x);
        Self {
            paddle_x,
            paddle_input: PaddleInput::None,
            ball,
            ball_prev: ball,
            ball_vel: Vec2::new(0.0, 0.0),
            stuck: true,
            lives: layout.lives,
            score: 0,
            ticks: 0,
            status: GameStatus::Playing,
            layout,
        }
    }

    /// A conventional Breakout board: eight columns by five rows of bricks.
    pub fn default_layout() -> Layout {
        let (width, height) = (800.0, 600.0);
        let (cols, rows) = (8usize, 5usize);
        let (margin, top) = (40.0, 70.0);
        let brick_w = (width - margin * 2.0) / cols as f32;
        let brick_h = 24.0;

        let mut bricks = Vec::with_capacity(cols * rows);
        for row in 0..rows {
            for col in 0..cols {
                bricks.push(Brick {
                    rect: Rect::new(
                        Vec2::new(
                            margin + brick_w * (col as f32 + 0.5),
                            top + brick_h * (row as f32 + 0.5),
                        ),
                        Vec2::new(brick_w / 2.0 - 2.0, brick_h / 2.0 - 2.0),
                    ),
                    // The top two rows take two hits, for a little texture.
                    hits: if row < 2 { 2 } else { 1 },
                    row,
                });
            }
        }

        Layout {
            width,
            height,
            paddle_half_width: 55.0,
            paddle_speed: 520.0,
            ball_radius: 7.0,
            ball_speed: 330.0,
            lives: 3,
            bricks,
        }
    }

    /// Where the ball sits when resting on the paddle.
    fn ball_rest_position(layout: &Layout, paddle_x: f32) -> Vec2 {
        Vec2::new(paddle_x, layout.height - 46.0 - layout.ball_radius)
    }

    /// Sets which way the paddle is being pushed. Applied by the next step.
    pub fn set_paddle_input(&mut self, input: PaddleInput) {
        self.paddle_input = input;
    }

    /// Launches the ball if it is resting on the paddle.
    ///
    /// Returns whether a launch happened, so a frontend need not track it.
    pub fn launch(&mut self) -> bool {
        if !self.stuck || self.status != GameStatus::Playing {
            return false;
        }
        // Fixed launch angle, so a game is reproducible from its inputs alone.
        let speed = self.layout.ball_speed;
        self.ball_vel = Vec2::new(speed * 0.55, -speed * 0.835);
        self.stuck = false;
        true
    }

    /// Advances the world by exactly [`DT`] seconds.
    ///
    /// Never takes a frame's elapsed time: see the module documentation for why
    /// that would break both determinism and the physics.
    pub fn step(&mut self) -> StepOutcome {
        let mut outcome = StepOutcome::default();
        if self.status != GameStatus::Playing {
            return outcome;
        }
        self.ticks += 1;

        // Paddle first, so a ball resting on it follows.
        let dx = match self.paddle_input {
            PaddleInput::Left => -self.layout.paddle_speed * DT,
            PaddleInput::Right => self.layout.paddle_speed * DT,
            PaddleInput::None => 0.0,
        };
        let half = self.layout.paddle_half_width;
        self.paddle_x = (self.paddle_x + dx).clamp(half, self.layout.width - half);

        self.ball_prev = self.ball;

        if self.stuck {
            self.ball = Self::ball_rest_position(&self.layout, self.paddle_x);
            self.ball_prev = self.ball;
            return outcome;
        }

        self.ball.x += self.ball_vel.x * DT;
        self.ball.y += self.ball_vel.y * DT;

        self.collide_walls(&mut outcome);
        self.collide_paddle(&mut outcome);
        self.collide_bricks(&mut outcome);
        self.check_loss(&mut outcome);

        if self.status == GameStatus::Playing && self.bricks_remaining() == 0 {
            self.status = GameStatus::Won;
            outcome.finished = true;
        }
        outcome
    }

    fn collide_walls(&mut self, outcome: &mut StepOutcome) {
        let r = self.layout.ball_radius;
        if self.ball.x - r < 0.0 {
            self.ball.x = r;
            self.ball_vel.x = self.ball_vel.x.abs();
            outcome.hit_wall = true;
        } else if self.ball.x + r > self.layout.width {
            self.ball.x = self.layout.width - r;
            self.ball_vel.x = -self.ball_vel.x.abs();
            outcome.hit_wall = true;
        }
        if self.ball.y - r < 0.0 {
            self.ball.y = r;
            self.ball_vel.y = self.ball_vel.y.abs();
            outcome.hit_wall = true;
        }
    }

    fn collide_paddle(&mut self, outcome: &mut StepOutcome) {
        let paddle = self.paddle_rect();
        // Only bounce when travelling downward, or the ball can stick to the
        // paddle's side and jitter.
        if self.ball_vel.y <= 0.0 || !paddle.overlaps_circle(self.ball, self.layout.ball_radius) {
            return;
        }
        self.ball.y = paddle.top() - self.layout.ball_radius;

        // Where the ball struck, in -1..=1, steers the bounce — the one piece
        // of "feel" in the game, and the reason a paddle is not just a wall.
        let offset =
            ((self.ball.x - self.paddle_x) / self.layout.paddle_half_width).clamp(-1.0, 1.0);
        let speed = self.layout.ball_speed;
        let vx = offset * speed * 0.75;
        // Keep the total speed constant so the ball never crawls or runs away.
        let vy = -(speed * speed - vx * vx).max(1.0).sqrt();
        self.ball_vel = Vec2::new(vx, vy);
        outcome.hit_paddle = true;
    }

    fn collide_bricks(&mut self, outcome: &mut StepOutcome) {
        let r = self.layout.ball_radius;
        // First brick in index order, so the result never depends on iteration
        // order — the difference between a deterministic game and a nearly
        // deterministic one.
        let hit = self
            .layout
            .bricks
            .iter()
            .position(|b| b.alive() && b.rect.overlaps_circle(self.ball, r));
        let Some(index) = hit else {
            return;
        };

        let rect = self.layout.bricks[index].rect;
        // Reflect along whichever axis the ball entered from, judged by how far
        // it has penetrated each edge.
        let overlap_x = (r + rect.half.x) - (self.ball.x - rect.centre.x).abs();
        let overlap_y = (r + rect.half.y) - (self.ball.y - rect.centre.y).abs();
        if overlap_x < overlap_y {
            self.ball_vel.x = -self.ball_vel.x;
            self.ball.x += if self.ball.x < rect.centre.x {
                -overlap_x
            } else {
                overlap_x
            };
        } else {
            self.ball_vel.y = -self.ball_vel.y;
            self.ball.y += if self.ball.y < rect.centre.y {
                -overlap_y
            } else {
                overlap_y
            };
        }

        let brick = &mut self.layout.bricks[index];
        brick.hits -= 1;
        self.score += 10;
        outcome.hit_brick = Some(index);
        if !brick.alive() {
            self.score += 15;
            outcome.broke_brick = true;
        }
    }

    fn check_loss(&mut self, outcome: &mut StepOutcome) {
        if self.ball.y - self.layout.ball_radius <= self.layout.height {
            return;
        }
        outcome.lost_life = true;
        self.lives = self.lives.saturating_sub(1);
        if self.lives == 0 {
            self.status = GameStatus::Lost;
            outcome.finished = true;
            return;
        }
        self.stuck = true;
        self.ball_vel = Vec2::new(0.0, 0.0);
        self.ball = Self::ball_rest_position(&self.layout, self.paddle_x);
        self.ball_prev = self.ball;
    }

    /// The ball's drawn position, `alpha` of the way from the previous step to
    /// the current one.
    ///
    /// Pass the fraction reported by whatever is converting frame time into
    /// steps. Drawing [`ball`](Self::ball) directly judders whenever the frame
    /// rate is not a multiple of the simulation rate.
    pub fn ball_at(&self, alpha: f32) -> Vec2 {
        self.ball_prev.lerp(self.ball, alpha)
    }

    /// The ball's simulated position after the last step.
    pub fn ball(&self) -> Vec2 {
        self.ball
    }

    /// The ball's velocity.
    pub fn ball_velocity(&self) -> Vec2 {
        self.ball_vel
    }

    /// Whether the ball is resting on the paddle, waiting to be launched.
    pub fn ball_is_stuck(&self) -> bool {
        self.stuck
    }

    /// The paddle's rectangle.
    pub fn paddle_rect(&self) -> Rect {
        Rect::new(
            Vec2::new(self.paddle_x, self.layout.height - 34.0),
            Vec2::new(self.layout.paddle_half_width, 9.0),
        )
    }

    /// The paddle's centre on the x axis.
    pub fn paddle_x(&self) -> f32 {
        self.paddle_x
    }

    /// Every brick, destroyed ones included — check [`Brick::alive`].
    pub fn bricks(&self) -> &[Brick] {
        &self.layout.bricks
    }

    /// How many bricks are still standing.
    pub fn bricks_remaining(&self) -> usize {
        self.layout.bricks.iter().filter(|b| b.alive()).count()
    }

    /// Play-field size.
    pub fn size(&self) -> Vec2 {
        Vec2::new(self.layout.width, self.layout.height)
    }

    /// Lives left.
    pub fn lives(&self) -> u32 {
        self.lives
    }

    /// Current score.
    pub fn score(&self) -> u32 {
        self.score
    }

    /// Steps taken.
    pub fn ticks(&self) -> u64 {
        self.ticks
    }

    /// Where the game stands.
    pub fn status(&self) -> GameStatus {
        self.status
    }

    /// Whether the game has ended.
    pub fn is_over(&self) -> bool {
        self.status != GameStatus::Playing
    }

    /// Places the ball and its velocity directly, for tests.
    ///
    /// Collision response is the most intricate part of this library and the
    /// hardest to reach by playing — landing a ball on a specific brick face by
    /// choosing a launch angle is not a test, it is a coincidence. This puts the
    /// ball exactly where a case needs it. Test-only, so the public API stays
    /// honest about what a player can do.
    #[cfg(test)]
    pub(crate) fn place_ball(&mut self, position: Vec2, velocity: Vec2) {
        self.ball = position;
        self.ball_prev = position;
        self.ball_vel = velocity;
        self.stuck = false;
    }
}

#[cfg(test)]
mod tests;
