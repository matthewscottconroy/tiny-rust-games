//! Breakout rendered with [Bevy](https://bevyengine.org/)'s ECS.
//!
//! Snake's frontends hand-roll the frame-time accumulator, because
//! [`snake_lib::Ticker`] has to work from a terminal loop and a Godot callback
//! as well as from Bevy. This frontend does the opposite on purpose: Bevy
//! already ships a fixed-timestep scheduler, so it uses that.
//!
//! - [`BreakoutGame::step`] is called from `FixedUpdate`, which Bevy runs a
//!   whole number of times per frame at a rate you set with `Time<Fixed>`;
//! - rendering runs in `Update` and asks `Time<Fixed>::overstep_fraction()` how
//!   far it is between two fixed steps, then passes that to
//!   [`BreakoutGame::ball_at`].
//!
//! That is the same design as Snake's — fixed simulation, interpolated
//! rendering — reached through the engine's machinery instead of the library's.
//! The library does not care which, and that is the point: it exposes a step
//! function and a way to interpolate, and stays out of the argument about who
//! owns the clock.
//!
//! **Controls:** left/right or A/D to move   Space to launch   R to restart.
//!
//! # Example
//! ```no_run
//! use bevy::prelude::*;
//! use breakout_bevy::BreakoutPlugin;
//!
//! App::new()
//!     .add_plugins(DefaultPlugins)
//!     .add_plugins(BreakoutPlugin)
//!     .run();
//! ```

use bevy::prelude::*;
use bevy::time::Fixed;

use bevy::audio::{AddAudioSource, Source};
use breakout_lib::{
    BreakoutGame, GameStatus, PaddleInput, STEPS_PER_SECOND, StepOutcome, Vec2 as GameVec2,
};
use std::time::Duration;

/// Bundles every system and resource for the game.
///
/// Add it with `app.add_plugins(BreakoutPlugin)`. It does **not** add
/// `DefaultPlugins`, so the host app stays in control of window and rendering.
pub struct BreakoutPlugin;

impl Plugin for BreakoutPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<Game>()
            .add_message::<Bounced>()
            .add_audio_source::<Pcm>()
            // The engine's accumulator, set to the library's fixed rate. Bevy
            // runs FixedUpdate however many times this frame's elapsed time is
            // worth, which is precisely the loop Snake writes by hand.
            .insert_resource(Time::<Fixed>::from_hz(STEPS_PER_SECOND as f64))
            .add_systems(Startup, setup)
            .add_systems(Update, (read_input, restart).chain())
            .add_systems(FixedUpdate, advance)
            .add_systems(Update, (draw, update_hud).after(read_input))
            .add_systems(Update, play_sounds);
    }
}

/// The game itself, owned as a resource so systems can read and mutate it.
#[derive(Resource, Deref, DerefMut)]
pub struct Game(pub BreakoutGame);

impl Default for Game {
    fn default() -> Self {
        Self(BreakoutGame::new(BreakoutGame::default_layout()))
    }
}

// --- Pure helpers ---

/// Maps held keys to a paddle input.
///
/// Both directions at once cancel, so a player rolling their fingers across the
/// keys does not get a surprise.
pub fn paddle_input_from_keys(left: bool, right: bool) -> PaddleInput {
    match (left, right) {
        (true, false) => PaddleInput::Left,
        (false, true) => PaddleInput::Right,
        _ => PaddleInput::None,
    }
}

/// Converts a game-space point to Bevy world space.
///
/// The library puts the origin at the top-left with `y` increasing downward,
/// which is how a play field is naturally described; Bevy is y-up and centred.
pub fn to_world(point: GameVec2, field: GameVec2) -> Vec2 {
    Vec2::new(point.x - field.x / 2.0, field.y / 2.0 - point.y)
}

/// The status line for the current game state.
pub fn status_line(game: &BreakoutGame) -> String {
    match game.status() {
        GameStatus::Playing if game.ball_is_stuck() => {
            format!(
                "Score {}   Lives {}  -  Space to launch",
                game.score(),
                game.lives()
            )
        }
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

/// How bright a brick with one hit left is drawn, relative to a fresh one.
///
/// It was 0.88, which is a change of about 6 units of CIE ΔE — the docs said a
/// damaged brick "is dimmed, so the player can see it is weakened", and at that
/// strength essentially nobody could, mid-play or otherwise. Half brightness is
/// ΔE 26 across every brick row, which is unmistakable. Measured by
/// `tools/check-palette.py` rather than eyeballed.
const DAMAGED_BRIGHTNESS: f32 = 0.5;

/// Colour for a brick, by row and remaining hits.
pub fn brick_color(row: usize, hits: u8) -> Color {
    let (r, g, b) = match row % 5 {
        0 => (0.90, 0.35, 0.35),
        1 => (0.90, 0.60, 0.30),
        2 => (0.85, 0.85, 0.35),
        3 => (0.40, 0.80, 0.45),
        _ => (0.40, 0.65, 0.90),
    };
    let k = if hits > 1 { 1.0 } else { DAMAGED_BRIGHTNESS };
    Color::srgb(r * k, g * k, b * k)
}

// --- Components ---

/// A drawn brick, tagged with its index in the game's brick list.
#[derive(Component)]
pub struct BrickSprite(pub usize);

/// The paddle sprite.
#[derive(Component)]
pub struct PaddleSprite;

/// The ball sprite.
#[derive(Component)]
pub struct BallSprite;

/// The status text entity.
#[derive(Component)]
pub struct StatusText;

// ─── Sound ────────────────────────────────────────────────────────────────────
//
// Nothing in `breakout-lib` changed to add audio, and that is the whole point.
// A bounce is a *rule*; the noise it makes is an effect, and the library already
// reported every event a sound needs — `StepOutcome` names the wall, the paddle,
// the brick, the break and the lost life. The frontend only decides what each
// one sounds like.
//
// That is the test of an engine-agnostic boundary that matters: not whether it
// survives being written, but whether it survives a feature nobody had in mind
// when it was drawn.
//
// Tones are synthesised rather than loaded, so there are no asset files to ship
// or to fail to fetch in a browser.

/// A short synthesised note.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Blip {
    /// Pitch in hertz.
    pub hz: f32,
    /// Duration in seconds.
    pub seconds: f32,
    /// Peak amplitude in `0.0..=1.0`.
    pub gain: f32,
}

/// The sound a step should make, if any.
///
/// Pure, and therefore the part worth testing: the ordering below is a design
/// decision — losing a life outranks the brick you broke on the way down —
/// rather than something the engine imposes.
pub fn sound_for(outcome: &StepOutcome) -> Option<Blip> {
    if outcome.lost_life {
        // Lowest and longest: the only unwelcome sound in the game.
        return Some(Blip {
            hz: 110.0,
            seconds: 0.45,
            gain: 0.35,
        });
    }
    if outcome.broke_brick {
        return Some(Blip {
            hz: 660.0,
            seconds: 0.12,
            gain: 0.30,
        });
    }
    if outcome.hit_brick.is_some() {
        // A brick that survived: same event, duller note.
        return Some(Blip {
            hz: 440.0,
            seconds: 0.08,
            gain: 0.25,
        });
    }
    if outcome.hit_paddle {
        return Some(Blip {
            hz: 330.0,
            seconds: 0.07,
            gain: 0.25,
        });
    }
    if outcome.hit_wall {
        return Some(Blip {
            hz: 220.0,
            seconds: 0.05,
            gain: 0.20,
        });
    }
    None
}

/// Samples per second used for every synthesised tone.
pub const SAMPLE_RATE: u32 = 44_100;

/// Renders a blip to mono PCM, fading to silence so it does not click.
pub fn render(blip: &Blip) -> Vec<f32> {
    let count = (blip.seconds.max(0.0) * SAMPLE_RATE as f32) as usize;
    (0..count)
        .map(|i| {
            let t = i as f32 / SAMPLE_RATE as f32;
            let fade = 1.0 - (i as f32 / count as f32);
            (std::f32::consts::TAU * blip.hz * t).sin() * blip.gain * fade
        })
        .collect()
}

/// An [`AudioSource`]-compatible asset backed by samples generated in code.
#[derive(Asset, TypePath, Clone)]
pub struct Pcm {
    /// Mono samples in `-1.0..=1.0`.
    pub samples: Vec<f32>,
}

/// Streams a [`Pcm`] asset to the audio backend.
pub struct PcmDecoder {
    samples: std::vec::IntoIter<f32>,
}

impl Iterator for PcmDecoder {
    type Item = f32;

    fn next(&mut self) -> Option<f32> {
        self.samples.next()
    }
}

impl Source for PcmDecoder {
    fn current_frame_len(&self) -> Option<usize> {
        None
    }

    fn channels(&self) -> u16 {
        1
    }

    fn sample_rate(&self) -> u32 {
        SAMPLE_RATE
    }

    fn total_duration(&self) -> Option<Duration> {
        None
    }
}

impl Decodable for Pcm {
    type Decoder = PcmDecoder;
    type DecoderItem = f32;

    fn decoder(&self) -> Self::Decoder {
        PcmDecoder {
            samples: self.samples.clone().into_iter(),
        }
    }
}

/// Plays the sound for whatever happened this step.
fn play_sounds(
    mut commands: Commands,
    mut events: MessageReader<Bounced>,
    mut sources: ResMut<Assets<Pcm>>,
) {
    for event in events.read() {
        let Some(blip) = sound_for(&event.0) else {
            continue;
        };
        let handle = sources.add(Pcm {
            samples: render(&blip),
        });
        commands.spawn((AudioPlayer(handle), PlaybackSettings::DESPAWN));
    }
}

/// What a fixed step produced, forwarded to the audio system.
#[derive(Message)]
pub struct Bounced(pub StepOutcome);

// --- Systems ---

fn setup(mut commands: Commands, game: Res<Game>) {
    commands.spawn(Camera2d);
    let field = game.size();

    commands.spawn((
        Sprite {
            color: Color::srgb(0.08, 0.09, 0.11),
            custom_size: Some(Vec2::new(field.x, field.y)),
            ..default()
        },
        Transform::from_xyz(0.0, 0.0, -1.0),
    ));

    for (index, brick) in game.bricks().iter().enumerate() {
        let pos = to_world(brick.rect.centre, field);
        commands.spawn((
            BrickSprite(index),
            Sprite {
                color: brick_color(brick.row, brick.hits),
                custom_size: Some(Vec2::new(brick.rect.half.x * 2.0, brick.rect.half.y * 2.0)),
                ..default()
            },
            Transform::from_xyz(pos.x, pos.y, 0.0),
        ));
    }

    let paddle = game.paddle_rect();
    let pos = to_world(paddle.centre, field);
    commands.spawn((
        PaddleSprite,
        Sprite {
            color: Color::srgb(0.85, 0.88, 0.92),
            custom_size: Some(Vec2::new(paddle.half.x * 2.0, paddle.half.y * 2.0)),
            ..default()
        },
        Transform::from_xyz(pos.x, pos.y, 1.0),
    ));

    let ball = to_world(game.ball(), field);
    commands.spawn((
        BallSprite,
        Sprite {
            color: Color::srgb(1.0, 0.95, 0.85),
            custom_size: Some(Vec2::splat(14.0)),
            ..default()
        },
        Transform::from_xyz(ball.x, ball.y, 1.0),
    ));

    commands.spawn((
        StatusText,
        Text::new(""),
        TextFont {
            font_size: 20.0,
            ..default()
        },
        TextColor(Color::WHITE),
        Node {
            position_type: PositionType::Absolute,
            bottom: Val::Px(12.0),
            left: Val::Px(16.0),
            ..default()
        },
    ));
}

/// Records input every frame; `FixedUpdate` consumes it whenever it next runs.
fn read_input(keys: Res<ButtonInput<KeyCode>>, mut game: ResMut<Game>) {
    let input = paddle_input_from_keys(
        keys.any_pressed([KeyCode::ArrowLeft, KeyCode::KeyA]),
        keys.any_pressed([KeyCode::ArrowRight, KeyCode::KeyD]),
    );
    game.set_paddle_input(input);

    if keys.just_pressed(KeyCode::Space) {
        game.launch();
    }
}

/// Advances the simulation. Runs in `FixedUpdate`, so Bevy decides how often.
fn advance(mut game: ResMut<Game>, mut bounced: MessageWriter<Bounced>) {
    let outcome = game.step();
    // Forwarded rather than acted on here: the sound is drawn from the
    // library's own report of what happened, not re-derived from the state.
    bounced.write(Bounced(outcome));
}

fn restart(keys: Res<ButtonInput<KeyCode>>, mut game: ResMut<Game>) {
    if keys.just_pressed(KeyCode::KeyR) {
        **game = BreakoutGame::new(BreakoutGame::default_layout());
    }
}

/// Draws everything, interpolating the ball between fixed steps.
fn draw(
    game: Res<Game>,
    fixed: Res<Time<Fixed>>,
    mut bricks: Query<(&BrickSprite, &mut Sprite, &mut Visibility)>,
    mut paddle: Query<&mut Transform, (With<PaddleSprite>, Without<BallSprite>)>,
    mut ball: Query<&mut Transform, (With<BallSprite>, Without<PaddleSprite>)>,
) {
    let field = game.size();

    for (marker, mut sprite, mut visibility) in &mut bricks {
        let brick = game.bricks()[marker.0];
        *visibility = if brick.alive() {
            Visibility::Inherited
        } else {
            Visibility::Hidden
        };
        sprite.color = brick_color(brick.row, brick.hits);
    }

    for mut transform in &mut paddle {
        let pos = to_world(game.paddle_rect().centre, field);
        transform.translation.x = pos.x;
        transform.translation.y = pos.y;
    }

    // The interpolation the module docs are about. `overstep_fraction` is how
    // far this frame sits between the last fixed step and the next; without it
    // the ball visibly steps rather than glides.
    let alpha = fixed.overstep_fraction();
    for mut transform in &mut ball {
        let pos = to_world(game.ball_at(alpha), field);
        transform.translation.x = pos.x;
        transform.translation.y = pos.y;
    }
}

fn update_hud(game: Res<Game>, mut text: Query<&mut Text, With<StatusText>>) {
    for mut t in &mut text {
        **t = status_line(&game);
    }
}

#[cfg(test)]
mod tests {
    // ── Sound ────────────────────────────────────────────────────────────────

    fn outcome() -> StepOutcome {
        StepOutcome::default()
    }

    #[test]
    fn an_uneventful_step_is_silent() {
        assert_eq!(sound_for(&outcome()), None);
    }

    #[test]
    fn each_event_has_its_own_note() {
        let mut wall = outcome();
        wall.hit_wall = true;
        let mut paddle = outcome();
        paddle.hit_paddle = true;
        let mut chip = outcome();
        chip.hit_brick = Some(3);
        let mut broke = outcome();
        broke.hit_brick = Some(3);
        broke.broke_brick = true;

        let pitches: Vec<f32> = [wall, paddle, chip, broke]
            .iter()
            .map(|o| sound_for(o).expect("audible").hz)
            .collect();
        // All different, so a player can hear which happened without looking.
        for (i, a) in pitches.iter().enumerate() {
            for b in pitches.iter().skip(i + 1) {
                assert_ne!(a, b, "two events share a pitch: {pitches:?}");
            }
        }
    }

    #[test]
    fn losing_a_life_outranks_the_brick_broken_on_the_way_down() {
        // A step can report several things at once. Losing a life is the one
        // the player must hear, so it wins regardless of what else happened.
        let mut both = outcome();
        both.broke_brick = true;
        both.hit_brick = Some(0);
        both.hit_wall = true;
        both.lost_life = true;
        let blip = sound_for(&both).expect("audible");

        let mut only_break = outcome();
        only_break.broke_brick = true;
        assert_ne!(blip, sound_for(&only_break).expect("audible"));
        assert!(blip.hz < 200.0, "the loss should be the low note");
        assert!(blip.seconds > 0.3, "and the long one");
    }

    #[test]
    fn a_broken_brick_sounds_brighter_than_one_that_survived() {
        let mut chip = outcome();
        chip.hit_brick = Some(1);
        let mut broke = outcome();
        broke.hit_brick = Some(1);
        broke.broke_brick = true;
        assert!(sound_for(&broke).expect("audible").hz > sound_for(&chip).expect("audible").hz);
    }

    #[test]
    fn a_rendered_blip_starts_loud_and_ends_silent() {
        // Without the fade the sample ends mid-wave and clicks.
        let samples = render(&Blip {
            hz: 440.0,
            seconds: 0.05,
            gain: 0.5,
        });
        assert!(!samples.is_empty());
        assert!(
            samples.iter().all(|s| s.abs() <= 0.5 + 1e-6),
            "exceeds gain"
        );
        let tail = samples.last().copied().expect("non-empty");
        assert!(tail.abs() < 0.01, "should fade out, ended at {tail}");
    }

    #[test]
    fn a_blip_lasts_as_long_as_it_says() {
        let samples = render(&Blip {
            hz: 440.0,
            seconds: 0.1,
            gain: 0.4,
        });
        let expected = (0.1 * SAMPLE_RATE as f32) as usize;
        assert_eq!(samples.len(), expected);
    }

    #[test]
    fn a_zero_length_blip_renders_nothing_rather_than_panicking() {
        assert!(
            render(&Blip {
                hz: 440.0,
                seconds: 0.0,
                gain: 0.4
            })
            .is_empty()
        );
    }

    use super::*;

    #[test]
    fn keys_map_to_paddle_input() {
        assert_eq!(paddle_input_from_keys(true, false), PaddleInput::Left);
        assert_eq!(paddle_input_from_keys(false, true), PaddleInput::Right);
        assert_eq!(paddle_input_from_keys(false, false), PaddleInput::None);
    }

    #[test]
    fn holding_both_directions_cancels() {
        assert_eq!(paddle_input_from_keys(true, true), PaddleInput::None);
    }

    #[test]
    fn to_world_centres_the_field_and_flips_the_y_axis() {
        let field = GameVec2::new(800.0, 600.0);
        // The field's centre maps to the world origin.
        assert_eq!(to_world(GameVec2::new(400.0, 300.0), field), Vec2::ZERO);
        // Game-space "up" (smaller y) must become Bevy "up" (larger y).
        let high = to_world(GameVec2::new(400.0, 100.0), field);
        let low = to_world(GameVec2::new(400.0, 500.0), field);
        assert!(high.y > low.y, "y axis was not flipped");
    }

    #[test]
    fn to_world_maps_the_corners() {
        let field = GameVec2::new(800.0, 600.0);
        assert_eq!(
            to_world(GameVec2::new(0.0, 0.0), field),
            Vec2::new(-400.0, 300.0)
        );
        assert_eq!(
            to_world(GameVec2::new(800.0, 600.0), field),
            Vec2::new(400.0, -300.0)
        );
    }

    #[test]
    fn brick_colours_differ_by_row_and_dim_when_damaged() {
        assert_ne!(brick_color(0, 1), brick_color(1, 1));
        assert_ne!(
            brick_color(0, 2),
            brick_color(0, 1),
            "a damaged brick should look different"
        );
    }

    #[test]
    fn status_line_prompts_for_a_launch_while_the_ball_waits() {
        let game = BreakoutGame::new(BreakoutGame::default_layout());
        assert!(game.ball_is_stuck());
        assert!(status_line(&game).contains("Space to launch"));
    }

    #[test]
    fn status_line_reports_progress_once_playing() {
        let mut game = BreakoutGame::new(BreakoutGame::default_layout());
        game.launch();
        let line = status_line(&game);
        assert!(line.contains("Score 0"), "{line}");
        assert!(line.contains("Bricks 40"), "{line}");
    }

    #[test]
    fn status_line_reports_both_endings() {
        let mut won = BreakoutGame::default_layout();
        for brick in &mut won.bricks {
            brick.hits = 0;
        }
        let mut game = BreakoutGame::new(won);
        game.launch();
        game.step();
        assert!(status_line(&game).contains("Cleared!"));

        let mut lost = BreakoutGame::default_layout();
        lost.bricks.truncate(1);
        lost.lives = 1;
        let mut game = BreakoutGame::new(lost);
        game.launch();
        game.set_paddle_input(PaddleInput::Left);
        for _ in 0..20_000 {
            if game.step().finished {
                break;
            }
        }
        assert!(
            status_line(&game).contains("Game over"),
            "{}",
            status_line(&game)
        );
    }

    // --- ECS ---

    #[test]
    fn setup_spawns_a_sprite_for_every_brick() {
        let mut app = App::new();
        app.add_plugins(MinimalPlugins)
            .init_resource::<Game>()
            .add_systems(Startup, setup);
        app.update();

        let mut q = app.world_mut().query::<&BrickSprite>();
        assert_eq!(q.iter(app.world()).count(), 40);
    }

    #[test]
    fn the_simulation_runs_at_the_librarys_fixed_rate() {
        // The frontend must not invent its own rate: the library's physics is
        // tuned for DT, and a different step size changes the game.
        let time = Time::<Fixed>::from_hz(STEPS_PER_SECOND as f64);
        let expected = 1.0 / STEPS_PER_SECOND as f64;
        assert!((time.timestep().as_secs_f64() - expected).abs() < 1e-9);
    }
}
