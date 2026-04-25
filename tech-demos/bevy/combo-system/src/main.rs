//! Combo system demo — input-buffer sequence matching.
//!
//! Key ideas:
//! - A fixed-length `InputBuffer` resource stores the most recent key presses
//!   as a `VecDeque<KeyCode>`.  Old entries fall off the back when the buffer
//!   reaches its capacity.
//! - `matches_sequence` checks whether the *tail* of the buffer equals a
//!   given pattern, so a combo fires at the moment its last input is pressed.
//! - Recognised combos flash a coloured banner and add to the score; the
//!   buffer is cleared after a match to prevent overlap.
//! - Controls are arrow keys (directional) and WASD (alternative bindings).
//!
//! **Controls:** Arrow keys / WASD — enter inputs and trigger combos.

use bevy::prelude::*;
use bevy::window::WindowResolution;
use std::collections::VecDeque;

/// Maximum number of recent inputs stored in the buffer.
const BUFFER_CAP: usize = 8;
/// Seconds the combo flash banner stays visible.
const FLASH_DURATION: f32 = 1.2;

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

// ─── Bevy app ────────────────────────────────────────────────────────────────

fn main() {
    App::new()
        .add_plugins(DefaultPlugins.set(WindowPlugin {
            primary_window: Some(Window {
                title: "Combo System — Arrow keys to enter inputs".to_string(),
                resolution: (700u32, 400u32).into(),
                ..default()
            }),
            ..default()
        }))
        .init_resource::<InputBuffer>()
        .init_resource::<Score>()
        .init_resource::<ComboFlash>()
        .add_systems(Startup, setup)
        .add_systems(Update, (collect_input, check_combos, tick_flash, update_hud).chain())
        .run();
}

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
fn collect_input(input: Res<ButtonInput<KeyCode>>, mut buf: ResMut<InputBuffer>) {
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
            trim_to_max(&mut buf.0, BUFFER_CAP);
            break; // one input per frame
        }
    }
}

/// Checks the buffer tail against every combo pattern.
fn check_combos(
    mut buf: ResMut<InputBuffer>,
    mut flash: ResMut<ComboFlash>,
    mut score: ResMut<Score>,
) {
    let buf_slice: Vec<KeyCode> = buf.0.iter().copied().collect();
    for combo in combos() {
        if matches_sequence(&buf_slice, combo.pattern) {
            flash.name = combo.name.to_string();
            flash.color = combo.color;
            flash.timer = FLASH_DURATION;
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
                    let alpha = (flash.timer / FLASH_DURATION).min(1.0);
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
}
