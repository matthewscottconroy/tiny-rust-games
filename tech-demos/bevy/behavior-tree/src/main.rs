//! Runnable entry point for the behavior-tree demo.
//!
//! All the reusable logic lives in the crate library ([`behavior_tree`]) as
//! [`BehaviorTreePlugin`]. This binary is just the thin harness that boots the
//! engine with a window and adds the plugin.
//!
//! **Controls:** WASD / Arrow keys — move the player toward the guard to
//! trigger its behavior tree (patrol → chase → attack).

use behavior_tree::BehaviorTreePlugin;
use bevy::prelude::*;

fn main() {
    App::new()
        .add_plugins(DefaultPlugins.set(WindowPlugin {
            primary_window: Some(Window {
                title: "Behavior Tree".into(),
                resolution: (800u32, 500u32).into(),
                ..default()
            }),
            ..default()
        }))
        .add_plugins(BehaviorTreePlugin)
        .run();
}
