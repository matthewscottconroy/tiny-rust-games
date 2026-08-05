//! Runnable entry point for the y-sort demo.
//!
//! All the reusable logic lives in the crate library ([`y_sort`]) as
//! [`YSortPlugin`]. This binary is just the thin harness that boots the engine
//! and adds the plugin — copy `lib.rs` into your own project (or depend on this
//! crate) and add the one plugin to reuse the feature.

use bevy::prelude::*;
use y_sort::YSortPlugin;

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .add_plugins(YSortPlugin)
        .run();
}
