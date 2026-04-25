//! Dialog system demo — conversation tree with a typing effect.
//!
//! Key ideas:
//! - A dialog tree is a `Vec<DialogNode>`, where each node holds speaker
//!   name, full text, and a list of `(label, next_node_id)` choices.
//! - `advance_dialog` is a pure function: given the current node and a choice
//!   index it returns the next node ID, or `None` when the index is out of
//!   range or the choice list is empty (conversation ended).
//! - `visible_text` slices a `&str` safely by char count so non-ASCII text
//!   is never split mid-codepoint.
//! - A `TypingTimer` resource drives `char_index` forward over time; once all
//!   characters are shown, the choices are revealed.
//! - Press **SPACE** to skip typing / advance on nodes with no choices.
//!   Press **1**, **2**, or **3** to pick a choice.
//!
//! **Controls:** SPACE — advance / skip typing   1 / 2 / 3 — choose.

use bevy::prelude::*;
use bevy::window::WindowResolution;

// ─── Dialog tree ─────────────────────────────────────────────────────────────

/// A single node in the conversation tree.
#[derive(Clone)]
pub struct DialogNode {
    /// Who is speaking.
    pub speaker: &'static str,
    /// Full text of the node.
    pub text: &'static str,
    /// List of player choices: (button label, next node index).
    /// Empty = end of conversation.
    pub choices: &'static [(&'static str, usize)],
}

fn build_tree() -> Vec<DialogNode> {
    vec![
        // 0
        DialogNode {
            speaker: "Guild Master",
            text: "Greetings, traveler. Word of your deeds has reached us. Will you join our guild?",
            choices: &[("Yes, I accept.", 1), ("Not today.", 2)],
        },
        // 1
        DialogNode {
            speaker: "Guild Master",
            text: "Excellent! Your first task: recover the lost relic from the northern ruins. Speak to Mira at the city gates.",
            choices: &[("Understood.", 3)],
        },
        // 2
        DialogNode {
            speaker: "Guild Master",
            text: "A shame. The offer stands should you change your mind. Our door is always open.",
            choices: &[("Farewell.", 3)],
        },
        // 3
        DialogNode {
            speaker: "Guild Master",
            text: "Safe travels. May fortune guide your blade.",
            choices: &[],
        },
    ]
}

// ─── Pure helpers ────────────────────────────────────────────────────────────

/// Returns the node ID reached by making `choice` from `current`, or `None`
/// when the choice is out of range or there are no choices (end of dialog).
pub fn advance_dialog(nodes: &[DialogNode], current: usize, choice: usize) -> Option<usize> {
    let node = nodes.get(current)?;
    let (_, next) = node.choices.get(choice)?;
    Some(*next)
}

/// Returns a `&str` slice containing the first `char_count` characters of
/// `text`, never splitting a multi-byte codepoint.
pub fn visible_text(text: &str, char_count: usize) -> &str {
    let end = text
        .char_indices()
        .nth(char_count)
        .map_or(text.len(), |(i, _)| i);
    &text[..end]
}

// ─── Resources & components ──────────────────────────────────────────────────

/// Tracks conversation state.
#[derive(Resource)]
struct DialogState {
    node_id: usize,
    char_index: usize,
    typing_timer: f32,
    finished: bool,
}

impl Default for DialogState {
    fn default() -> Self {
        Self { node_id: 0, char_index: 0, typing_timer: 0.0, finished: false }
    }
}

/// Seconds per character reveal.
const CHARS_PER_SEC: f32 = 40.0;

#[derive(Component)]
enum DialogUi {
    Speaker,
    Body,
    Choice(usize),
    Hint,
}

// ─── Bevy app ────────────────────────────────────────────────────────────────

fn main() {
    App::new()
        .add_plugins(DefaultPlugins.set(WindowPlugin {
            primary_window: Some(Window {
                title: "Dialog System — SPACE / 1 2 3 to advance".to_string(),
                resolution: (720u32, 480u32).into(),
                ..default()
            }),
            ..default()
        }))
        .init_resource::<DialogState>()
        .add_systems(Startup, setup)
        .add_systems(Update, (tick_typing, handle_input, update_ui).chain())
        .run();
}

fn setup(mut commands: Commands) {
    commands.spawn(Camera2d);

    // Dialog box background
    commands.spawn((
        Sprite { color: Color::srgb(0.08, 0.08, 0.12), custom_size: Some(Vec2::new(680.0, 240.0)), ..default() },
        Transform::from_translation(Vec3::new(0.0, -90.0, 0.0)),
    ));
    commands.spawn((
        Sprite { color: Color::srgb(0.2, 0.2, 0.3), custom_size: Some(Vec2::new(684.0, 244.0)), ..default() },
        Transform::from_translation(Vec3::new(0.0, -90.0, -0.1)),
    ));

    // Scene image placeholder
    commands.spawn((
        Sprite { color: Color::srgb(0.1, 0.12, 0.18), custom_size: Some(Vec2::new(720.0, 200.0)), ..default() },
        Transform::from_translation(Vec3::new(0.0, 150.0, 0.0)),
    ));
    commands.spawn((
        Text::new("[ Scene ]"),
        TextFont { font_size: 20.0, ..default() },
        TextColor(Color::srgb(0.3, 0.3, 0.4)),
        Node {
            position_type: PositionType::Absolute,
            top: Val::Px(100.0),
            left: Val::Px(0.0),
            right: Val::Px(0.0),
            ..default()
        },
    ));

    let base = TextFont { font_size: 18.0, ..default() };

    // Speaker name
    commands.spawn((
        Text::new("Speaker"),
        TextFont { font_size: 20.0, ..default() },
        TextColor(Color::srgb(0.8, 0.75, 0.4)),
        Node {
            position_type: PositionType::Absolute,
            bottom: Val::Px(196.0),
            left: Val::Px(28.0),
            ..default()
        },
        DialogUi::Speaker,
    ));

    // Dialog body text
    commands.spawn((
        Text::new(""),
        base.clone(),
        TextColor(Color::srgb(0.92, 0.92, 0.92)),
        Node {
            position_type: PositionType::Absolute,
            bottom: Val::Px(130.0),
            left: Val::Px(28.0),
            right: Val::Px(28.0),
            ..default()
        },
        DialogUi::Body,
    ));

    // Choice buttons (up to 3)
    for i in 0..3usize {
        commands.spawn((
            Text::new(""),
            base.clone(),
            TextColor(Color::srgb(0.5, 0.85, 0.5)),
            Node {
                position_type: PositionType::Absolute,
                bottom: Val::Px(60.0 - i as f32 * 26.0),
                left: Val::Px(28.0),
                ..default()
            },
            DialogUi::Choice(i),
        ));
    }

    // Hint
    commands.spawn((
        Text::new("SPACE — advance   1/2/3 — choose"),
        TextFont { font_size: 14.0, ..default() },
        TextColor(Color::srgb(0.45, 0.45, 0.55)),
        Node {
            position_type: PositionType::Absolute,
            bottom: Val::Px(8.0),
            right: Val::Px(16.0),
            ..default()
        },
        DialogUi::Hint,
    ));
}

fn tick_typing(time: Res<Time>, mut state: ResMut<DialogState>) {
    let tree = build_tree();
    let Some(node) = tree.get(state.node_id) else { return };
    let total_chars = node.text.chars().count();
    if state.char_index >= total_chars {
        state.finished = true;
        return;
    }
    state.typing_timer += time.delta_secs();
    let new_chars = (state.typing_timer * CHARS_PER_SEC) as usize;
    state.char_index = (state.char_index + new_chars).min(total_chars);
    state.typing_timer -= new_chars as f32 / CHARS_PER_SEC;
}

fn handle_input(input: Res<ButtonInput<KeyCode>>, mut state: ResMut<DialogState>) {
    let tree = build_tree();
    let Some(node) = tree.get(state.node_id) else { return };

    // SPACE skips typing or advances when there is exactly one choice or no choices.
    if input.just_pressed(KeyCode::Space) {
        if !state.finished {
            state.char_index = node.text.chars().count();
            state.finished = true;
        } else if node.choices.is_empty() {
            // End of dialog — wrap back to start.
            *state = DialogState::default();
        } else if node.choices.len() == 1 {
            if let Some(next) = advance_dialog(&tree, state.node_id, 0) {
                *state = DialogState { node_id: next, ..default() };
            }
        }
        return;
    }

    if !state.finished {
        return;
    }

    for (key, idx) in [
        (KeyCode::Digit1, 0usize),
        (KeyCode::Digit2, 1),
        (KeyCode::Digit3, 2),
    ] {
        if input.just_pressed(key) {
            if let Some(next) = advance_dialog(&tree, state.node_id, idx) {
                *state = DialogState { node_id: next, ..default() };
            }
            return;
        }
    }
}

fn update_ui(
    state: Res<DialogState>,
    mut query: Query<(&mut Text, &mut TextColor, &DialogUi)>,
) {
    let tree = build_tree();
    let Some(node) = tree.get(state.node_id) else { return };
    let shown = visible_text(node.text, state.char_index);

    for (mut text, mut color, label) in &mut query {
        match label {
            DialogUi::Speaker => text.0 = node.speaker.to_string(),
            DialogUi::Body => text.0 = shown.to_string(),
            DialogUi::Choice(i) => {
                if state.finished {
                    if let Some((label_str, _)) = node.choices.get(*i) {
                        text.0 = format!("[{}] {}", i + 1, label_str);
                    } else {
                        text.0 = String::new();
                    }
                } else {
                    text.0 = String::new();
                }
            }
            DialogUi::Hint => {
                if node.choices.is_empty() && state.finished {
                    color.0 = Color::srgb(0.8, 0.75, 0.3);
                    text.0 = "SPACE — restart".to_string();
                } else {
                    color.0 = Color::srgb(0.45, 0.45, 0.55);
                    text.0 = "SPACE — skip / advance   1/2/3 — choose".to_string();
                }
            }
        }
    }
}

// ─── Tests ───────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    fn tree() -> Vec<DialogNode> {
        build_tree()
    }

    #[test]
    fn advance_valid_choice_returns_next() {
        assert_eq!(advance_dialog(&tree(), 0, 0), Some(1));
        assert_eq!(advance_dialog(&tree(), 0, 1), Some(2));
    }

    #[test]
    fn advance_out_of_range_choice_returns_none() {
        assert_eq!(advance_dialog(&tree(), 0, 5), None);
    }

    #[test]
    fn advance_end_node_has_no_choices() {
        assert_eq!(advance_dialog(&tree(), 3, 0), None);
    }

    #[test]
    fn advance_invalid_node_returns_none() {
        assert_eq!(advance_dialog(&tree(), 999, 0), None);
    }

    #[test]
    fn visible_text_partial() {
        assert_eq!(visible_text("hello world", 5), "hello");
    }

    #[test]
    fn visible_text_full() {
        let s = "abc";
        assert_eq!(visible_text(s, 10), "abc");
    }

    #[test]
    fn visible_text_zero_returns_empty() {
        assert_eq!(visible_text("hello", 0), "");
    }
}
