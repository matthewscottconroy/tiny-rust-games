//! Runnable entry point for the movable-sprite demo.
//!
//! All the reusable logic lives in the crate library ([`movable_sprite`]) as
//! [`MovableSpritePlugin`]. This binary is just the thin harness that boots the
//! engine and adds the plugin.

use bevy::prelude::*;
use movable_sprite::MovableSpritePlugin;

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .add_plugins(MovableSpritePlugin)
        .run();
}
