//! Achievement system — a reusable Bevy plugin.
//!
//! This crate is a *building block*: drop [`AchievementSystemPlugin`] into any
//! Bevy app with `app.add_plugins(AchievementSystemPlugin)` and it tracks
//! progress-based milestones and fires unlock toasts.
//!
//! Key ideas:
//! - A [`AchievementRegistry`] resource holds all achievements as a `Vec`.
//!   Each has a `goal`, a live `progress`, and an `unlocked` flag.
//! - [`is_unlocked`] and [`achievement_fraction`] are pure functions used for
//!   both unlock detection and HUD progress-bar rendering.
//! - `check_achievements` runs every frame, comparing the latest counters
//!   against each achievement's goal and firing an [`UnlockMsg`] on the first
//!   frame a goal is reached.
//! - HUD rows are individual ECS entities; `update_hud` rewrites them only
//!   when the registry has changed, keeping frame cost low.
//!
//! **Controls:** SPACE — score a point; W/A/S/D — count movement steps;
//! K — simulate a kill.
//!
//! # Example
//! ```no_run
//! use bevy::prelude::*;
//! use achievement_system::AchievementSystemPlugin;
//!
//! App::new()
//!     .add_plugins(DefaultPlugins)
//!     .add_plugins(AchievementSystemPlugin)
//!     .run();
//! ```

use bevy::prelude::*;

/// Bundles every system, message, and resource for the achievement feature.
///
/// Add it with `app.add_plugins(AchievementSystemPlugin)`. It does **not** add
/// `DefaultPlugins`, so the host app stays in control of window and rendering
/// setup.
pub struct AchievementSystemPlugin;

impl Plugin for AchievementSystemPlugin {
    fn build(&self, app: &mut App) {
        app.add_message::<UnlockMsg>()
            .insert_resource(AchievementRegistry(vec![
                Achievement::new("First Point",   1),
                Achievement::new("Scorer",        10),
                Achievement::new("High Scorer",   50),
                Achievement::new("First Step",    1),
                Achievement::new("Walker",        25),
                Achievement::new("First Kill",    1),
                Achievement::new("Monster Slayer",10),
            ]))
            .init_resource::<Score>()
            .init_resource::<StepCount>()
            .init_resource::<KillCount>()
            .add_systems(Startup, setup)
            .add_systems(
                Update,
                (handle_input, check_achievements, update_hud, announce_unlocks).chain(),
            );
    }
}

// ── Pure achievement logic ────────────────────────────────────────────────────

/// Returns the fraction of the goal that has been completed, clamped to `[0, 1]`.
///
/// A zero `goal` is treated as already complete.
pub fn achievement_fraction(progress: u32, goal: u32) -> f32 {
    if goal == 0 {
        return 1.0;
    }
    (progress as f32 / goal as f32).min(1.0)
}

/// Returns `true` when `progress` has met or exceeded `goal`.
pub fn is_unlocked(progress: u32, goal: u32) -> bool {
    progress >= goal
}

/// Formats `"Name — progress / goal"` for the HUD (progress clamped at goal).
pub fn progress_text(name: &str, progress: u32, goal: u32) -> String {
    format!("{} — {} / {}", name, progress.min(goal), goal)
}

// ── Data model ────────────────────────────────────────────────────────────────

/// A single trackable achievement.
pub struct Achievement {
    /// Display name.
    pub name: &'static str,
    /// Counter value at which the achievement unlocks.
    pub goal: u32,
    /// Best progress seen so far.
    pub progress: u32,
    /// Whether the achievement has been unlocked.
    pub unlocked: bool,
}

impl Achievement {
    /// Creates a fresh, locked achievement at zero progress.
    pub fn new(name: &'static str, goal: u32) -> Self {
        Self { name, goal, progress: 0, unlocked: false }
    }
}

// ── ECS ───────────────────────────────────────────────────────────────────────

/// Registry of all achievements for the session.
#[derive(Resource)]
pub struct AchievementRegistry(pub Vec<Achievement>);

/// Cumulative score for the session.
#[derive(Resource, Default)]
pub struct Score(pub u32);

/// Cumulative WASD step count.
#[derive(Resource, Default)]
pub struct StepCount(pub u32);

/// Cumulative kill count.
#[derive(Resource, Default)]
pub struct KillCount(pub u32);

/// Fired when an achievement unlocks for the first time.
#[derive(Message)]
pub struct UnlockMsg(pub String);

/// Tags a HUD row entity with the achievement index it represents.
#[derive(Component)]
struct AchievementRow(usize);

/// Spawns the camera, hint label, and one HUD row per achievement.
fn setup(mut commands: Commands, registry: Res<AchievementRegistry>) {
    commands.spawn(Camera2d);

    commands.spawn((
        Text::new("SPACE = score   WASD = steps   K = kill"),
        TextFont { font_size: 15.0, ..default() },
        TextColor(Color::srgb(0.55, 0.55, 0.55)),
        Node {
            position_type: PositionType::Absolute,
            top: Val::Px(12.0),
            left: Val::Px(12.0),
            ..default()
        },
    ));

    for (i, ach) in registry.0.iter().enumerate() {
        commands.spawn((
            Text::new(progress_text(ach.name, ach.progress, ach.goal)),
            TextFont { font_size: 16.0, ..default() },
            TextColor(Color::srgb(0.78, 0.78, 0.78)),
            Node {
                position_type: PositionType::Absolute,
                top: Val::Px(46.0 + i as f32 * 30.0),
                left: Val::Px(20.0),
                ..default()
            },
            AchievementRow(i),
        ));
    }
}

/// Reads key input and increments the appropriate counters.
fn handle_input(
    input: Res<ButtonInput<KeyCode>>,
    mut score: ResMut<Score>,
    mut steps: ResMut<StepCount>,
    mut kills: ResMut<KillCount>,
) {
    if input.just_pressed(KeyCode::Space) {
        score.0 += 1;
    }
    for key in [KeyCode::KeyW, KeyCode::KeyA, KeyCode::KeyS, KeyCode::KeyD] {
        if input.just_pressed(key) {
            steps.0 += 1;
        }
    }
    if input.just_pressed(KeyCode::KeyK) {
        kills.0 += 1;
    }
}

/// Compares counters against each achievement's goal and fires [`UnlockMsg`] on first unlock.
fn check_achievements(
    score: Res<Score>,
    steps: Res<StepCount>,
    kills: Res<KillCount>,
    mut registry: ResMut<AchievementRegistry>,
    mut writer: MessageWriter<UnlockMsg>,
) {
    // Map each achievement index to the relevant counter.
    let counters: [u32; 7] = [
        score.0, score.0, score.0,  // First Point, Scorer, High Scorer
        steps.0, steps.0,           // First Step, Walker
        kills.0, kills.0,           // First Kill, Monster Slayer
    ];
    for (i, &counter) in counters.iter().enumerate() {
        let ach = &mut registry.0[i];
        ach.progress = ach.progress.max(counter);
        if !ach.unlocked && is_unlocked(ach.progress, ach.goal) {
            ach.unlocked = true;
            writer.write(UnlockMsg(format!("[Achievement unlocked] {}", ach.name)));
        }
    }
}

/// Rewrites HUD rows — text and colour — when the registry changes.
fn update_hud(
    registry: Res<AchievementRegistry>,
    mut rows: Query<(&AchievementRow, &mut Text, &mut TextColor)>,
) {
    if !registry.is_changed() {
        return;
    }
    for (row, mut text, mut color) in &mut rows {
        let ach = &registry.0[row.0];
        *text = Text::new(progress_text(ach.name, ach.progress, ach.goal));
        color.0 = if ach.unlocked {
            Color::srgb(1.0, 0.85, 0.2) // gold when unlocked
        } else {
            Color::srgb(0.78, 0.78, 0.78)
        };
    }
}

/// Prints unlock messages to the Bevy console log.
fn announce_unlocks(mut reader: MessageReader<UnlockMsg>) {
    for msg in reader.read() {
        info!("{}", msg.0);
    }
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn achievement_fraction_at_zero_progress() {
        assert_eq!(achievement_fraction(0, 10), 0.0);
    }

    #[test]
    fn achievement_fraction_at_goal() {
        assert_eq!(achievement_fraction(10, 10), 1.0);
    }

    #[test]
    fn achievement_fraction_halfway() {
        assert!((achievement_fraction(5, 10) - 0.5).abs() < 1e-6);
    }

    #[test]
    fn achievement_fraction_clamps_above_goal() {
        assert_eq!(achievement_fraction(20, 10), 1.0);
    }

    #[test]
    fn achievement_fraction_zero_goal_is_complete() {
        assert_eq!(achievement_fraction(0, 0), 1.0);
    }

    #[test]
    fn is_unlocked_at_exact_goal() {
        assert!(is_unlocked(10, 10));
    }

    #[test]
    fn is_unlocked_above_goal() {
        assert!(is_unlocked(15, 10));
    }

    #[test]
    fn is_not_unlocked_below_goal() {
        assert!(!is_unlocked(9, 10));
    }

    #[test]
    fn progress_text_formats_correctly() {
        assert_eq!(progress_text("Scorer", 3, 10), "Scorer — 3 / 10");
    }

    #[test]
    fn progress_text_clamps_display_at_goal() {
        assert_eq!(progress_text("Test", 25, 10), "Test — 10 / 10");
    }

    #[test]
    fn all_achievements_start_locked() {
        let ach = Achievement::new("Test", 5);
        assert!(!ach.unlocked);
        assert_eq!(ach.progress, 0);
    }

    #[test]
    fn setup_spawns_one_row_per_achievement() {
        let mut app = App::new();
        app.add_plugins(MinimalPlugins)
            .insert_resource(AchievementRegistry(vec![
                Achievement::new("A", 1),
                Achievement::new("B", 2),
                Achievement::new("C", 3),
            ]))
            .add_systems(Startup, setup);
        app.update();

        let mut q = app.world_mut().query::<&AchievementRow>();
        assert_eq!(q.iter(app.world()).count(), 3);
    }
}
