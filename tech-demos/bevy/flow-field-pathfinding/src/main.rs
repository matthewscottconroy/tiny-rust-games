//! Runnable entry point for the flow-field pathfinding demo.
//!
//! All the reusable logic lives in the crate library ([`flow_field_pathfinding`])
//! as [`FlowFieldPathfindingPlugin`]. This binary is just the thin harness that
//! boots the engine, configures the window, and adds the plugin — copy `lib.rs`
//! into your own project (or depend on this crate) and add the one plugin to
//! reuse the feature.

use bevy::prelude::*;
use flow_field_pathfinding::FlowFieldPathfindingPlugin;

fn main() {
    App::new()
        .add_plugins(DefaultPlugins.set(WindowPlugin {
            primary_window: Some(Window {
                title: "Flow-Field Pathfinding".into(),
                resolution: (900u32, 600u32).into(),
                ..default()
            }),
            ..default()
        }))
        .add_plugins(FlowFieldPathfindingPlugin)
        .run();
}
