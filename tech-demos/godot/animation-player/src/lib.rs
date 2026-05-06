//! Animation Player demo — controls an AnimationPlayer timeline from Rust:
//! play, seek, adjust speed scale, and react to the `animation_finished` signal.

use godot::classes::{AnimationPlayer, INode2D, Node2D};
use godot::prelude::*;

struct AnimationPlayerExtension;
#[gdextension]
unsafe impl ExtensionLibrary for AnimationPlayerExtension {}

/// Root Node2D that controls a child AnimationPlayer from Rust.
#[derive(GodotClass)]
#[class(base=Node2D)]
struct AnimationController {
    /// Playback speed multiplier exposed to the Godot editor.
    #[export]
    playback_speed: f32,

    /// Name of the currently playing animation.
    current_anim: String,

    /// Number of times the current animation has looped.
    loop_count: i32,

    base: Base<Node2D>,
}

#[godot_api]
impl INode2D for AnimationController {
    fn init(base: Base<Node2D>) -> Self {
        Self {
            playback_speed: 1.0,
            current_anim: String::from("idle"),
            loop_count: 0,
            base,
        }
    }

    fn ready(&mut self) {
        let speed = self.playback_speed;

        // Configure the AnimationPlayer
        let mut player = self.base().get_node_as::<AnimationPlayer>("AnimationPlayer");
        player.set_speed_scale(speed);

        // Connect animation_finished signal → on_animation_finished
        let callable = Callable::from_object_method(&self.base(), "on_animation_finished");
        player.connect("animation_finished", &callable);

        // Start playing — use call() as a safe fallback since play_ex may
        // not exist if no animation library is loaded at design time.
        player.call("play", &[Variant::from(GString::from("idle"))]);
    }
}

#[godot_api]
impl AnimationController {
    /// Called by the AnimationPlayer when an animation finishes.
    #[func]
    pub fn on_animation_finished(&mut self, anim_name: GString) {
        self.loop_count += 1;
        let count = self.loop_count;
        let name = anim_name.to_string();
        godot_print!(
            "{}",
            format_anim_status(name.as_str(), count, self.playback_speed)
        );
        // Replay to loop
        if let Some(mut player) = self.base().try_get_node_as::<AnimationPlayer>("AnimationPlayer")
        {
            player.call("play", &[Variant::from(anim_name)]);
        }
    }

    /// Play a named animation.
    #[func]
    pub fn play_animation(&mut self, name: GString) {
        self.current_anim = name.to_string();
        if let Some(mut player) = self.base().try_get_node_as::<AnimationPlayer>("AnimationPlayer")
        {
            player.call("play", &[Variant::from(name)]);
        }
    }

    /// Seek the animation timeline to an absolute position (in seconds).
    #[func]
    pub fn seek_to(&mut self, position: f64) {
        if let Some(mut player) = self.base().try_get_node_as::<AnimationPlayer>("AnimationPlayer")
        {
            player.seek(position);
        }
    }

    /// Returns how many times the current animation has looped.
    #[func]
    pub fn get_loop_count(&self) -> i32 {
        self.loop_count
    }
}

// ---------------------------------------------------------------------------
// Pure helper functions
// ---------------------------------------------------------------------------

/// Format a status string showing animation name, loop count, and speed.
pub fn format_anim_status(name: &str, loops: i32, speed: f32) -> String {
    format!("anim='{}' loops={} speed={:.2}x", name, loops, speed)
}

/// Human-readable speed label (e.g. "2.00x").
pub fn speed_display(speed: f32) -> String {
    format!("{:.2}x", speed)
}

/// Pluralised loop counter label.
pub fn loop_label(count: i32) -> String {
    if count == 1 {
        String::from("1 loop")
    } else {
        format!("{} loops", count)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn format_anim_status_basic() {
        let s = format_anim_status("idle", 3, 1.0);
        assert_eq!(s, "anim='idle' loops=3 speed=1.00x");
    }

    #[test]
    fn format_anim_status_zero_loops() {
        let s = format_anim_status("walk", 0, 2.5);
        assert_eq!(s, "anim='walk' loops=0 speed=2.50x");
    }

    #[test]
    fn speed_display_one() {
        assert_eq!(speed_display(1.0), "1.00x");
    }

    #[test]
    fn speed_display_half() {
        assert_eq!(speed_display(0.5), "0.50x");
    }

    #[test]
    fn speed_display_double() {
        assert_eq!(speed_display(2.0), "2.00x");
    }

    #[test]
    fn loop_label_singular() {
        assert_eq!(loop_label(1), "1 loop");
    }

    #[test]
    fn loop_label_plural_two() {
        assert_eq!(loop_label(2), "2 loops");
    }

    #[test]
    fn loop_label_plural_zero() {
        assert_eq!(loop_label(0), "0 loops");
    }

    #[test]
    fn loop_label_large() {
        assert_eq!(loop_label(100), "100 loops");
    }
}
