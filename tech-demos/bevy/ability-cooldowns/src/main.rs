//! Runnable entry point for the ability-cooldowns demo.
//!
//! All the reusable logic lives in the crate library ([`ability_cooldowns`]) as
//! [`AbilityCooldownsPlugin`]. This binary is just the thin harness that boots
//! the engine and adds the plugin — copy `lib.rs` into your own project (or
//! depend on this crate) and add the one plugin to reuse the feature.

use bevy::prelude::*;
use ability_cooldowns::AbilityCooldownsPlugin;

fn main() {
    App::new()
        .add_plugins(DefaultPlugins.set(WindowPlugin {
            primary_window: Some(Window {
                title: "Ability Cooldowns".into(),
                resolution: (800u32, 500u32).into(),
                ..default()
            }),
            ..default()
        }))
        .add_plugins(AbilityCooldownsPlugin)
        .run();
}
