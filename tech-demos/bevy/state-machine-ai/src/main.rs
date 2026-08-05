//! Runnable entry point for the state-machine-AI demo.
//!
//! All the reusable logic lives in the crate library ([`state_machine_ai`]) as
//! [`StateMachineAiPlugin`]. This binary is just the thin harness that boots
//! the engine and adds the plugin — copy `lib.rs` into your own project (or
//! depend on this crate) and add the one plugin to reuse the feature.

use bevy::prelude::*;
use state_machine_ai::StateMachineAiPlugin;

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .add_plugins(StateMachineAiPlugin)
        .run();
}
