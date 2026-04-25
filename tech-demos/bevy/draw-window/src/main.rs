//! Draw window demo — the absolute minimum Bevy app that opens a window.
//!
//! `DefaultPlugins` includes the window, renderer, input, and asset plugins.
//! No custom systems are needed; Bevy's built-in plugins handle the event loop.

use bevy::prelude::*;

fn main() {
    App::new().add_plugins(DefaultPlugins).run();
}
