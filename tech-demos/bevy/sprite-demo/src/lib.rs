//! Sprite loading — a reusable Bevy plugin.
//!
//! This crate is a *building block*: drop [`SpriteDemoPlugin`] into any Bevy app
//! with `app.add_plugins(SpriteDemoPlugin)`. It shows the simplest way to
//! display an image: `AssetServer::load` returns a handle immediately, Bevy
//! loads the file asynchronously in the background, and the sprite appears once
//! ready. Tune the image path via [`SpriteDemoConfig`].
//!
//! Place `assets/sprite.png` relative to this crate's directory.
//!
//! # Example
//! ```no_run
//! use bevy::prelude::*;
//! use sprite_demo::SpriteDemoPlugin;
//!
//! App::new()
//!     .add_plugins(DefaultPlugins)
//!     .add_plugins(SpriteDemoPlugin)
//!     .run();
//! ```

use bevy::prelude::*;

/// Bundles the setup system and config for the sprite demo.
///
/// Add it with `app.add_plugins(SpriteDemoPlugin)`. It does **not** add
/// `DefaultPlugins`, so the host app stays in control of window and rendering
/// setup.
pub struct SpriteDemoPlugin;

impl Plugin for SpriteDemoPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<SpriteDemoConfig>()
            .add_systems(Startup, setup);
    }
}

/// Default asset path loaded when no override is provided.
pub const DEFAULT_SPRITE_PATH: &str = "sprite.png";

/// Tunable parameters for the sprite demo. Override before adding the plugin,
/// e.g. `app.insert_resource(SpriteDemoConfig { sprite_path: "hero.png".into() })`.
#[derive(Resource, Clone, Debug)]
pub struct SpriteDemoConfig {
    /// Asset-relative path of the image to display.
    pub sprite_path: String,
}

impl Default for SpriteDemoConfig {
    fn default() -> Self {
        Self {
            sprite_path: DEFAULT_SPRITE_PATH.to_string(),
        }
    }
}

/// Spawns a camera and a single sprite loaded from disk.
///
/// This system reads [`AssetServer`], so it must not run under a headless
/// `MinimalPlugins` app (there is no `AssetServer` there and it would panic).
fn setup(mut commands: Commands, asset_server: Res<AssetServer>, config: Res<SpriteDemoConfig>) {
    commands.spawn(Camera2d);
    commands.spawn(Sprite {
        image: asset_server.load(config.sprite_path.clone()),
        ..default()
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn config_default_uses_default_path() {
        assert_eq!(SpriteDemoConfig::default().sprite_path, DEFAULT_SPRITE_PATH);
    }

    #[test]
    fn default_sprite_path_is_png() {
        assert_eq!(DEFAULT_SPRITE_PATH, "sprite.png");
    }

    #[test]
    fn config_override_is_preserved() {
        let cfg = SpriteDemoConfig {
            sprite_path: "hero.png".to_string(),
        };
        assert_eq!(cfg.sprite_path, "hero.png");
    }
}
