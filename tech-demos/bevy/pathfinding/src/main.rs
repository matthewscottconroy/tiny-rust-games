//! Runnable entry point for the pathfinding demo.
//!
//! All the reusable logic lives in the crate library ([`pathfinding`]) as
//! [`PathfindingPlugin`]. This binary is just the thin harness that boots the
//! engine and adds the plugin — copy `lib.rs` into your own project (or depend
//! on this crate) and add the one plugin to reuse the feature.
//!
//! **Controls:** Arrow keys / WASD to move the player (cyan).

use bevy::prelude::*;
use pathfinding::PathfindingPlugin;

fn main() {
    App::new()
        .add_plugins(DefaultPlugins.set(WindowPlugin {
            primary_window: Some(Window {
                title: "A* Pathfinding — arrow keys to move".to_string(),
                resolution: (800, 560).into(),
                ..default()
            }),
            ..default()
        }))
        .add_plugins(PathfindingPlugin)
        .run();
}
