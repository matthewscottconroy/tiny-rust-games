//! Runnable entry point for the object-pooling demo.
//!
//! All the reusable logic lives in the crate library ([`object_pooling`]) as
//! [`ObjectPoolingPlugin`]. This binary is just the thin harness that boots the
//! engine and adds the plugin — copy `lib.rs` into your own project (or depend
//! on this crate) and add the one plugin to reuse the feature.

use bevy::prelude::*;
use object_pooling::ObjectPoolingPlugin;

fn main() {
    App::new()
        .add_plugins(DefaultPlugins.set(WindowPlugin {
            primary_window: Some(Window {
                title: "Object Pooling — SPACE / click to fire".to_string(),
                resolution: (800u32, 500u32).into(),
                ..default()
            }),
            ..default()
        }))
        .add_plugins(ObjectPoolingPlugin)
        .run();
}
