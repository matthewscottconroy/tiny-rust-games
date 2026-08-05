//! Runnable entry point for the hello-plugin demo.
//!
//! All the reusable logic lives in the crate library ([`hello_plugin`]) as
//! [`HelloPlugin`]. This binary is just the thin harness that boots the engine
//! and adds the plugin.

use bevy::prelude::*;
use hello_plugin::HelloPlugin;

fn main() {
    App::new()
        .add_plugins(MinimalPlugins)
        .add_plugins(HelloPlugin)
        .run();
}
