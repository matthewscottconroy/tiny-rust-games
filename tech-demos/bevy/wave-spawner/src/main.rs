//! Wave-spawner demo — escalating enemy waves with an inter-wave countdown.
//!
//! Key ideas:
//! - `WaveState` tracks whether the simulation is in an active-wave phase or a
//!   between-wave countdown phase.
//! - Enemy count scales with wave number; enemies are despawned at wave end.
//! - Enemies are placed around the arena boundary using the golden-ratio angle
//!   to distribute them evenly without repeating clusters.
//! - The HUD shows the current wave, enemy count, and time remaining.
//!
//! **Controls:** passive — watch the waves spawn automatically.

use bevy::prelude::*;

/// Enemies to spawn in wave *n* (1-based).
pub fn enemy_count_for_wave(wave: u32) -> u32 {
    3 + (wave - 1) * 2
}

/// Duration of each active wave in seconds.
const WAVE_DURATION: f32 = 8.0;
/// Countdown between waves in seconds.
const COUNTDOWN_DURATION: f32 = 3.0;
/// Spawn radius for enemies.
const SPAWN_RADIUS: f32 = 280.0;

/// Golden angle in radians (≈137.5°) for evenly distributed spawns.
const GOLDEN_ANGLE: f32 = 2.399_963;

fn main() {
    App::new()
        .add_plugins(DefaultPlugins.set(WindowPlugin {
            primary_window: Some(Window {
                title: "Wave Spawner".to_string(),
                resolution: (800.0, 500.0).into(),
                ..default()
            }),
            ..default()
        }))
        .insert_resource(WaveState::default())
        .add_systems(Startup, setup)
        .add_systems(Update, (tick_wave, update_hud))
        .run();
}

/// Tracks the wave progression state.
#[derive(Resource)]
struct WaveState {
    wave: u32,
    phase: WavePhase,
    timer: f32,
}

/// Whether we're actively fighting or waiting between waves.
#[derive(PartialEq)]
enum WavePhase {
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
struct Enemy;

/// Marks the HUD text entities.
#[derive(Component)]
enum HudLabel {
    Wave,
    Count,
    Timer,
}

/// Spawns the camera and HUD.
fn setup(mut commands: Commands) {
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
        Text::new("Next wave in: 3.0 s"),
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
    mut state: ResMut<WaveState>,
    enemy_query: Query<Entity, With<Enemy>>,
) {
    let dt = time.delta_secs();
    state.timer += dt;

    match state.phase {
        WavePhase::Countdown => {
            if state.timer >= COUNTDOWN_DURATION {
                state.timer = 0.0;
                state.wave += 1;
                state.phase = WavePhase::Active;
                spawn_wave(&mut commands, state.wave);
            }
        }
        WavePhase::Active => {
            if state.timer >= WAVE_DURATION {
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
fn spawn_wave(commands: &mut Commands, wave: u32) {
    let count = enemy_count_for_wave(wave);
    for i in 0..count {
        let angle = i as f32 * GOLDEN_ANGLE;
        let pos = Vec3::new(
            SPAWN_RADIUS * angle.cos(),
            SPAWN_RADIUS * angle.sin(),
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
    enemy_query: Query<&Enemy>,
    mut hud_query: Query<(&mut Text, &HudLabel)>,
) {
    for (mut text, label) in &mut hud_query {
        match label {
            HudLabel::Wave => text.0 = format!("Wave: {}", state.wave),
            HudLabel::Count => text.0 = format!("Enemies: {}", enemy_query.iter().count()),
            HudLabel::Timer => {
                let remaining = match state.phase {
                    WavePhase::Countdown => COUNTDOWN_DURATION - state.timer,
                    WavePhase::Active => WAVE_DURATION - state.timer,
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
        assert!(SPAWN_RADIUS > 0.0);
    }

    #[test]
    fn wave_duration_and_countdown_are_positive() {
        assert!(WAVE_DURATION > 0.0);
        assert!(COUNTDOWN_DURATION > 0.0);
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
}
