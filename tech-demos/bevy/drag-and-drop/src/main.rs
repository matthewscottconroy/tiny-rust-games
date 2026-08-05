//! Runnable entry point for the drag-and-drop demo.
//!
//! All the reusable logic lives in the crate library ([`drag_and_drop`]) as
//! [`DragAndDropPlugin`]. This binary is just the thin harness that boots the
//! engine, configures the window, and adds the plugin — copy `lib.rs` into your
//! own project (or depend on this crate) and add the one plugin to reuse the
//! feature.

use bevy::prelude::*;
use drag_and_drop::DragAndDropPlugin;

fn main() {
    App::new()
        .add_plugins(DefaultPlugins.set(WindowPlugin {
            primary_window: Some(Window {
                title: "Drag and Drop Demo".into(),
                resolution: (900u32, 600u32).into(),
                ..default()
            }),
            ..default()
        }))
        .add_plugins(DragAndDropPlugin)
        .run();
}
