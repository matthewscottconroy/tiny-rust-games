//! Runnable entry point for the upgrade-tree demo.
//!
//! All the reusable logic lives in the crate library ([`upgrade_tree`]) as
//! [`UpgradeTreePlugin`]. This binary is just the thin harness that boots the
//! engine and adds the plugin.

use bevy::prelude::*;
use upgrade_tree::UpgradeTreePlugin;

fn main() {
    App::new()
        .add_plugins(DefaultPlugins.set(WindowPlugin {
            primary_window: Some(Window {
                title: "Upgrade Tree".into(),
                resolution: (800u32, 500u32).into(),
                ..default()
            }),
            ..default()
        }))
        .add_plugins(UpgradeTreePlugin)
        .run();
}
