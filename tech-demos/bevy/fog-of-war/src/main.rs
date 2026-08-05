//! Runnable entry point for the fog-of-war demo.
//!
//! All the reusable logic lives in the crate library ([`fog_of_war`]) as
//! [`FogOfWarPlugin`]. This binary is just the thin harness that boots the
//! engine (with a titled window) and adds the plugin — copy `lib.rs` into your
//! own project (or depend on this crate) and add the one plugin to reuse the
//! feature.

use bevy::prelude::*;
use bevy::window::WindowResolution;
use fog_of_war::FogOfWarPlugin;

fn main() {
    App::new()
        .add_plugins(DefaultPlugins.set(WindowPlugin {
            primary_window: Some(Window {
                title: "Fog of War — WASD to move".to_string(),
                resolution: WindowResolution::from((768u32, 556u32)),
                ..default()
            }),
            ..default()
        }))
        .add_plugins(FogOfWarPlugin)
        .run();
}
