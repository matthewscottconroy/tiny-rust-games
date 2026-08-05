//! Save / load — a reusable Bevy plugin persisting a resource to disk as JSON.
//!
//! This crate is a *building block*: drop [`SaveLoadPlugin`] into any Bevy app
//! with `app.add_plugins(SaveLoadPlugin)` and it manages a persistent
//! [`SaveData`] resource. Tune it through the [`SaveLoadConfig`] resource
//! without editing the plugin's internals.
//!
//! Key ideas:
//! - `serde` + `serde_json` serialize the [`SaveData`] resource to a JSON file.
//! - On startup we try to load an existing save; if none exists we use defaults.
//! - `S` saves, `L` loads explicitly, `SPACE` increments score, `R` resets.
//! - The save file path is relative to the working directory
//!   (the directory where `cargo run` is invoked).
//!
//! # Example
//! ```no_run
//! use bevy::prelude::*;
//! use save_load::SaveLoadPlugin;
//!
//! App::new()
//!     .add_plugins(DefaultPlugins)
//!     .add_plugins(SaveLoadPlugin)
//!     .run();
//! ```

use bevy::prelude::*;
use serde::{Deserialize, Serialize};
use std::fs;

/// Bundles every system and resource for the save/load feature.
///
/// Add it with `app.add_plugins(SaveLoadPlugin)`. It does **not** add
/// `DefaultPlugins`, so the host app stays in control of window and rendering
/// setup.
pub struct SaveLoadPlugin;

impl Plugin for SaveLoadPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<SaveLoadConfig>()
            .init_resource::<SaveData>()
            .add_systems(Startup, (load_on_startup, setup))
            .add_systems(Update, (handle_input, update_hud));
    }
}

// --- Configuration ---

/// Tunable parameters for the save/load feature. Override before adding the
/// plugin, e.g.
/// `app.insert_resource(SaveLoadConfig { score_increment: 25, ..default() })`.
#[derive(Resource, Clone)]
pub struct SaveLoadConfig {
    /// Points added per SPACE press.
    pub score_increment: u32,
    /// Path of the save file, relative to the working directory.
    pub save_path: String,
}

impl Default for SaveLoadConfig {
    fn default() -> Self {
        Self { score_increment: 10, save_path: "savegame.json".to_string() }
    }
}

// --- Saveable resource ---

/// All persistent game state written to and read from the save file.
#[derive(Resource, Serialize, Deserialize, Default, Clone)]
pub struct SaveData {
    /// Current score.
    pub score: u32,
    /// Current level.
    pub level: u32,
    /// Highest score reached so far.
    pub high_score: u32,
}

// --- Components ---

/// Marker for the score / level display text.
#[derive(Component)]
pub struct ScoreText;

/// Marker for the transient status line (e.g. "Saved to …").
#[derive(Component)]
pub struct StatusText;

// --- Startup ---

/// Attempts to read an existing save file and overwrite the default [`SaveData`].
///
/// Silently ignores missing or malformed files so the game always starts.
fn load_on_startup(config: Res<SaveLoadConfig>, mut save: ResMut<SaveData>) {
    if let Ok(json) = fs::read_to_string(&config.save_path) {
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
    config: Res<SaveLoadConfig>,
    mut save: ResMut<SaveData>,
    mut status_query: Query<&mut Text, With<StatusText>>,
) {
    let mut status: Option<String> = None;

    if input.just_pressed(KeyCode::Space) {
        save.score += config.score_increment;
        if save.score > save.high_score {
            save.high_score = save.score;
        }
    }

    if input.just_pressed(KeyCode::KeyS) {
        match serde_json::to_string_pretty(save.as_ref()) {
            Ok(json) => {
                match fs::write(&config.save_path, &json) {
                    Ok(_) => status = Some(format!("Saved to {}", config.save_path)),
                    Err(e) => status = Some(format!("Save failed: {}", e)),
                }
            }
            Err(e) => status = Some(format!("Serialize failed: {}", e)),
        }
    }

    if input.just_pressed(KeyCode::KeyL) {
        match fs::read_to_string(&config.save_path) {
            Ok(json) => match serde_json::from_str::<SaveData>(&json) {
                Ok(loaded) => {
                    *save = loaded;
                    status = Some(format!("Loaded from {}", config.save_path));
                }
                Err(e) => status = Some(format!("Deserialize failed: {}", e)),
            },
            Err(_) => status = Some(format!("No save file found at {}", config.save_path)),
        }
    }

    if input.just_pressed(KeyCode::KeyR) {
        *save = SaveData::default();
        let _ = fs::remove_file(&config.save_path);
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

    // --- SaveLoadConfig ---

    #[test]
    fn config_default_matches_documented_values() {
        let c = SaveLoadConfig::default();
        assert_eq!(c.score_increment, 10);
        assert_eq!(c.save_path, "savegame.json");
    }
}
