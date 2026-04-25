//! Save / load demo.
//!
//! Key ideas:
//! - `serde` + `serde_json` serialize a [`Resource`] to a JSON file on disk.
//! - On startup we try to load an existing save; if none exists we use defaults.
//! - `S` saves, `L` loads explicitly, `SPACE` increments score, `R` resets.
//! - The save file path is relative to the working directory
//!   (the directory where `cargo run` is invoked).

use bevy::prelude::*;
use serde::{Deserialize, Serialize};
use std::fs;

const SAVE_PATH: &str = "savegame.json";

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .init_resource::<SaveData>()
        .add_systems(Startup, (load_on_startup, setup))
        .add_systems(Update, (handle_input, update_hud))
        .run();
}

// --- Saveable resource ---

/// All persistent game state written to and read from the save file.
#[derive(Resource, Serialize, Deserialize, Default, Clone)]
struct SaveData {
    score: u32,
    level: u32,
    high_score: u32,
}

// --- Components ---

/// Marker for the score / level display text.
#[derive(Component)]
struct ScoreText;

/// Marker for the transient status line (e.g. "Saved to …").
#[derive(Component)]
struct StatusText;

// --- Startup ---

/// Attempts to read an existing save file and overwrite the default [`SaveData`].
///
/// Silently ignores missing or malformed files so the game always starts.
fn load_on_startup(mut save: ResMut<SaveData>) {
    if let Ok(json) = fs::read_to_string(SAVE_PATH) {
        if let Ok(loaded) = serde_json::from_str::<SaveData>(&json) {
            *save = loaded;
        }
    }
}

/// Spawns the camera, decorative sprite, and HUD labels.
fn setup(mut commands: Commands, save: Res<SaveData>) {
    commands.spawn(Camera2d);

    commands.spawn((
        Sprite {
            color: Color::srgb(0.3, 0.55, 0.85),
            custom_size: Some(Vec2::splat(50.0)),
            ..default()
        },
        Transform::default(),
    ));

    commands.spawn((
        Text::new(score_text(&save)),
        TextFont { font_size: 22.0, ..default() },
        TextColor(Color::WHITE),
        Node {
            position_type: PositionType::Absolute,
            top: Val::Px(12.0),
            left: Val::Px(12.0),
            ..default()
        },
        ScoreText,
    ));

    commands.spawn((
        Text::new("Ready"),
        TextFont { font_size: 15.0, ..default() },
        TextColor(Color::srgb(0.6, 0.85, 0.6)),
        Node {
            position_type: PositionType::Absolute,
            top: Val::Px(50.0),
            left: Val::Px(12.0),
            ..default()
        },
        StatusText,
    ));

    commands.spawn((
        Text::new("SPACE = +score   S = save   L = load   R = reset"),
        TextFont { font_size: 14.0, ..default() },
        TextColor(Color::srgb(0.55, 0.55, 0.55)),
        Node {
            position_type: PositionType::Absolute,
            bottom: Val::Px(10.0),
            left: Val::Px(10.0),
            ..default()
        },
    ));
}

// --- Systems ---

/// Handles all keyboard input: score increment, save, load, and reset.
fn handle_input(
    input: Res<ButtonInput<KeyCode>>,
    mut save: ResMut<SaveData>,
    mut status_query: Query<&mut Text, With<StatusText>>,
) {
    let mut status: Option<String> = None;

    if input.just_pressed(KeyCode::Space) {
        save.score += 10;
        if save.score > save.high_score {
            save.high_score = save.score;
        }
    }

    if input.just_pressed(KeyCode::KeyS) {
        match serde_json::to_string_pretty(save.as_ref()) {
            Ok(json) => {
                match fs::write(SAVE_PATH, &json) {
                    Ok(_) => status = Some(format!("Saved to {}", SAVE_PATH)),
                    Err(e) => status = Some(format!("Save failed: {}", e)),
                }
            }
            Err(e) => status = Some(format!("Serialize failed: {}", e)),
        }
    }

    if input.just_pressed(KeyCode::KeyL) {
        match fs::read_to_string(SAVE_PATH) {
            Ok(json) => match serde_json::from_str::<SaveData>(&json) {
                Ok(loaded) => {
                    *save = loaded;
                    status = Some(format!("Loaded from {}", SAVE_PATH));
                }
                Err(e) => status = Some(format!("Deserialize failed: {}", e)),
            },
            Err(_) => status = Some(format!("No save file found at {}", SAVE_PATH)),
        }
    }

    if input.just_pressed(KeyCode::KeyR) {
        *save = SaveData::default();
        let _ = fs::remove_file(SAVE_PATH);
        status = Some("Reset — save file deleted".to_string());
    }

    if let Some(msg) = status {
        for mut text in &mut status_query {
            *text = Text::new(msg.clone());
        }
    }
}

/// Rewrites the score display whenever [`SaveData`] changes.
fn update_hud(save: Res<SaveData>, mut query: Query<&mut Text, With<ScoreText>>) {
    if !save.is_changed() { return; }
    for mut text in &mut query {
        *text = Text::new(score_text(&save));
    }
}

/// Formats the score / high-score / level string shown in the HUD.
pub fn score_text(save: &SaveData) -> String {
    format!(
        "Score: {}   High: {}   Level: {}",
        save.score, save.high_score, save.level
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    // --- score_text ---

    #[test]
    fn score_text_contains_all_fields() {
        let save = SaveData { score: 42, high_score: 100, level: 3 };
        let text = score_text(&save);
        assert!(text.contains("42"),  "score missing from text");
        assert!(text.contains("100"), "high score missing from text");
        assert!(text.contains("3"),   "level missing from text");
    }

    #[test]
    fn score_text_zero_fields() {
        let text = score_text(&SaveData::default());
        assert!(text.contains('0'));
    }

    // --- SaveData ---

    #[test]
    fn save_data_default_zeroes_all_fields() {
        let s = SaveData::default();
        assert_eq!(s.score,      0);
        assert_eq!(s.high_score, 0);
        assert_eq!(s.level,      0);
    }

    #[test]
    fn save_data_json_roundtrip() {
        let original = SaveData { score: 99, high_score: 200, level: 7 };
        let json = serde_json::to_string(&original).expect("serialize failed");
        let loaded: SaveData = serde_json::from_str(&json).expect("deserialize failed");
        assert_eq!(loaded.score,      original.score);
        assert_eq!(loaded.high_score, original.high_score);
        assert_eq!(loaded.level,      original.level);
    }

    #[test]
    fn save_data_clone_is_independent() {
        let mut a = SaveData { score: 10, high_score: 20, level: 1 };
        let b = a.clone();
        a.score = 999;
        assert_eq!(b.score, 10, "clone should not share state");
    }
}
