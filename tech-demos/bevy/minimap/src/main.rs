//! Runnable entry point for the minimap demo.
//!
//! All the reusable logic lives in the crate library ([`minimap`]) as
//! [`MinimapPlugin`]. This binary is just the thin harness that boots the
//! engine and adds the plugin — copy `lib.rs` into your own project (or depend
//! on this crate) and add the one plugin to reuse the feature.

use bevy::{prelude::*, window::WindowResolution};
use minimap::{MinimapPlugin, WIN_H, WIN_W};

fn main() {
    App::new()
        .add_plugins(DefaultPlugins.set(WindowPlugin {
            primary_window: Some(Window {
                title: "Minimap — WASD to move | Minimap: top-right".to_string(),
                resolution: WindowResolution::new(WIN_W, WIN_H),
                ..default()
            }),
            ..default()
        }))
        .add_plugins(MinimapPlugin)
        .run();
}
