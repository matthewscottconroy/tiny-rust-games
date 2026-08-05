//! Runnable entry point for the floating-text demo.
//!
//! All the reusable logic lives in the crate library ([`floating_text`]) as
//! [`FloatingTextPlugin`]. This binary is just the thin harness that boots the
//! engine and adds the plugin — copy `lib.rs` into your own project (or depend
//! on this crate) and add the one plugin to reuse the feature.

use bevy::prelude::*;
use floating_text::FloatingTextPlugin;

fn main() {
    App::new()
        .add_plugins(DefaultPlugins.set(WindowPlugin {
            primary_window: Some(Window {
                title: "Floating Text — click to spawn damage numbers".to_string(),
                resolution: (800, 500).into(),
                ..default()
            }),
            ..default()
        }))
        .add_plugins(FloatingTextPlugin)
        .run();
}
