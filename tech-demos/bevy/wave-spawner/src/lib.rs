//! Wave-spawner demo — a reusable Bevy plugin.
//!
//! This crate is a *building block*: drop [`WaveSpawnerPlugin`] into any Bevy
//! app with `app.add_plugins(WaveSpawnerPlugin)` and it runs escalating enemy
//! waves with an inter-wave countdown and a live HUD. Tune the wave/countdown
//! durations and spawn radius through [`WaveSpawnerConfig`].
//!
//! Key ideas:
//! - [`WaveState`] tracks whether the simulation is in an active-wave phase or a
//!   between-wave countdown phase.
//! - Enemy count scales with wave number; enemies are despawned at wave end.
//! - Enemies are placed around the arena boundary using the golden-ratio angle
//!   to distribute them evenly without repeating clusters.
//! - The HUD shows the current wave, enemy count, and time remaining.
//!
//! **Controls:** passive — watch the waves spawn automatically.
//!
//! # Example
//! ```no_run
//! use bevy::prelude::*;
//! use wave_spawner::WaveSpawnerPlugin;
//!
//! App::new()
//!     .add_plugins(DefaultPlugins)
//!     .add_plugins(WaveSpawnerPlugin)
//!     .run();
//! ```

use bevy::prelude::*;

/// Bundles every system and resource for the wave-spawner feature.
///
/// Add it with `app.add_plugins(WaveSpawnerPlugin)`. It does **not** add
/// `DefaultPlugins`, so the host app stays in control of window and rendering
/// setup.
pub struct WaveSpawnerPlugin;

impl Plugin for WaveSpawnerPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<WaveSpawnerConfig>()
            .init_resource::<WaveState>()
            .add_systems(Startup, setup)
            .add_systems(Update, (tick_wave, update_hud));
    }
}

// ── Configuration ─────────────────────────────────────────────────────────────

/// Golden angle in radians (≈137.5°) for evenly distributed spawns.
const GOLDEN_ANGLE: f32 = 2.399_963;

/// Tunable parameters for the wave spawner. Override before adding the plugin,
/// e.g. `app.insert_resource(WaveSpawnerConfig { wave_duration: 12.0, ..default() })`.
#[derive(Resource, Clone, Copy, Debug)]
pub struct WaveSpawnerConfig {
    /// Duration of each active wave in seconds.
    pub wave_duration: f32,
    /// Countdown between waves in seconds.
    pub countdown_duration: f32,
    /// Spawn radius for enemies.
    pub spawn_radius: f32,
}

impl Default for WaveSpawnerConfig {
    fn default() -> Self {
        Self { wave_duration: 8.0, countdown_duration: 3.0, spawn_radius: 280.0 }
    }
}

/// Enemies to spawn in wave *n* (1-based).
pub fn enemy_count_for_wave(wave: u32) -> u32 {
    3 + (wave - 1) * 2
}

/// Tracks the wave progression state.
#[derive(Resource)]
pub struct WaveState {
    pub wave: u32,
    pub phase: WavePhase,
    pub timer: f32,
}

/// Whether we're actively fighting or waiting between waves.
#[derive(PartialEq)]
pub enum WavePhase {
    Active,
    Countdown,
}

impl Default for WaveState {
    fn default() -> Self {
        Self { wave: 0, phase: WavePhase::Countdown, timer: 0.0 }
    }
}

/// Marker for enemy entities.
#[derive(Component)]
pub struct Enemy;

/// Marks the HUD text entities.
#[derive(Component)]
pub enum HudLabel {
    Wave,
    Count,
    Timer,
}

/// Spawns the camera and HUD.
fn setup(mut commands: Commands, config: Res<WaveSpawnerConfig>) {
    commands.spawn(Camera2d);

    let label_style = TextFont { font_size: 22.0, ..default() };

    commands.spawn((
        Text::new("Wave: 0"),
        label_style.clone(),
        TextColor(Color::WHITE),
        Node {
            position_type: PositionType::Absolute,
            top: Val::Px(12.0),
            left: Val::Px(12.0),
            ..default()
        },
        HudLabel::Wave,
    ));
    commands.spawn((
        Text::new("Enemies: 0"),
        label_style.clone(),
        TextColor(Color::WHITE),
        Node {
            position_type: PositionType::Absolute,
            top: Val::Px(40.0),
            left: Val::Px(12.0),
            ..default()
        },
        HudLabel::Count,
    ));
    commands.spawn((
        Text::new(format!("Next wave in: {:.1} s", config.countdown_duration)),
        label_style.clone(),
        TextColor(Color::srgb(1.0, 0.9, 0.3)),
        Node {
            position_type: PositionType::Absolute,
            top: Val::Px(68.0),
            left: Val::Px(12.0),
            ..default()
        },
        HudLabel::Timer,
    ));
}

/// Advances the wave timer and transitions between phases.
fn tick_wave(
    mut commands: Commands,
    time: Res<Time>,
    config: Res<WaveSpawnerConfig>,
    mut state: ResMut<WaveState>,
    enemy_query: Query<Entity, With<Enemy>>,
) {
    let dt = time.delta_secs();
    state.timer += dt;

    match state.phase {
        WavePhase::Countdown => {
            if state.timer >= config.countdown_duration {
                state.timer = 0.0;
                state.wave += 1;
                state.phase = WavePhase::Active;
                spawn_wave(&mut commands, state.wave, &config);
            }
        }
        WavePhase::Active => {
            if state.timer >= config.wave_duration {
                state.timer = 0.0;
                state.phase = WavePhase::Countdown;
                // Despawn remaining enemies.
                for entity in &enemy_query {
                    commands.entity(entity).despawn();
                }
            }
        }
    }
}

/// Spawns enemies for `wave` around the arena boundary using the golden angle.
fn spawn_wave(commands: &mut Commands, wave: u32, config: &WaveSpawnerConfig) {
    let count = enemy_count_for_wave(wave);
    for i in 0..count {
        let angle = i as f32 * GOLDEN_ANGLE;
        let pos = Vec3::new(
            config.spawn_radius * angle.cos(),
            config.spawn_radius * angle.sin(),
            0.0,
        );
        // Tint gets redder with each wave (clamped so it stays in range).
        let redness = (0.5 + wave as f32 * 0.1).min(1.0);
        commands.spawn((
            Sprite { color: Color::srgb(redness, 0.2, 0.2), custom_size: Some(Vec2::splat(18.0)), ..default() },
            Transform::from_translation(pos),
            Enemy,
        ));
    }
}

/// Keeps the HUD labels current.
fn update_hud(
    state: Res<WaveState>,
    config: Res<WaveSpawnerConfig>,
    enemy_query: Query<&Enemy>,
    mut hud_query: Query<(&mut Text, &HudLabel)>,
) {
    for (mut text, label) in &mut hud_query {
        match label {
            HudLabel::Wave => text.0 = format!("Wave: {}", state.wave),
            HudLabel::Count => text.0 = format!("Enemies: {}", enemy_query.iter().count()),
            HudLabel::Timer => {
                let remaining = match state.phase {
                    WavePhase::Countdown => config.countdown_duration - state.timer,
                    WavePhase::Active => config.wave_duration - state.timer,
                };
                let label_str = if state.phase == WavePhase::Active { "Wave ends in" } else { "Next wave in" };
                text.0 = format!("{}: {:.1} s", label_str, remaining.max(0.0));
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn wave_1_spawns_three_enemies() {
        assert_eq!(enemy_count_for_wave(1), 3);
    }

    #[test]
    fn enemy_count_grows_by_two_per_wave() {
        for w in 1..=5 {
            let expected = 3 + (w - 1) * 2;
            assert_eq!(enemy_count_for_wave(w), expected, "wave {}", w);
        }
    }

    #[test]
    fn enemy_count_is_always_positive() {
        for w in 1..=20 {
            assert!(enemy_count_for_wave(w) > 0);
        }
    }

    #[test]
    fn spawn_radius_is_positive() {
        assert!(WaveSpawnerConfig::default().spawn_radius > 0.0);
    }

    #[test]
    fn wave_duration_and_countdown_are_positive() {
        let c = WaveSpawnerConfig::default();
        assert!(c.wave_duration > 0.0);
        assert!(c.countdown_duration > 0.0);
    }

    #[test]
    fn golden_angle_is_in_valid_radian_range() {
        assert!(GOLDEN_ANGLE > 0.0 && GOLDEN_ANGLE < std::f32::consts::TAU);
    }

    #[test]
    fn default_wave_state_starts_at_wave_zero() {
        let state = WaveState::default();
        assert_eq!(state.wave, 0);
        assert_eq!(state.timer, 0.0);
    }

    #[test]
    fn config_default_matches_documented_values() {
        let c = WaveSpawnerConfig::default();
        assert_eq!(c.wave_duration, 8.0);
        assert_eq!(c.countdown_duration, 3.0);
        assert_eq!(c.spawn_radius, 280.0);
    }

    #[test]
    fn plugin_spawns_hud_labels() {
        let mut app = App::new();
        app.add_plugins((MinimalPlugins, WaveSpawnerPlugin));
        app.update();

        let mut q = app.world_mut().query::<&HudLabel>();
        assert_eq!(q.iter(app.world()).count(), 3);
    }
}
