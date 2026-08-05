//! Runnable entry point for the procedural-dungeon demo.
//!
//! All the reusable logic lives in the crate library ([`procedural_dungeon`]) as
//! [`ProceduralDungeonPlugin`]. This binary is just the thin harness that boots
//! the engine and adds the plugin — copy `lib.rs` into your own project (or
//! depend on this crate) and add the one plugin to reuse the feature.
//!
//! **Controls:** SPACE — regenerate dungeon.

use bevy::prelude::*;
use bevy::window::WindowResolution;
use procedural_dungeon::ProceduralDungeonPlugin;

fn main() {
    App::new()
        .add_plugins(DefaultPlugins.set(WindowPlugin {
            primary_window: Some(Window {
                title: "Procedural Dungeon — SPACE to regenerate".to_string(),
                resolution: WindowResolution::from((864u32, 534u32)),
                ..default()
            }),
            ..default()
        }))
        .add_plugins(ProceduralDungeonPlugin)
        .run();
}
