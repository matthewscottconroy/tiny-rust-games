//! Runnable entry point for the combo-system demo.
//!
//! All the reusable logic lives in the crate library ([`combo_system`]) as
//! [`ComboSystemPlugin`]. This binary is just the thin harness that boots the
//! engine and adds the plugin — copy `lib.rs` into your own project (or depend
//! on this crate) and add the one plugin to reuse the feature.

use bevy::prelude::*;
use combo_system::ComboSystemPlugin;

fn main() {
    App::new()
        .add_plugins(DefaultPlugins.set(WindowPlugin {
            primary_window: Some(Window {
                title: "Combo System — Arrow keys to enter inputs".to_string(),
                resolution: (700u32, 400u32).into(),
                ..default()
            }),
            ..default()
        }))
        .add_plugins(ComboSystemPlugin)
        .run();
}
