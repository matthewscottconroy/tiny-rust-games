//! Runnable entry point for the turn-based combat demo.
//!
//! All the reusable logic lives in the crate library ([`turn_based`]) as
//! [`TurnBasedPlugin`]. This binary is just the thin harness that boots the
//! engine and adds the plugin.
//!
//! **Controls:** WASD / Arrow keys — move or attack   SPACE — end turn.

use bevy::prelude::*;
use bevy::window::WindowResolution;
use turn_based::TurnBasedPlugin;

fn main() {
    App::new()
        .add_plugins(DefaultPlugins.set(WindowPlugin {
            primary_window: Some(Window {
                title: "Turn-Based Combat — WASD/move SPACE/end turn".to_string(),
                resolution: WindowResolution::from((644u32, 520u32)),
                ..default()
            }),
            ..default()
        }))
        .add_plugins(TurnBasedPlugin)
        .run();
}
