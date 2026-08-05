//! Runnable entry point for the enemy-chase-ai demo.
//!
//! All the reusable logic lives in the crate library ([`enemy_chase_ai`]) as
//! [`EnemyChaseAiPlugin`]. This binary is just the thin harness that boots the
//! engine and adds the plugin — copy `lib.rs` into your own project (or depend
//! on this crate) and add the one plugin to reuse the feature.

use bevy::prelude::*;
use enemy_chase_ai::EnemyChaseAiPlugin;

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .add_plugins(EnemyChaseAiPlugin)
        .run();
}
