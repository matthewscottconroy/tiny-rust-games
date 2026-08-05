//! Runnable entry point for the card-game demo.
//!
//! All the reusable logic lives in the crate library ([`card_game`]) as
//! [`CardGamePlugin`]. This binary is just the thin harness that boots the
//! engine and adds the plugin — copy `lib.rs` into your own project (or depend
//! on this crate) and add the one plugin to reuse the feature.

use bevy::prelude::*;
use card_game::CardGamePlugin;

fn main() {
    App::new()
        .add_plugins(DefaultPlugins.set(WindowPlugin {
            primary_window: Some(Window {
                title: "Card Game".into(),
                resolution: (800u32, 600u32).into(),
                ..default()
            }),
            ..default()
        }))
        .add_plugins(CardGamePlugin)
        .run();
}
