//! Menu With Options GDExtension demo — a settings menu with live config from Rust.
//!
//! Demonstrates:
//!
//! - Subclassing `Control` and populating an `OptionButton` in `ready()`.
//! - Connecting a `Button`'s `pressed` signal to a Rust `#[func]` handler.
//! - Reading `OptionButton`, `CheckBox`, and `HSlider` values in `on_apply`.
//! - Storing the combined settings in a plain Rust struct (`MenuConfig`).
//! - Exposing `get_difficulty()` and `get_volume()` as Godot callables.
//! - Pure helpers (`difficulty_name`, `volume_to_db`, `validate_volume`)
//!   fully covered by unit tests.

use godot::classes::{Button, CheckBox, Control, HSlider, IControl, OptionButton};
use godot::prelude::*;

// ─── Extension entry point ───────────────────────────────────────────────────

struct MenuWithOptionsExt;

#[gdextension]
unsafe impl ExtensionLibrary for MenuWithOptionsExt {}

// ─── Data types ───────────────────────────────────────────────────────────────

/// Snapshot of the user's chosen settings when the Apply button is pressed.
#[derive(Debug, Clone, Default)]
pub struct MenuConfig {
    /// Difficulty tier: 0 = Easy, 1 = Normal, 2 = Hard.
    pub difficulty: u8,
    /// Whether the game should run fullscreen.
    pub fullscreen: bool,
    /// Volume level in the range 0..100.
    pub volume: f32,
}

// ─── Pure functions ───────────────────────────────────────────────────────────

/// Returns the human-readable name for a difficulty tier.
///
/// # Examples
/// ```
/// assert_eq!(menu_with_options::difficulty_name(0), "Easy");
/// assert_eq!(menu_with_options::difficulty_name(1), "Normal");
/// assert_eq!(menu_with_options::difficulty_name(2), "Hard");
/// assert_eq!(menu_with_options::difficulty_name(99), "Normal");
/// ```
pub fn difficulty_name(d: u8) -> &'static str {
    match d {
        0 => "Easy",
        2 => "Hard",
        _ => "Normal",
    }
}

/// Converts a 0-to-100 volume level to a decibel value in `[-80, 0]`.
///
/// Volume 0 maps to −80 dB (silence); volume 100 maps to 0 dB (full).
///
/// # Examples
/// ```
/// assert!((menu_with_options::volume_to_db(100.0) - 0.0).abs() < 1e-4);
/// assert!((menu_with_options::volume_to_db(0.0) - (-80.0)).abs() < 1e-4);
/// assert!((menu_with_options::volume_to_db(50.0) - (-40.0)).abs() < 1e-4);
/// ```
pub fn volume_to_db(v: f32) -> f32 {
    let clamped = v.clamp(0.0, 100.0);
    // Linear interpolation: 0→-80, 100→0
    -80.0 + clamped * 0.8
}

/// Clamps a volume value to the valid range `[0, 100]`.
///
/// # Examples
/// ```
/// assert_eq!(menu_with_options::validate_volume(50.0), 50.0);
/// assert_eq!(menu_with_options::validate_volume(-10.0), 0.0);
/// assert_eq!(menu_with_options::validate_volume(110.0), 100.0);
/// ```
pub fn validate_volume(v: f32) -> f32 {
    v.clamp(0.0, 100.0)
}

// ─── OptionsMenu node ─────────────────────────────────────────────────────────

/// A `Control` that hosts an options/settings screen.
///
/// Expected scene children (exact names):
/// - `"DifficultyOption"` — `OptionButton` with Easy / Normal / Hard items
/// - `"FullscreenCheck"`  — `CheckBox`
/// - `"VolumeSlider"`     — `HSlider` (min=0, max=100)
/// - `"ApplyButton"`      — `Button` whose `pressed` signal is connected here
#[derive(GodotClass)]
#[class(base=Control)]
pub struct OptionsMenu {
    /// Most recently applied configuration.
    config: MenuConfig,
    base: Base<Control>,
}

#[godot_api]
impl IControl for OptionsMenu {
    fn init(base: Base<Control>) -> Self {
        Self {
            config: MenuConfig::default(),
            base,
        }
    }

    fn ready(&mut self) {
        // Populate difficulty options.
        if let Some(mut opt) = self
            .base()
            .try_get_node_as::<OptionButton>("DifficultyOption")
        {
            opt.add_item("Easy");
            opt.add_item("Normal");
            opt.add_item("Hard");
            opt.select(1); // default: Normal
        }

        // Connect Apply button.
        if let Some(mut button) = self.base().try_get_node_as::<Button>("ApplyButton") {
            let callable = self.base().callable("on_apply");
            button.connect("pressed", &callable);
        }

        godot_print!("[OptionsMenu] Ready.");
    }
}

#[godot_api]
impl OptionsMenu {
    /// Called when the Apply button is pressed.  Reads all control values and
    /// stores them in `self.config`.
    #[func]
    pub fn on_apply(&mut self) {
        let difficulty = if let Some(opt) = self
            .base()
            .try_get_node_as::<OptionButton>("DifficultyOption")
        {
            opt.get_selected() as u8
        } else {
            1
        };

        let fullscreen =
            if let Some(cb) = self.base().try_get_node_as::<CheckBox>("FullscreenCheck") {
                cb.is_pressed()
            } else {
                false
            };

        let volume = if let Some(slider) = self.base().try_get_node_as::<HSlider>("VolumeSlider") {
            validate_volume(slider.get_value() as f32)
        } else {
            100.0
        };

        self.config = MenuConfig {
            difficulty,
            fullscreen,
            volume,
        };
        godot_print!(
            "[OptionsMenu] Applied — difficulty: {}, fullscreen: {}, volume: {:.0}",
            difficulty_name(difficulty),
            fullscreen,
            volume
        );
    }

    /// Returns the selected difficulty index from the last `on_apply` call.
    #[func]
    pub fn get_difficulty(&self) -> i32 {
        self.config.difficulty as i32
    }

    /// Returns the selected volume (0–100) from the last `on_apply` call.
    #[func]
    pub fn get_volume(&self) -> f32 {
        self.config.volume
    }
}

// ─── Tests ────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    // difficulty_name

    #[test]
    fn difficulty_name_easy() {
        assert_eq!(difficulty_name(0), "Easy");
    }

    #[test]
    fn difficulty_name_normal() {
        assert_eq!(difficulty_name(1), "Normal");
    }

    #[test]
    fn difficulty_name_hard() {
        assert_eq!(difficulty_name(2), "Hard");
    }

    #[test]
    fn difficulty_name_unknown_is_normal() {
        assert_eq!(difficulty_name(99), "Normal");
    }

    // volume_to_db

    #[test]
    fn volume_to_db_max() {
        assert!((volume_to_db(100.0) - 0.0).abs() < 1e-4);
    }

    #[test]
    fn volume_to_db_min() {
        assert!((volume_to_db(0.0) - (-80.0)).abs() < 1e-4);
    }

    #[test]
    fn volume_to_db_half() {
        assert!((volume_to_db(50.0) - (-40.0)).abs() < 1e-4);
    }

    // validate_volume

    #[test]
    fn validate_volume_within_range() {
        assert_eq!(validate_volume(50.0), 50.0);
    }

    #[test]
    fn validate_volume_below_zero() {
        assert_eq!(validate_volume(-10.0), 0.0);
    }

    #[test]
    fn validate_volume_above_max() {
        assert_eq!(validate_volume(110.0), 100.0);
    }
}
