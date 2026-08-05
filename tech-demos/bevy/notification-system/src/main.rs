//! Runnable entry point for the notification-system demo.
//!
//! All the reusable logic lives in the crate library ([`notification_system`])
//! as [`NotificationSystemPlugin`]. This binary is just the thin harness that
//! boots the engine and adds the plugin — copy `lib.rs` into your own project
//! (or depend on this crate) and add the one plugin to reuse the feature.

use bevy::prelude::*;
use notification_system::NotificationSystemPlugin;

fn main() {
    App::new()
        .add_plugins(DefaultPlugins.set(WindowPlugin {
            primary_window: Some(Window {
                title: "Notification System".into(),
                resolution: (700u32, 420u32).into(),
                ..default()
            }),
            ..default()
        }))
        .add_plugins(NotificationSystemPlugin)
        .run();
}
