//! Audio Manager GDExtension demo — polyphonic SFX pool management from Rust.
//!
//! Demonstrates:
//!
//! - Managing a fixed pool of `AudioStreamPlayer` children for polyphonic playback.
//! - Loading an `AudioStream` resource at runtime and assigning it to a free player.
//! - `stop_all()` and `set_master_volume()` bulk operations on the player pool.
//! - Tracking play statistics via a `play_count` field.
//! - Pure audio-math helpers (`db_to_linear`, `linear_to_db`, `clamp_volume`)
//!   covered by unit tests.

use godot::classes::{AudioStreamPlayer, INode, ResourceLoader};
use godot::prelude::*;

// ─── Extension entry point ───────────────────────────────────────────────────

struct AudioManagerExt;

#[gdextension]
unsafe impl ExtensionLibrary for AudioManagerExt {}

// ─── Pure functions ───────────────────────────────────────────────────────────

/// Converts a decibel value to a linear amplitude factor.
///
/// `db_to_linear(0.0) == 1.0`, `db_to_linear(-20.0) ≈ 0.1`.
///
/// # Examples
/// ```
/// assert!((audio_manager::db_to_linear(0.0) - 1.0).abs() < 1e-5);
/// assert!((audio_manager::db_to_linear(-20.0) - 0.1).abs() < 1e-4);
/// ```
pub fn db_to_linear(db: f32) -> f32 {
    10.0_f32.powf(db / 20.0)
}

/// Converts a linear amplitude factor to decibels.
///
/// Returns `-80.0` for inputs ≤ 0 to avoid −∞.
///
/// # Examples
/// ```
/// assert!((audio_manager::linear_to_db(1.0) - 0.0).abs() < 1e-4);
/// assert!((audio_manager::linear_to_db(0.1) - (-20.0)).abs() < 1e-3);
/// assert_eq!(audio_manager::linear_to_db(0.0), -80.0);
/// ```
pub fn linear_to_db(linear: f32) -> f32 {
    if linear <= 0.0 {
        return -80.0;
    }
    20.0 * linear.log10()
}

/// Clamps a dB value to the valid range `[-80, 6]`.
///
/// # Examples
/// ```
/// assert_eq!(audio_manager::clamp_volume(0.0), 0.0);
/// assert_eq!(audio_manager::clamp_volume(-100.0), -80.0);
/// assert_eq!(audio_manager::clamp_volume(10.0), 6.0);
/// ```
pub fn clamp_volume(db: f32) -> f32 {
    db.clamp(-80.0, 6.0)
}

// ─── AudioManager node ───────────────────────────────────────────────────────

/// A `Node` that maintains a polyphonic pool of `AudioStreamPlayer` children.
///
/// On `ready()` the node expects `pool_size` children named `"Player0"`,
/// `"Player1"`, … in the scene tree.  `play_sound` finds the first idle
/// player, loads the requested stream, and starts playback.
#[derive(GodotClass)]
#[class(base=Node)]
pub struct AudioManager {
    /// Number of players in the pool (must match the scene layout).
    #[export]
    pool_size: i32,
    /// Total number of sounds played since this node entered the scene tree.
    play_count: i32,
    base: Base<Node>,
}

#[godot_api]
impl INode for AudioManager {
    fn init(base: Base<Node>) -> Self {
        Self {
            pool_size: 4,
            play_count: 0,
            base,
        }
    }

    fn ready(&mut self) {
        godot_print!(
            "[AudioManager] Ready — pool size: {}.",
            self.pool_size
        );
    }
}

#[godot_api]
impl AudioManager {
    /// Plays the audio resource at `stream_path` using the first idle pool player.
    ///
    /// If all players are busy the call is silently ignored.
    #[func]
    pub fn play_sound(&mut self, stream_path: GString) {
        let pool_size = self.pool_size;
        for i in 0..pool_size {
            let name = format!("Player{}", i);
            if let Some(mut player) = self.base().try_get_node_as::<AudioStreamPlayer>(&name) {
                if player.is_playing() {
                    continue;
                }
                // Load the stream resource.
                let mut loader = ResourceLoader::singleton();
                if let Some(res) = loader.load(&stream_path) {
                    if let Ok(stream) = res.try_cast::<godot::classes::AudioStream>() {
                        player.set_stream(&stream);
                        player.play();
                        self.play_count += 1;
                        godot_print!(
                            "[AudioManager] Playing '{}' on Player{}. Count: {}",
                            stream_path,
                            i,
                            self.play_count
                        );
                        return;
                    }
                }
                // Stream could not be loaded — skip.
                godot_print!("[AudioManager] Failed to load stream: {}", stream_path);
                return;
            }
        }
        godot_print!("[AudioManager] All players busy — dropping sound.");
    }

    /// Stops all players in the pool immediately.
    #[func]
    pub fn stop_all(&mut self) {
        let pool_size = self.pool_size;
        for i in 0..pool_size {
            let name = format!("Player{}", i);
            if let Some(mut player) = self.base().try_get_node_as::<AudioStreamPlayer>(&name) {
                player.stop();
            }
        }
        godot_print!("[AudioManager] All players stopped.");
    }

    /// Sets the `volume_db` property on every pool player.
    #[func]
    pub fn set_master_volume(&mut self, volume_db: f32) {
        let clamped = clamp_volume(volume_db);
        let pool_size = self.pool_size;
        for i in 0..pool_size {
            let name = format!("Player{}", i);
            if let Some(mut player) = self.base().try_get_node_as::<AudioStreamPlayer>(&name) {
                player.set_volume_db(clamped);
            }
        }
        godot_print!("[AudioManager] Master volume set to {:.1} dB.", clamped);
    }

    /// Returns the total number of sounds played.
    #[func]
    pub fn get_play_count(&self) -> i32 {
        self.play_count
    }
}

// ─── Tests ────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    // db_to_linear

    #[test]
    fn db_to_linear_zero_db() {
        assert!((db_to_linear(0.0) - 1.0).abs() < 1e-5);
    }

    #[test]
    fn db_to_linear_minus_20() {
        assert!((db_to_linear(-20.0) - 0.1).abs() < 1e-4);
    }

    #[test]
    fn db_to_linear_positive_6() {
        // 6 dB ≈ 2.0 (actually ~1.995)
        assert!(db_to_linear(6.0) > 1.9 && db_to_linear(6.0) < 2.1);
    }

    // linear_to_db

    #[test]
    fn linear_to_db_one() {
        assert!((linear_to_db(1.0) - 0.0).abs() < 1e-4);
    }

    #[test]
    fn linear_to_db_tenth() {
        assert!((linear_to_db(0.1) - (-20.0)).abs() < 1e-3);
    }

    #[test]
    fn linear_to_db_zero_returns_minus_80() {
        assert_eq!(linear_to_db(0.0), -80.0);
    }

    // clamp_volume

    #[test]
    fn clamp_volume_within_range() {
        assert_eq!(clamp_volume(0.0), 0.0);
    }

    #[test]
    fn clamp_volume_below_min() {
        assert_eq!(clamp_volume(-100.0), -80.0);
    }

    #[test]
    fn clamp_volume_above_max() {
        assert_eq!(clamp_volume(10.0), 6.0);
    }
}
