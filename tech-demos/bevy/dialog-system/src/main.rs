//! Runnable entry point for the dialog-system demo.
//!
//! All the reusable logic lives in the crate library ([`dialog_system`]) as
//! [`DialogSystemPlugin`]. This binary is just the thin harness that boots the
//! engine and adds the plugin — copy `lib.rs` into your own project (or depend
//! on this crate) and add the one plugin to reuse the feature.

use bevy::prelude::*;
use bevy::window::WindowResolution;
use dialog_system::DialogSystemPlugin;

fn main() {
    App::new()
        .add_plugins(DefaultPlugins.set(WindowPlugin {
            primary_window: Some(Window {
                title: "Dialog System — SPACE / 1 2 3 to advance".to_string(),
                resolution: WindowResolution::from((720u32, 480u32)),
                ..default()
            }),
            ..default()
        }))
        .add_plugins(DialogSystemPlugin)
        .run();
}
