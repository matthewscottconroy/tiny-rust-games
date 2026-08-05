//! Runnable entry point for the gamepad-input demo.
//!
//! All the reusable logic lives in the crate library ([`gamepad_input`]) as
//! [`GamepadInputPlugin`]. This binary is just the thin harness that boots the
//! engine and adds the plugin — copy `lib.rs` into your own project (or depend
//! on this crate) and add the one plugin to reuse the feature.

use bevy::prelude::*;
use gamepad_input::GamepadInputPlugin;

fn main() {
    App::new()
        .add_plugins(DefaultPlugins.set(WindowPlugin {
            primary_window: Some(Window {
                title: "Gamepad Input Demo".to_string(),
                resolution: (800, 500).into(),
                ..default()
            }),
            ..default()
        }))
        .add_plugins(GamepadInputPlugin)
        .run();
}
