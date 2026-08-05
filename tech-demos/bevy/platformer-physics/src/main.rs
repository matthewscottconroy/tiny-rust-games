//! Runnable entry point for the platformer-physics demo.
//!
//! All the reusable logic lives in the crate library ([`platformer_physics`]) as
//! [`PlatformerPhysicsPlugin`]. This binary is just the thin harness that boots
//! the engine and adds the plugin — copy `lib.rs` into your own project (or
//! depend on this crate) and add the one plugin to reuse the feature.
//!
//! **Controls:** A/D or arrow keys to move; SPACE or W/Up to jump.

use bevy::prelude::*;
use platformer_physics::PlatformerPhysicsPlugin;

fn main() {
    App::new()
        .add_plugins(DefaultPlugins.set(WindowPlugin {
            primary_window: Some(Window {
                title: "Platformer Physics — A/D move, SPACE jump".to_string(),
                resolution: (800, 500).into(),
                ..default()
            }),
            ..default()
        }))
        .add_plugins(PlatformerPhysicsPlugin)
        .run();
}
