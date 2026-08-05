//! Runnable entry point for the one-shot-systems demo.
//!
//! All the reusable logic lives in the crate library ([`one_shot_systems`]) as
//! [`OneShotSystemsPlugin`]. This binary is just the thin harness that boots the
//! engine and adds the plugin — copy `lib.rs` into your own project (or depend
//! on this crate) and add the one plugin to reuse the feature.

use bevy::prelude::*;
use one_shot_systems::OneShotSystemsPlugin;

fn main() {
    App::new()
        .add_plugins(DefaultPlugins.set(WindowPlugin {
            primary_window: Some(Window {
                title: "One-Shot Systems — Click Buttons".into(),
                resolution: (700u32, 450u32).into(),
                ..default()
            }),
            ..default()
        }))
        .add_plugins(OneShotSystemsPlugin)
        .run();
}
