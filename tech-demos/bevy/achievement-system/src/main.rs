//! Runnable entry point for the achievement-system demo.
//!
//! All the reusable logic lives in the crate library ([`achievement_system`]) as
//! [`AchievementSystemPlugin`]. This binary is just the thin harness that boots
//! the engine and adds the plugin — copy `lib.rs` into your own project (or
//! depend on this crate) and add the one plugin to reuse the feature.

use bevy::prelude::*;
use achievement_system::AchievementSystemPlugin;

fn main() {
    App::new()
        .add_plugins(DefaultPlugins.set(WindowPlugin {
            primary_window: Some(Window {
                title: "Achievement System — SPACE score, WASD steps, K kill".into(),
                resolution: (700u32, 460u32).into(),
                ..default()
            }),
            ..default()
        }))
        .add_plugins(AchievementSystemPlugin)
        .run();
}
