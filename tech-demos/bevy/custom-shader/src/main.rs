//! Runnable entry point for the custom-shader demo.
//!
//! All the reusable logic lives in the crate library ([`custom_shader`]) as
//! [`CustomShaderPlugin`], which registers the ripple material's render
//! sub-plugin and update systems. This binary is just the thin harness that
//! boots the engine and adds the plugin — copy `lib.rs` into your own project
//! (or depend on this crate) and add the one plugin to reuse the feature.

use bevy::prelude::*;
use custom_shader::CustomShaderPlugin;

fn main() {
    App::new()
        .add_plugins(DefaultPlugins.set(WindowPlugin {
            primary_window: Some(Window {
                title: "Custom Shader — Ripple".into(),
                resolution: (800u32, 600u32).into(),
                ..default()
            }),
            ..default()
        }))
        .add_plugins(CustomShaderPlugin)
        .run();
}
