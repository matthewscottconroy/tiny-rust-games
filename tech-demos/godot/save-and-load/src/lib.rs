//! Save and Load demo — Godot 4.3 + gdext 0.5
//!
//! Demonstrates reading and writing `user://save.json` via Godot's
//! `FileAccess` API.  Serialization is done by hand (no serde) using simple
//! string formatting and splitting so the demo has no external crate deps.
//!
//! Teaches: hand-rolled JSON serialization written to `user://` through Godot's
//! `FileAccess`, with no external crate dependency.
//!
//! Concept: save-load

use godot::classes::{FileAccess, INode, Label, Node, file_access};
use godot::prelude::*;

// ---------------------------------------------------------------------------
// Extension entry-point
// ---------------------------------------------------------------------------

struct SaveAndLoadExtension;
#[gdextension]
unsafe impl ExtensionLibrary for SaveAndLoadExtension {}

// ---------------------------------------------------------------------------
// Pure-Rust data type
// ---------------------------------------------------------------------------

/// The values persisted to disk.
#[derive(Debug, Clone, PartialEq)]
pub struct SaveData {
    /// Level the player had reached.
    pub level: i32,
    /// Score at the time of saving.
    pub score: i32,
    /// Player's chosen name.
    pub player_name: String,
}

// ---------------------------------------------------------------------------
// Pure helper functions
// ---------------------------------------------------------------------------

/// Build a minimal JSON string from a `SaveData`.
pub fn serialize_save(data: &SaveData) -> String {
    format!(
        r#"{{"level":{},"score":{},"player_name":"{}"}}"#,
        data.level,
        data.score,
        data.player_name.replace('"', "\\\"")
    )
}

/// Parse a JSON string produced by `serialize_save`.
/// Returns `None` if the string is malformed.
pub fn deserialize_save(json: &str) -> Option<SaveData> {
    let level = extract_i32(json, "level")?;
    let score = extract_i32(json, "score")?;
    let player_name = extract_string(json, "player_name")?;
    Some(SaveData {
        level,
        score,
        player_name,
    })
}

/// Extract an integer value for a given JSON key.
pub fn extract_i32(json: &str, key: &str) -> Option<i32> {
    let needle = format!("\"{}\":", key);
    let start = json.find(&needle)? + needle.len();
    let rest = json[start..].trim_start();
    let end = rest
        .find(|c: char| !c.is_ascii_digit() && c != '-')
        .unwrap_or(rest.len());
    rest[..end].parse().ok()
}

/// Extract a quoted string value for a given JSON key.
pub fn extract_string(json: &str, key: &str) -> Option<String> {
    let needle = format!("\"{}\":\"", key);
    let start = json.find(&needle)? + needle.len();
    let rest = &json[start..];
    let end = rest.find('"')?;
    Some(rest[..end].replace("\\\"", "\""))
}

/// Return a default (new-game) save.
pub fn default_save() -> SaveData {
    SaveData {
        level: 1,
        score: 0,
        player_name: "Player".to_string(),
    }
}

/// Format the level for display.
pub fn level_display(level: i32) -> String {
    format!("Level {}", level)
}

// ---------------------------------------------------------------------------
// GodotClass
// ---------------------------------------------------------------------------

/// Godot node that serialises `SaveData` to `user://`.
#[derive(GodotClass)]
#[class(base=Node)]
pub struct SaveManager {
    data: SaveData,
    base: Base<Node>,
}

#[godot_api]
impl INode for SaveManager {
    fn init(base: Base<Node>) -> Self {
        Self {
            data: default_save(),
            base,
        }
    }

    fn ready(&mut self) {
        self.refresh_label();
    }
}

#[godot_api]
impl SaveManager {
    /// Writes the current state to the save file.
    #[func]
    pub fn save_game(&mut self) {
        let json = serialize_save(&self.data);
        if let Some(mut file) = FileAccess::open("user://save.json", file_access::ModeFlags::WRITE)
        {
            file.store_string(json.as_str());
        }
    }

    /// Reads the save file, returning `false` if none exists.
    #[func]
    pub fn load_game(&mut self) -> bool {
        if let Some(file) = FileAccess::open("user://save.json", file_access::ModeFlags::READ) {
            let json = file.get_as_text().to_string();
            if let Some(parsed) = deserialize_save(&json) {
                self.data = parsed;
                self.refresh_label();
                return true;
            }
        }
        false
    }

    /// Sets the name that will be written on the next save.
    #[func]
    pub fn set_player_name(&mut self, name: GString) {
        self.data.player_name = name.to_string();
    }

    /// Sets the level that will be written on the next save.
    #[func]
    pub fn set_level(&mut self, level: i32) {
        self.data.level = level;
    }

    /// Sets the score that will be written on the next save.
    #[func]
    pub fn set_score(&mut self, score: i32) {
        self.data.score = score;
    }

    fn refresh_label(&mut self) {
        let text = format!(
            "{}\nScore: {}\nPlayer: {}",
            level_display(self.data.level),
            self.data.score,
            self.data.player_name
        );
        if let Some(node) = self.base().get_node_or_null("Label")
            && let Ok(mut label) = node.try_cast::<Label>()
        {
            label.set_text(text.as_str());
        }
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn serialize_basic() {
        let d = SaveData {
            level: 3,
            score: 150,
            player_name: "Alice".to_string(),
        };
        let json = serialize_save(&d);
        assert!(json.contains("\"level\":3"));
        assert!(json.contains("\"score\":150"));
        assert!(json.contains("\"player_name\":\"Alice\""));
    }

    #[test]
    fn deserialize_basic() {
        let json = r#"{"level":3,"score":150,"player_name":"Alice"}"#;
        let d = deserialize_save(json).expect("should parse");
        assert_eq!(d.level, 3);
        assert_eq!(d.score, 150);
        assert_eq!(d.player_name, "Alice");
    }

    #[test]
    fn roundtrip() {
        let original = SaveData {
            level: 5,
            score: 999,
            player_name: "Bob".to_string(),
        };
        let json = serialize_save(&original);
        let parsed = deserialize_save(&json).expect("roundtrip should work");
        assert_eq!(parsed, original);
    }

    #[test]
    fn default_save_values() {
        let d = default_save();
        assert_eq!(d.level, 1);
        assert_eq!(d.score, 0);
        assert_eq!(d.player_name, "Player");
    }

    #[test]
    fn level_display_format() {
        assert_eq!(level_display(1), "Level 1");
        assert_eq!(level_display(99), "Level 99");
    }

    #[test]
    fn deserialize_invalid_returns_none() {
        assert!(deserialize_save("not json").is_none());
    }

    #[test]
    fn serialize_escapes_quotes_in_name() {
        let d = SaveData {
            level: 1,
            score: 0,
            player_name: r#"O"Brien"#.to_string(),
        };
        let json = serialize_save(&d);
        // Quotes inside name must be escaped
        assert!(json.contains(r#"O\"Brien"#));
    }
}
