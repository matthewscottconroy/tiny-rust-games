//! Runnable entry point for the knockback-hitstop demo.
//!
//! All the reusable logic lives in the crate library ([`knockback_hitstop`]) as
//! [`KnockbackHitstopPlugin`]. This binary is just the thin harness that boots
//! the engine and adds the plugin — copy `lib.rs` into your own project (or
//! depend on this crate) and add the one plugin to reuse the feature.

use bevy::prelude::*;
use knockback_hitstop::KnockbackHitstopPlugin;

fn main() {
    App::new()
        .add_plugins(DefaultPlugins.set(WindowPlugin {
            primary_window: Some(Window {
                title: "Knockback & Hit-Stop".into(),
                resolution: (800u32, 500u32).into(),
                ..default()
            }),
            ..default()
        }))
        .add_plugins(KnockbackHitstopPlugin)
        .run();
}
