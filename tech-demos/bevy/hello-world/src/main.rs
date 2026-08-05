//! Runnable entry point for the hello-world demo.
//!
//! All the reusable logic lives in the crate library ([`hello_world`]) as
//! [`HelloWorldPlugin`]. This binary is just the thin harness that boots the
//! engine and adds the plugin.

use bevy::prelude::*;
use hello_world::HelloWorldPlugin;

fn main() {
    App::new()
        .add_plugins(MinimalPlugins)
        .add_plugins(HelloWorldPlugin)
        .run();
}
