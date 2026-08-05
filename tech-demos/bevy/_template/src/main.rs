//! Runnable entry point for the template demo.
//!
//! The reusable feature lives in the crate library ([`template`]) as
//! [`TemplatePlugin`]. This binary is just the thin harness that boots the
//! engine and adds the plugin.

use bevy::prelude::*;
use template::TemplatePlugin;

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .add_plugins(TemplatePlugin)
        .run();
}
