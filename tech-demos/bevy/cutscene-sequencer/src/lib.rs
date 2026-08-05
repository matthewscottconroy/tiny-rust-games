//! # Cutscene Sequencer — a reusable Bevy plugin.
//!
//! This crate is a *building block*: drop [`CutsceneSequencerPlugin`] into any
//! Bevy app with `app.add_plugins(CutsceneSequencerPlugin)` and it plays a
//! data-driven cutscene in which a `Vec<CutsceneStep>` drives itself forward
//! using elapsed time tracked in a [`Cutscene`] resource. No coroutines or
//! async — pure ECS state machine.
//!
//! ## Sequence (plays on startup, SPACE to replay)
//! 1. Fade in from black       (0.5 s)
//! 2. Pan camera left → center (1.5 s, smooth lerp)
//! 3. Show subtitle text       (2.0 s)
//! 4. Pan camera center → right(1.5 s)
//! 5. Show subtitle text       (2.0 s)
//! 6. Fade out to black        (0.5 s)
//! 7. Show "Press SPACE to replay"
//!
//! ## Architecture
//! - `advance_cutscene` ticks `elapsed` and promotes `current` when the step
//!   duration is met.
//! - `apply_camera_step`, `apply_fade_step`, and `apply_text_step` read the
//!   current step and update their respective entities.
//! - `handle_replay` resets the resource on SPACE.
//!
//! Tune the sequence timings and camera pan through the
//! [`CutsceneSequencerConfig`] resource without editing the plugin's internals.
//!
//! # Example
//! ```no_run
//! use bevy::prelude::*;
//! use cutscene_sequencer::CutsceneSequencerPlugin;
//!
//! App::new()
//!     .add_plugins(DefaultPlugins)
//!     .add_plugins(CutsceneSequencerPlugin)
//!     .run();
//! ```

use bevy::prelude::*;

/// Bundles every system and resource for the cutscene-sequencer feature.
///
/// Add it with `app.add_plugins(CutsceneSequencerPlugin)`. It does **not** add
/// `DefaultPlugins`, so the host app stays in control of window and rendering
/// setup.
pub struct CutsceneSequencerPlugin;

impl Plugin for CutsceneSequencerPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<CutsceneSequencerConfig>()
            .add_systems(Startup, (init_cutscene, setup))
            .add_systems(
                Update,
                (
                    advance_cutscene,
                    apply_camera_step,
                    apply_fade_step,
                    apply_text_step,
                    handle_replay,
                    update_debug_hud,
                )
                    .chain(),
            );
    }
}

// ---------------------------------------------------------------------------
// Configuration
// ---------------------------------------------------------------------------

/// Tunable parameters for the cutscene. Override before adding the plugin, e.g.
/// `app.insert_resource(CutsceneSequencerConfig { fade_seconds: 1.0, ..default() })`.
#[derive(Resource, Clone, Copy, Debug)]
pub struct CutsceneSequencerConfig {
    /// Duration of each fade (in/out) step, in seconds.
    pub fade_seconds: f32,
    /// Duration of each camera pan step, in seconds.
    pub pan_seconds: f32,
    /// Duration each subtitle stays on screen, in seconds.
    pub text_seconds: f32,
    /// Camera position at the start of the first pan.
    pub pan_left: Vec2,
    /// Camera position at the middle of the sequence.
    pub pan_center: Vec2,
    /// Camera position at the end of the second pan.
    pub pan_right: Vec2,
}

impl Default for CutsceneSequencerConfig {
    fn default() -> Self {
        Self {
            fade_seconds: 0.5,
            pan_seconds: 1.5,
            text_seconds: 2.0,
            pan_left: Vec2::new(-300.0, 0.0),
            pan_center: Vec2::new(0.0, 0.0),
            pan_right: Vec2::new(300.0, 0.0),
        }
    }
}

// ---------------------------------------------------------------------------
// Pure functions
// ---------------------------------------------------------------------------

/// Returns the normalized progress `[0, 1]` of `elapsed` through `duration`.
/// Clamped so it never exceeds 1.0 or goes below 0.0.
pub fn step_progress(elapsed: f32, duration: f32) -> f32 {
    if duration <= 0.0 {
        return 1.0;
    }
    (elapsed / duration).clamp(0.0, 1.0)
}

/// Smoothstep curve: `3t² − 2t³`.  Input is clamped to `[0, 1]` before
/// evaluation, so passing values outside that range is safe.
pub fn smoothstep(t: f32) -> f32 {
    let t = t.clamp(0.0, 1.0);
    t * t * (3.0 - 2.0 * t)
}

/// Linearly interpolates between `a` and `b` by `t` (unclamped).
pub fn lerp_vec2(a: Vec2, b: Vec2, t: f32) -> Vec2 {
    a + (b - a) * t
}

/// Returns the camera world position for a `PanCamera` step at normalized
/// `progress`, using smoothstep easing.
pub fn pan_position(from: Vec2, to: Vec2, progress: f32) -> Vec2 {
    lerp_vec2(from, to, smoothstep(progress))
}

// ---------------------------------------------------------------------------
// Cutscene data types
// ---------------------------------------------------------------------------

/// A single step in the cutscene sequence.
#[derive(Clone, Debug)]
pub enum CutsceneStep {
    /// Fade the black overlay from opaque to transparent.
    FadeIn { duration: f32 },
    /// Fade the black overlay from transparent to opaque.
    FadeOut { duration: f32 },
    /// Animate the camera from `from` to `to` over `duration` seconds.
    PanCamera { from: Vec2, to: Vec2, duration: f32 },
    /// Show `text` for `duration` seconds.
    ShowText { text: String, duration: f32 },
    /// Pause for `duration` seconds with no other action.
    Wait { duration: f32 },
}

impl CutsceneStep {
    /// Returns the duration of this step in seconds.
    pub fn duration(&self) -> f32 {
        match self {
            Self::FadeIn { duration }
            | Self::FadeOut { duration }
            | Self::Wait { duration } => *duration,
            Self::PanCamera { duration, .. } => *duration,
            Self::ShowText { duration, .. } => *duration,
        }
    }
}

/// Drives the cutscene; lives as a `Resource`.
#[derive(Resource)]
pub struct Cutscene {
    pub steps: Vec<CutsceneStep>,
    pub current: usize,
    pub elapsed: f32,
    pub finished: bool,
}

impl Cutscene {
    /// Constructs the default story sequence from `config`.
    pub fn default_sequence(config: &CutsceneSequencerConfig) -> Self {
        Self {
            steps: vec![
                CutsceneStep::FadeIn { duration: config.fade_seconds },
                CutsceneStep::PanCamera {
                    from: config.pan_left,
                    to: config.pan_center,
                    duration: config.pan_seconds,
                },
                CutsceneStep::ShowText {
                    text: "Chapter 1: The Beginning".into(),
                    duration: config.text_seconds,
                },
                CutsceneStep::PanCamera {
                    from: config.pan_center,
                    to: config.pan_right,
                    duration: config.pan_seconds,
                },
                CutsceneStep::ShowText {
                    text: "Something stirs in the darkness...".into(),
                    duration: config.text_seconds,
                },
                CutsceneStep::FadeOut { duration: config.fade_seconds },
            ],
            current: 0,
            elapsed: 0.0,
            finished: false,
        }
    }
}

// ---------------------------------------------------------------------------
// ECS marker components
// ---------------------------------------------------------------------------

/// Marks the full-screen black fade overlay sprite.
#[derive(Component)]
struct FadeOverlay;

/// Marks the subtitle text entity.
#[derive(Component)]
struct SubtitleText;

/// Marks the "Press SPACE to replay" text entity.
#[derive(Component)]
struct ReplayPrompt;

/// Marks the HUD debug text entity.
#[derive(Component)]
struct DebugHud;

// ---------------------------------------------------------------------------
// Startup
// ---------------------------------------------------------------------------

/// Inserts the [`Cutscene`] resource built from [`CutsceneSequencerConfig`].
fn init_cutscene(mut commands: Commands, config: Res<CutsceneSequencerConfig>) {
    commands.insert_resource(Cutscene::default_sequence(&config));
}

fn setup(mut commands: Commands) {
    // Camera starts off to the left (first pan step will move it)
    commands.spawn(Camera2d);

    // Colored background
    commands.spawn((
        Sprite {
            color: Color::srgb(0.05, 0.08, 0.18),
            custom_size: Some(Vec2::new(2000.0, 2000.0)),
            ..default()
        },
        Transform::from_translation(Vec3::new(0.0, 0.0, -10.0)),
    ));

    // Left landmark
    commands.spawn((
        Sprite {
            color: Color::srgb(0.3, 0.5, 0.2),
            custom_size: Some(Vec2::new(60.0, 120.0)),
            ..default()
        },
        Transform::from_translation(Vec3::new(-300.0, -60.0, 0.0)),
    ));

    // Center landmark
    commands.spawn((
        Sprite {
            color: Color::srgb(0.6, 0.4, 0.1),
            custom_size: Some(Vec2::new(80.0, 160.0)),
            ..default()
        },
        Transform::from_translation(Vec3::new(0.0, -80.0, 0.0)),
    ));

    // Right landmark
    commands.spawn((
        Sprite {
            color: Color::srgb(0.2, 0.4, 0.6),
            custom_size: Some(Vec2::new(60.0, 100.0)),
            ..default()
        },
        Transform::from_translation(Vec3::new(300.0, -50.0, 0.0)),
    ));

    // Full-screen black fade overlay (z = 50 so it covers everything)
    commands.spawn((
        FadeOverlay,
        Sprite {
            color: Color::srgba(0.0, 0.0, 0.0, 1.0),
            custom_size: Some(Vec2::new(2000.0, 2000.0)),
            ..default()
        },
        Transform::from_translation(Vec3::new(0.0, 0.0, 50.0)),
    ));

    // Subtitle text (hidden until a ShowText step)
    commands.spawn((
        SubtitleText,
        Text::new(""),
        TextFont { font_size: 28.0, ..default() },
        TextColor(Color::WHITE),
        Node {
            position_type: PositionType::Absolute,
            bottom: Val::Px(60.0),
            left: Val::Px(0.0),
            right: Val::Px(0.0),
            justify_content: JustifyContent::Center,
            ..default()
        },
        Visibility::Hidden,
    ));

    // "Press SPACE to replay" prompt (shown when cutscene finishes)
    commands.spawn((
        ReplayPrompt,
        Text::new("Press SPACE to replay"),
        TextFont { font_size: 22.0, ..default() },
        TextColor(Color::srgb(0.8, 0.8, 0.8)),
        Node {
            position_type: PositionType::Absolute,
            bottom: Val::Px(20.0),
            left: Val::Px(0.0),
            right: Val::Px(0.0),
            justify_content: JustifyContent::Center,
            ..default()
        },
        Visibility::Hidden,
    ));

    // Debug HUD
    commands.spawn((
        DebugHud,
        Text::new(""),
        TextFont { font_size: 13.0, ..default() },
        TextColor(Color::srgb(0.6, 0.6, 0.6)),
        Node {
            position_type: PositionType::Absolute,
            top: Val::Px(6.0),
            left: Val::Px(8.0),
            ..default()
        },
    ));
}

// ---------------------------------------------------------------------------
// Update systems
// ---------------------------------------------------------------------------

/// Advances the cutscene timer and promotes the step index when the current
/// step's duration is exhausted.
fn advance_cutscene(mut cutscene: ResMut<Cutscene>, time: Res<Time>) {
    if cutscene.finished {
        return;
    }
    cutscene.elapsed += time.delta_secs();
    if cutscene.current < cutscene.steps.len() {
        let dur = cutscene.steps[cutscene.current].duration();
        if cutscene.elapsed >= dur {
            cutscene.current += 1;
            cutscene.elapsed = 0.0;
        }
        if cutscene.current >= cutscene.steps.len() {
            cutscene.finished = true;
        }
    }
}

/// Lerps the camera position when the current step is `PanCamera`.
fn apply_camera_step(
    cutscene: Res<Cutscene>,
    mut camera_q: Query<&mut Transform, With<Camera2d>>,
) {
    let Ok(mut cam_tf) = camera_q.single_mut() else { return };
    if cutscene.finished {
        return;
    }
    let Some(step) = cutscene.steps.get(cutscene.current) else { return };
    if let CutsceneStep::PanCamera { from, to, duration } = step {
        let progress = step_progress(cutscene.elapsed, *duration);
        let target = pan_position(*from, *to, progress);
        cam_tf.translation.x = target.x;
        cam_tf.translation.y = target.y;
    }
}

/// Adjusts the fade overlay alpha for `FadeIn` and `FadeOut` steps.
fn apply_fade_step(
    cutscene: Res<Cutscene>,
    mut overlay_q: Query<&mut Sprite, With<FadeOverlay>>,
) {
    let Ok(mut sprite) = overlay_q.single_mut() else { return };
    let alpha = if cutscene.finished {
        // Keep fully opaque after FadeOut (last step)
        1.0_f32
    } else {
        match cutscene.steps.get(cutscene.current) {
            Some(CutsceneStep::FadeIn { duration }) => {
                1.0 - step_progress(cutscene.elapsed, *duration)
            }
            Some(CutsceneStep::FadeOut { duration }) => {
                step_progress(cutscene.elapsed, *duration)
            }
            _ => 0.0, // Fully transparent during all other steps
        }
    };
    sprite.color = Color::srgba(0.0, 0.0, 0.0, alpha);
}

/// Shows or hides the subtitle text based on the current `ShowText` step.
fn apply_text_step(
    cutscene: Res<Cutscene>,
    mut subtitle_q: Query<(&mut Text, &mut Visibility), With<SubtitleText>>,
    mut replay_q: Query<&mut Visibility, (With<ReplayPrompt>, Without<SubtitleText>)>,
) {
    let Ok((mut text, mut vis)) = subtitle_q.single_mut() else { return };
    let Ok(mut replay_vis) = replay_q.single_mut() else { return };

    if cutscene.finished {
        *vis = Visibility::Hidden;
        *replay_vis = Visibility::Visible;
        return;
    }

    *replay_vis = Visibility::Hidden;

    match cutscene.steps.get(cutscene.current) {
        Some(CutsceneStep::ShowText { text: content, .. }) => {
            **text = content.clone();
            *vis = Visibility::Visible;
        }
        _ => {
            *vis = Visibility::Hidden;
        }
    }
}

/// Resets the cutscene to the beginning when SPACE is pressed.
fn handle_replay(keys: Res<ButtonInput<KeyCode>>, mut cutscene: ResMut<Cutscene>) {
    if keys.just_pressed(KeyCode::Space) {
        cutscene.current = 0;
        cutscene.elapsed = 0.0;
        cutscene.finished = false;
    }
}

/// Updates the corner debug text with step index and elapsed time.
fn update_debug_hud(
    cutscene: Res<Cutscene>,
    mut hud_q: Query<&mut Text, With<DebugHud>>,
) {
    let Ok(mut text) = hud_q.single_mut() else { return };
    if cutscene.finished {
        **text = "Cutscene finished".into();
    } else {
        **text = format!(
            "Step {}/{} | elapsed {:.2}s",
            cutscene.current + 1,
            cutscene.steps.len(),
            cutscene.elapsed
        );
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn smoothstep_at_zero() {
        assert!((smoothstep(0.0) - 0.0).abs() < 1e-6);
    }

    #[test]
    fn smoothstep_at_half() {
        assert!((smoothstep(0.5) - 0.5).abs() < 1e-6);
    }

    #[test]
    fn smoothstep_at_one() {
        assert!((smoothstep(1.0) - 1.0).abs() < 1e-6);
    }

    #[test]
    fn smoothstep_clamped_below_zero() {
        assert!((smoothstep(-5.0) - 0.0).abs() < 1e-6);
    }

    #[test]
    fn smoothstep_clamped_above_one() {
        assert!((smoothstep(10.0) - 1.0).abs() < 1e-6);
    }

    #[test]
    fn step_progress_mid_way() {
        let p = step_progress(0.75, 1.5);
        assert!((p - 0.5).abs() < 1e-5);
    }

    #[test]
    fn step_progress_clamps_to_one() {
        let p = step_progress(99.0, 1.0);
        assert!((p - 1.0).abs() < 1e-6);
    }

    #[test]
    fn step_progress_zero_duration_returns_one() {
        assert!((step_progress(0.0, 0.0) - 1.0).abs() < 1e-6);
    }

    #[test]
    fn pan_position_at_start() {
        let from = Vec2::new(-300.0, 0.0);
        let to = Vec2::new(0.0, 0.0);
        let result = pan_position(from, to, 0.0);
        assert!((result - from).length() < 1e-4);
    }

    #[test]
    fn pan_position_at_end() {
        let from = Vec2::new(-300.0, 0.0);
        let to = Vec2::new(0.0, 0.0);
        let result = pan_position(from, to, 1.0);
        assert!((result - to).length() < 1e-4);
    }

    #[test]
    fn lerp_vec2_midpoint() {
        let a = Vec2::new(0.0, 0.0);
        let b = Vec2::new(10.0, 20.0);
        let mid = lerp_vec2(a, b, 0.5);
        assert!((mid.x - 5.0).abs() < 1e-5);
        assert!((mid.y - 10.0).abs() < 1e-5);
    }

    #[test]
    fn cutscene_step_duration_extraction() {
        let step = CutsceneStep::PanCamera {
            from: Vec2::ZERO,
            to: Vec2::ONE,
            duration: 1.5,
        };
        assert!((step.duration() - 1.5).abs() < 1e-6);

        let step2 = CutsceneStep::ShowText {
            text: "hello".into(),
            duration: 2.0,
        };
        assert!((step2.duration() - 2.0).abs() < 1e-6);
    }

    #[test]
    fn config_default_matches_documented_values() {
        let c = CutsceneSequencerConfig::default();
        assert!((c.fade_seconds - 0.5).abs() < 1e-6);
        assert!((c.pan_seconds - 1.5).abs() < 1e-6);
        assert!((c.text_seconds - 2.0).abs() < 1e-6);
    }

    #[test]
    fn default_sequence_has_six_steps() {
        let cfg = CutsceneSequencerConfig::default();
        let cutscene = Cutscene::default_sequence(&cfg);
        assert_eq!(cutscene.steps.len(), 6);
        assert_eq!(cutscene.current, 0);
        assert!(!cutscene.finished);
    }

    #[test]
    fn config_threads_into_sequence_durations() {
        let cfg = CutsceneSequencerConfig { fade_seconds: 3.0, ..Default::default() };
        let cutscene = Cutscene::default_sequence(&cfg);
        // First step is FadeIn using fade_seconds.
        assert!((cutscene.steps[0].duration() - 3.0).abs() < 1e-6);
    }
}
