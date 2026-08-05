//! Runnable entry point for the tween-animation demo.
//!
//! All the reusable logic lives in the crate library ([`tween_animation`]) as
//! [`TweenAnimationPlugin`]. This binary is just the thin harness that boots
//! the engine and adds the plugin.
//!
//! **Controls:** 1 — scale pop   2 — alpha pulse   (slide plays on startup).

use bevy::prelude::*;
use tween_animation::TweenAnimationPlugin;

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .add_plugins(TweenAnimationPlugin)
        .run();
}
