//! Runnable entry point for the bullet-pattern demo.
//!
//! All the reusable logic lives in the crate library ([`bullet_pattern`]) as
//! [`BulletPatternPlugin`]. This binary is just the thin harness that boots the
//! engine and adds the plugin.

use bevy::prelude::*;
use bullet_pattern::BulletPatternPlugin;

fn main() {
    App::new()
        .add_plugins(DefaultPlugins.set(WindowPlugin {
            primary_window: Some(Window {
                title: "Bullet Pattern".into(),
                resolution: (800u32, 600u32).into(),
                ..default()
            }),
            ..default()
        }))
        .add_plugins(BulletPatternPlugin)
        .run();
}
