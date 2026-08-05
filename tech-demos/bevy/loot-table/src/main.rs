//! Runnable entry point for the loot-table demo.
//!
//! All the reusable logic lives in the crate library ([`loot_table`]) as
//! [`LootTablePlugin`]. This binary is just the thin harness that boots the
//! engine and adds the plugin — copy `lib.rs` into your own project (or depend
//! on this crate) and add the one plugin to reuse the feature.

use bevy::prelude::*;
use bevy::window::WindowResolution;
use loot_table::LootTablePlugin;

fn main() {
    App::new()
        .add_plugins(DefaultPlugins.set(WindowPlugin {
            primary_window: Some(Window {
                title: "Loot Table — click enemies to kill them".into(),
                resolution: WindowResolution::from((720u32, 460u32)),
                ..default()
            }),
            ..default()
        }))
        .add_plugins(LootTablePlugin)
        .run();
}
