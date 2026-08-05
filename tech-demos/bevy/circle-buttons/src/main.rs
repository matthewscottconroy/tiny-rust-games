//! Runnable entry point for the circle-buttons demo.
//!
//! All the reusable logic lives in the crate library ([`circle_buttons`]) as
//! [`CircleGuiPlugin`]. This binary is just the thin harness that boots the
//! engine, inserts a [`CircleGuiConfig`], spawns the camera, and reads
//! [`CircleClicked`] messages from outside the plugin — copy `lib.rs` into your
//! own project (or depend on this crate) and add the one plugin to reuse it.

use bevy::prelude::*;
use circle_buttons::{CircleClicked, CircleGuiConfig, CircleGuiPlugin};

fn main() {
    App::new()
        .add_plugins(DefaultPlugins.set(WindowPlugin {
            primary_window: Some(Window {
                title: "Circle Buttons".to_string(),
                resolution: (860u32, 320u32).into(),
                ..default()
            }),
            ..default()
        }))
        // Caller inserts config BEFORE the plugin.
        .insert_resource(CircleGuiConfig {
            count: 6,
            radius: 48.0,
            spacing: 130.0,
            colors: vec![
                Color::srgb(0.88, 0.22, 0.22),
                Color::srgb(0.22, 0.75, 0.30),
                Color::srgb(0.22, 0.45, 0.90),
                Color::srgb(0.92, 0.80, 0.10),
                Color::srgb(0.80, 0.22, 0.80),
                Color::srgb(0.10, 0.80, 0.80),
            ],
        })
        .add_plugins(CircleGuiPlugin)
        // Camera — the plugin does not own this; the app does.
        .add_systems(Startup, |mut commands: Commands| {
            commands.spawn(Camera2d);
        })
        // External consumer: reads CircleClicked messages from outside the plugin.
        .add_systems(Update, log_clicks)
        .run();
}

/// Example of reading click results from outside the plugin.
fn log_clicks(mut events: MessageReader<CircleClicked>) {
    for ev in events.read() {
        println!(
            "[external] circle {} clicked at ({:.0}, {:.0})",
            ev.index, ev.position.x, ev.position.y
        );
    }
}
