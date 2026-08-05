//! Runnable entry point for the mouse-input demo.
//!
//! All the reusable logic lives in the crate library ([`mouse_input`]) as
//! [`MouseInputPlugin`]. This binary is just the thin harness that boots the
//! engine and adds the plugin — copy `lib.rs` into your own project (or depend
//! on this crate) and add the one plugin to reuse the feature.

use bevy::prelude::*;
use mouse_input::MouseInputPlugin;

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .add_plugins(MouseInputPlugin)
        .run();
}
