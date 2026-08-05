//! Runnable entry point for the spritesheet-animation demo.
//!
//! All the reusable logic lives in the crate library ([`spritesheet_animation`])
//! as [`SpritesheetAnimationPlugin`]. This binary is just the thin harness that
//! boots the engine and adds the plugin — copy `lib.rs` into your own project
//! (or depend on this crate) and add the one plugin to reuse the feature.

use bevy::prelude::*;
use spritesheet_animation::SpritesheetAnimationPlugin;

fn main() {
    App::new()
        .add_plugins(DefaultPlugins.set(ImagePlugin::default_nearest()))
        .add_plugins(SpritesheetAnimationPlugin)
        .run();
}
