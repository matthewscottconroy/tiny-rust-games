//! Runnable entry point for the line-of-sight demo.
//!
//! All the reusable logic lives in the crate library ([`line_of_sight`]) as
//! [`LineOfSightPlugin`]. This binary is just the thin harness that boots the
//! engine and adds the plugin — copy `lib.rs` into your own project (or depend
//! on this crate) and add the one plugin to reuse the feature.

use bevy::prelude::*;
use bevy::window::WindowResolution;
use line_of_sight::LineOfSightPlugin;

fn main() {
    App::new()
        .add_plugins(DefaultPlugins.set(WindowPlugin {
            primary_window: Some(Window {
                title: "Line of Sight — WASD to move".to_string(),
                resolution: WindowResolution::from((780u32, 548u32)),
                ..default()
            }),
            ..default()
        }))
        .add_plugins(LineOfSightPlugin)
        .run();
}
