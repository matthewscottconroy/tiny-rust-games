//! Runnable entry point for the audio demo.
//!
//! All the reusable logic lives in the crate library ([`audio`]) as
//! [`AudioPlugin`]. This binary is just the thin harness that boots the engine
//! and adds the plugin — copy `lib.rs` into your own project (or depend on this
//! crate) and add the one plugin to reuse the feature.

use bevy::prelude::*;
use audio::AudioPlugin;

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .add_plugins(AudioPlugin)
        .run();
}
