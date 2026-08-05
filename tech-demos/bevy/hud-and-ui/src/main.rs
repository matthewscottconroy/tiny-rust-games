//! Runnable entry point for the hud-and-ui demo.
//!
//! All the reusable logic lives in the crate library ([`hud_and_ui`]) as
//! [`HudAndUiPlugin`]. This binary is just the thin harness that boots the
//! engine and adds the plugin — copy `lib.rs` into your own project (or depend
//! on this crate) and add the one plugin to reuse the feature.

use bevy::prelude::*;
use hud_and_ui::HudAndUiPlugin;

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .add_plugins(HudAndUiPlugin)
        .run();
}
