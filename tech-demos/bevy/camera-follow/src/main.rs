//! Runnable entry point for the camera-follow demo.
//!
//! All the reusable logic lives in the crate library ([`camera_follow`]) as
//! [`CameraFollowPlugin`]. This binary is just the thin harness that boots the
//! engine and adds the plugin.

use bevy::prelude::*;
use camera_follow::CameraFollowPlugin;

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .add_plugins(CameraFollowPlugin)
        .run();
}
