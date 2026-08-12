//! Music With Transitions GDExtension demo — crossfade between music tracks from Rust.
//!
//! Teaches:
//!
//! - Holding two `AudioStreamPlayer` children (`PlayerA` and `PlayerB`) for
//!   smooth crossfading between music tracks.
//! - `crossfade_to(path)` loads a new stream on the inactive player, starts it
//!   at −80 dB, and begins a timed fade.
//! - Each `process()` tick advances `fade_progress`; volumes are lerped so the
//!   outgoing track fades out while the incoming track fades in.
//! - When the fade completes, the old player is stopped.
//! - Pure math helpers (`crossfade_volumes`, `lerp_db`) fully covered by tests.

use godot::classes::{AudioStreamPlayer, INode, ResourceLoader};
use godot::prelude::*;

// ─── Extension entry point ───────────────────────────────────────────────────

struct MusicWithTransitionsExt;

#[gdextension]
unsafe impl ExtensionLibrary for MusicWithTransitionsExt {}

// ─── Pure functions ───────────────────────────────────────────────────────────

/// Returns `(out_vol_db, in_vol_db)` for a crossfade at normalised `progress`
/// (0.0 = start of fade, 1.0 = end of fade).
///
/// The outgoing track fades from 0 dB to −80 dB; the incoming track fades
/// from −80 dB to 0 dB.
///
/// # Examples
/// ```
/// let (out, in_) = music_with_transitions::crossfade_volumes(0.0);
/// assert!((out - 0.0).abs() < 1e-5);
/// assert!((in_ - (-80.0)).abs() < 1e-5);
///
/// let (out, in_) = music_with_transitions::crossfade_volumes(1.0);
/// assert!((out - (-80.0)).abs() < 1e-5);
/// assert!((in_ - 0.0).abs() < 1e-5);
///
/// let (out, in_) = music_with_transitions::crossfade_volumes(0.5);
/// assert!((out - (-40.0)).abs() < 1e-4);
/// assert!((in_ - (-40.0)).abs() < 1e-4);
/// ```
pub fn crossfade_volumes(progress: f64) -> (f32, f32) {
    let t = progress.clamp(0.0, 1.0) as f32;
    let out_vol = lerp_db(0.0, -80.0, progress);
    let in_vol = lerp_db(-80.0, 0.0, progress);
    let _ = t; // t used via lerp_db
    (out_vol, in_vol)
}

/// Linearly interpolates between two dB values.
///
/// # Examples
/// ```
/// assert!((music_with_transitions::lerp_db(0.0, -80.0, 0.0) - 0.0).abs() < 1e-5);
/// assert!((music_with_transitions::lerp_db(0.0, -80.0, 1.0) - (-80.0)).abs() < 1e-5);
/// assert!((music_with_transitions::lerp_db(0.0, -80.0, 0.5) - (-40.0)).abs() < 1e-4);
/// ```
pub fn lerp_db(a: f32, b: f32, t: f64) -> f32 {
    let t = t.clamp(0.0, 1.0) as f32;
    a + (b - a) * t
}

// ─── MusicPlayer node ─────────────────────────────────────────────────────────

/// A `Node` that crossfades between two `AudioStreamPlayer` children.
///
/// `PlayerA` and `PlayerB` are alternated as the active/inactive player.
/// Calling `crossfade_to(path)` begins a smooth dB-space fade over
/// `fade_duration` seconds.
#[derive(GodotClass)]
#[class(base=Node)]
pub struct MusicPlayer {
    /// Duration of each crossfade in seconds.
    #[export]
    fade_duration: f64,
    /// Index of the currently active player: 0 = A, 1 = B.
    active: u8,
    /// Normalised fade progress (0.0 → 1.0).
    fade_progress: f64,
    /// Whether a crossfade is currently in progress.
    fading: bool,
    base: Base<Node>,
}

#[godot_api]
impl INode for MusicPlayer {
    fn init(base: Base<Node>) -> Self {
        Self {
            fade_duration: 2.0,
            active: 0,
            fade_progress: 0.0,
            fading: false,
            base,
        }
    }

    fn ready(&mut self) {
        godot_print!(
            "[MusicPlayer] Ready — fade duration: {:.1}s.",
            self.fade_duration
        );
    }

    fn process(&mut self, delta: f64) {
        if !self.fading {
            return;
        }

        let duration = self.fade_duration;
        self.fade_progress += delta / duration;

        let progress = self.fade_progress;
        let (out_vol, in_vol) = crossfade_volumes(progress);

        let active = self.active;
        let (active_name, inactive_name) = if active == 0 {
            ("PlayerA", "PlayerB")
        } else {
            ("PlayerB", "PlayerA")
        };

        if let Some(mut out_player) = self
            .base()
            .try_get_node_as::<AudioStreamPlayer>(active_name)
        {
            out_player.set_volume_db(out_vol);
        }
        if let Some(mut in_player) = self
            .base()
            .try_get_node_as::<AudioStreamPlayer>(inactive_name)
        {
            in_player.set_volume_db(in_vol);
        }

        if self.fade_progress >= 1.0 {
            // Fade complete — stop the old player and swap active.
            if let Some(mut old_player) = self
                .base()
                .try_get_node_as::<AudioStreamPlayer>(active_name)
            {
                old_player.stop();
            }
            self.active = 1 - self.active;
            self.fading = false;
            self.fade_progress = 0.0;
            godot_print!("[MusicPlayer] Crossfade complete.");
        }
    }
}

#[godot_api]
impl MusicPlayer {
    /// Begins a crossfade to the audio stream at `stream_path`.
    ///
    /// The stream is loaded onto the currently inactive player, which starts
    /// at −80 dB.  The fade begins immediately.
    #[func]
    pub fn crossfade_to(&mut self, stream_path: GString) {
        // Determine which player is inactive.
        let inactive_name = if self.active == 0 {
            "PlayerB"
        } else {
            "PlayerA"
        };

        let mut loader = ResourceLoader::singleton();
        if let Some(res) = loader.load(&stream_path)
            && let Ok(stream) = res.try_cast::<godot::classes::AudioStream>()
            && let Some(mut player) = self
                .base()
                .try_get_node_as::<AudioStreamPlayer>(inactive_name)
        {
            player.set_stream(&stream);
            player.set_volume_db(-80.0);
            player.play();
        }

        self.fade_progress = 0.0;
        self.fading = true;
        godot_print!("[MusicPlayer] Crossfade started → '{}'.", stream_path);
    }

    /// Returns `true` while a crossfade is in progress.
    #[func]
    pub fn is_fading(&self) -> bool {
        self.fading
    }
}

// ─── Tests ────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    // lerp_db

    #[test]
    fn lerp_db_at_zero() {
        assert!((lerp_db(0.0, -80.0, 0.0) - 0.0).abs() < 1e-5);
    }

    #[test]
    fn lerp_db_at_one() {
        assert!((lerp_db(0.0, -80.0, 1.0) - (-80.0)).abs() < 1e-5);
    }

    #[test]
    fn lerp_db_at_half() {
        assert!((lerp_db(0.0, -80.0, 0.5) - (-40.0)).abs() < 1e-4);
    }

    // crossfade_volumes

    #[test]
    fn crossfade_volumes_start() {
        let (out, in_) = crossfade_volumes(0.0);
        assert!((out - 0.0).abs() < 1e-5);
        assert!((in_ - (-80.0)).abs() < 1e-5);
    }

    #[test]
    fn crossfade_volumes_end() {
        let (out, in_) = crossfade_volumes(1.0);
        assert!((out - (-80.0)).abs() < 1e-5);
        assert!((in_ - 0.0).abs() < 1e-5);
    }

    #[test]
    fn crossfade_volumes_midpoint() {
        let (out, in_) = crossfade_volumes(0.5);
        assert!((out - (-40.0)).abs() < 1e-4);
        assert!((in_ - (-40.0)).abs() < 1e-4);
    }

    #[test]
    fn crossfade_volumes_clamps_above_one() {
        let (out, _in_) = crossfade_volumes(2.0);
        assert!((out - (-80.0)).abs() < 1e-5);
    }

    #[test]
    fn crossfade_volumes_clamps_below_zero() {
        let (_out, in_) = crossfade_volumes(-1.0);
        assert!((in_ - (-80.0)).abs() < 1e-5);
    }
}
