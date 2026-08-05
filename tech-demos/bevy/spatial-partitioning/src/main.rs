//! Runnable entry point for the spatial-partitioning demo.
use bevy::prelude::*;
use spatial_partitioning::SpatialPartitioningPlugin;

fn main() {
    App::new()
        .add_plugins(DefaultPlugins.set(WindowPlugin {
            primary_window: Some(Window {
                title: "Spatial Partitioning".into(),
                resolution: (800u32, 500u32).into(),
                ..default()
            }),
            ..default()
        }))
        .add_plugins(SpatialPartitioningPlugin)
        .run();
}
