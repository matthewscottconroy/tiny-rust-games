//! Combo system — a reusable Bevy plugin for input-buffer sequence matching.
//!
//! This crate is a *building block*: drop [`ComboSystemPlugin`] into any Bevy
//! app with `app.add_plugins(ComboSystemPlugin)` and it buffers recent key
//! presses, matches them against a set of combos, and flashes a banner + score
//! on a hit.
//!
//! Key ideas:
//! - A fixed-length input buffer stores the most recent key presses as a
//!   `VecDeque<KeyCode>`. Old entries fall off the back when the buffer reaches
//!   its capacity.
//! - [`matches_sequence`] checks whether the *tail* of the buffer equals a
//!   given pattern, so a combo fires at the moment its last input is pressed.
//! - Recognised combos flash a coloured banner and add to the score; the buffer
//!   is cleared after a match to prevent overlap.
//! - Controls are arrow keys (directional) and WASD (alternative bindings).
//! - Tune the buffer capacity and flash duration via [`ComboSystemConfig`].
//!
//! **Controls:** Arrow keys / WASD — enter inputs and trigger combos.
//!
//! # Example
//! ```no_run
//! use bevy::prelude::*;
//! use combo_system::ComboSystemPlugin;
//!
//! App::new()
//!     .add_plugins(DefaultPlugins)
//!     .add_plugins(ComboSystemPlugin)
//!     .run();
//! ```

use bevy::prelude::*;
use std::collections::VecDeque;

/// Bundles every system and resource for the combo-system feature.
///
/// Add it with `app.add_plugins(ComboSystemPlugin)`. It does **not** add
/// `DefaultPlugins`, so the host app stays in control of window and rendering
/// setup.
pub struct ComboSystemPlugin;

impl Plugin for ComboSystemPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<ComboSystemConfig>()
            .init_resource::<InputBuffer>()
            .init_resource::<Score>()
            .init_resource::<ComboFlash>()
            .add_systems(Startup, setup)
            .add_systems(Update, (collect_input, check_combos, tick_flash, update_hud).chain());
    }
}

// --- Configuration ---

/// Tunable parameters for the combo-system feature. Override before adding the
/// plugin, e.g. `app.insert_resource(ComboSystemConfig { buffer_cap: 12, ..default() })`.
#[derive(Resource, Clone, Copy, Debug)]
pub struct ComboSystemConfig {
    /// Maximum number of recent inputs stored in the buffer.
    pub buffer_cap: usize,
    /// Seconds the combo flash banner stays visible.
    pub flash_duration: f32,
}

impl Default for ComboSystemConfig {
    fn default() -> Self {
        Self { buffer_cap: 8, flash_duration: 1.2 }
    }
}

// ─── Pure helpers ────────────────────────────────────────────────────────────

/// Returns `true` when the *tail* of `buffer` exactly equals `pattern`.
///
/// An empty `pattern` always matches.  If `buffer` is shorter than `pattern`,
/// returns `false`.
pub fn matches_sequence<T: PartialEq>(buffer: &[T], pattern: &[T]) -> bool {
    if pattern.is_empty() {
        return true;
    }
    if buffer.len() < pattern.len() {
        return false;
    }
    let offset = buffer.len() - pattern.len();
    &buffer[offset..] == pattern
}

/// Trims `v` to at most `max` elements by removing from the front.
pub fn trim_to_max<T>(v: &mut VecDeque<T>, max: usize) {
    while v.len() > max {
        v.pop_front();
    }
}

// ─── Combo definitions ───────────────────────────────────────────────────────

struct ComboEntry {
    name: &'static str,
    color: Color,
    pattern: &'static [KeyCode],
}

fn combos() -> Vec<ComboEntry> {
    vec![
        ComboEntry {
            name: "DOUBLE UP!",
            color: Color::srgb(0.3, 1.0, 0.4),
            pattern: &[KeyCode::ArrowUp, KeyCode::ArrowUp],
        },
        ComboEntry {
            name: "SPIN!",
            color: Color::srgb(0.4, 0.6, 1.0),
            pattern: &[
                KeyCode::ArrowUp,
                KeyCode::ArrowRight,
                KeyCode::ArrowDown,
                KeyCode::ArrowLeft,
            ],
        },
        ComboEntry {
            name: "ZIGZAG!",
            color: Color::srgb(1.0, 0.8, 0.2),
            pattern: &[
                KeyCode::ArrowLeft,
                KeyCode::ArrowRight,
                KeyCode::ArrowLeft,
            ],
        },
        ComboEntry {
            name: "DIVE!",
            color: Color::srgb(1.0, 0.3, 0.5),
            pattern: &[KeyCode::ArrowDown, KeyCode::ArrowDown],
        },
    ]
}

// ─── Resources & components ──────────────────────────────────────────────────

#[derive(Resource, Default)]
struct InputBuffer(VecDeque<KeyCode>);

#[derive(Resource, Default)]
struct Score(u32);

/// Active combo flash: name, color, countdown timer.
#[derive(Resource)]
struct ComboFlash {
    name: String,
    color: Color,
    timer: f32,
}

impl Default for ComboFlash {
    fn default() -> Self {
        Self { name: String::new(), color: Color::WHITE, timer: 0.0 }
    }
}

#[derive(Component)]
enum HudLabel {
    Buffer,
    Flash,
    Score,
}

// ─── Systems ─────────────────────────────────────────────────────────────────

fn setup(mut commands: Commands) {
    commands.spawn(Camera2d);

    let style = TextFont { font_size: 22.0, ..default() };

    commands.spawn((
        Text::new("Buffer: "),
        style.clone(),
        TextColor(Color::srgb(0.8, 0.8, 0.8)),
        Node {
            position_type: PositionType::Absolute,
            top: Val::Px(20.0),
            left: Val::Px(20.0),
            ..default()
        },
        HudLabel::Buffer,
    ));
    commands.spawn((
        Text::new(""),
        TextFont { font_size: 38.0, ..default() },
        TextColor(Color::WHITE),
        Node {
            position_type: PositionType::Absolute,
            top: Val::Px(100.0),
            left: Val::Px(20.0),
            ..default()
        },
        HudLabel::Flash,
    ));
    commands.spawn((
        Text::new("Score: 0"),
        style.clone(),
        TextColor(Color::WHITE),
        Node {
            position_type: PositionType::Absolute,
            bottom: Val::Px(20.0),
            left: Val::Px(20.0),
            ..default()
        },
        HudLabel::Score,
    ));

    // Combo guide
    commands.spawn((
        Text::new(
            "Combos:\n↑↑  Double Up\n↑→↓←  Spin\n←→←  Zigzag\n↓↓  Dive",
        ),
        TextFont { font_size: 18.0, ..default() },
        TextColor(Color::srgb(0.6, 0.6, 0.6)),
        Node {
            position_type: PositionType::Absolute,
            top: Val::Px(20.0),
            right: Val::Px(20.0),
            ..default()
        },
    ));
}

/// Captures arrow-key presses into the buffer; WASD maps to arrows.
fn collect_input(
    input: Res<ButtonInput<KeyCode>>,
    config: Res<ComboSystemConfig>,
    mut buf: ResMut<InputBuffer>,
) {
    let mappings = [
        (KeyCode::ArrowUp, KeyCode::ArrowUp),
        (KeyCode::ArrowDown, KeyCode::ArrowDown),
        (KeyCode::ArrowLeft, KeyCode::ArrowLeft),
        (KeyCode::ArrowRight, KeyCode::ArrowRight),
        (KeyCode::KeyW, KeyCode::ArrowUp),
        (KeyCode::KeyS, KeyCode::ArrowDown),
        (KeyCode::KeyA, KeyCode::ArrowLeft),
        (KeyCode::KeyD, KeyCode::ArrowRight),
    ];
    for (key, mapped) in mappings {
        if input.just_pressed(key) {
            buf.0.push_back(mapped);
            trim_to_max(&mut buf.0, config.buffer_cap);
            break; // one input per frame
        }
    }
}

/// Checks the buffer tail against every combo pattern.
fn check_combos(
    config: Res<ComboSystemConfig>,
    mut buf: ResMut<InputBuffer>,
    mut flash: ResMut<ComboFlash>,
    mut score: ResMut<Score>,
) {
    let buf_slice: Vec<KeyCode> = buf.0.iter().copied().collect();
    for combo in combos() {
        if matches_sequence(&buf_slice, combo.pattern) {
            flash.name = combo.name.to_string();
            flash.color = combo.color;
            flash.timer = config.flash_duration;
            score.0 += 1;
            buf.0.clear();
            return;
        }
    }
}

fn tick_flash(time: Res<Time>, mut flash: ResMut<ComboFlash>) {
    if flash.timer > 0.0 {
        flash.timer = (flash.timer - time.delta_secs()).max(0.0);
    }
}

fn update_hud(
    config: Res<ComboSystemConfig>,
    buf: Res<InputBuffer>,
    flash: Res<ComboFlash>,
    score: Res<Score>,
    mut query: Query<(&mut Text, &mut TextColor, &HudLabel)>,
) {
    let key_name = |k: KeyCode| match k {
        KeyCode::ArrowUp => "↑",
        KeyCode::ArrowDown => "↓",
        KeyCode::ArrowLeft => "←",
        KeyCode::ArrowRight => "→",
        _ => "?",
    };
    let buf_str: String = buf.0.iter().map(|k| key_name(*k)).collect::<Vec<_>>().join(" ");

    for (mut text, mut color, label) in &mut query {
        match label {
            HudLabel::Buffer => text.0 = format!("Buffer: {}", buf_str),
            HudLabel::Score => text.0 = format!("Score: {}", score.0),
            HudLabel::Flash => {
                if flash.timer > 0.0 {
                    text.0 = flash.name.clone();
                    let alpha = (flash.timer / config.flash_duration).min(1.0);
                    let linear = flash.color.to_linear();
                    color.0 = Color::srgba(linear.red, linear.green, linear.blue, alpha);
                } else {
                    text.0 = String::new();
                }
            }
        }
    }
}

// ─── Tests ───────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn exact_match_returns_true() {
        let buf = vec![1u8, 2, 3];
        assert!(matches_sequence(&buf, &[1, 2, 3]));
    }

    #[test]
    fn tail_match_returns_true() {
        let buf = vec![9u8, 9, 1, 2, 3];
        assert!(matches_sequence(&buf, &[1, 2, 3]));
    }

    #[test]
    fn wrong_order_returns_false() {
        let buf = vec![1u8, 3, 2];
        assert!(!matches_sequence(&buf, &[1, 2, 3]));
    }

    #[test]
    fn buffer_shorter_than_pattern_returns_false() {
        let buf = vec![2u8, 3];
        assert!(!matches_sequence(&buf, &[1, 2, 3]));
    }

    #[test]
    fn empty_pattern_always_matches() {
        let buf = vec![1u8, 2, 3];
        assert!(matches_sequence(&buf, &[]));
        assert!(matches_sequence::<u8>(&[], &[]));
    }

    #[test]
    fn trim_to_max_removes_from_front() {
        let mut v: VecDeque<u8> = VecDeque::from([1, 2, 3, 4, 5]);
        trim_to_max(&mut v, 3);
        assert_eq!(v, VecDeque::from([3, 4, 5]));
    }

    #[test]
    fn config_default_matches_documented_values() {
        let c = ComboSystemConfig::default();
        assert_eq!(c.buffer_cap, 8);
        assert!((c.flash_duration - 1.2).abs() < 1e-6);
    }
}
