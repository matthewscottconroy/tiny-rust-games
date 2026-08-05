//! Runnable entry point for the stealth-AI demo.
//!
//! All the reusable logic lives in the crate library ([`stealth_ai`]) as
//! [`StealthAiPlugin`]. This binary is just the thin harness that boots the
//! engine and adds the plugin — copy `lib.rs` into your own project (or depend
//! on this crate) and add the one plugin to reuse the feature.

use bevy::prelude::*;
use bevy::window::WindowResolution;
use stealth_ai::StealthAiPlugin;

fn main() {
    App::new()
        .add_plugins(DefaultPlugins.set(WindowPlugin {
            primary_window: Some(Window {
                title: "Stealth AI — WASD to move, avoid the guard".to_string(),
                resolution: WindowResolution::from((800u32, 500u32)),
                ..default()
            }),
            ..default()
        }))
        .add_plugins(StealthAiPlugin)
        .run();
}
