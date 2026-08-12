//! Notification system — a reusable Bevy plugin for timed UI toasts.
//!
//! This crate is a *building block*: drop [`NotificationSystemPlugin`] into any
//! Bevy app with `app.add_plugins(NotificationSystemPlugin)` and any system can
//! queue a toast by writing a [`NotifyMessage`]. Toasts fade out and despawn
//! automatically, and survivors restack every frame.
//!
//! Key ideas:
//! - Any system sends a [`NotifyMessage`] to queue a new toast.
//! - Each notification is an ECS entity tagged [`Notification`]; it is
//!   despawned automatically when its elapsed time exceeds its lifetime.
//! - [`notification_alpha`] computes the opacity, fading from 1 to 0 over the
//!   last `fade_secs` seconds of the lifetime.
//! - [`layout_notifications`] repositions surviving toasts in a vertical stack
//!   (oldest at the bottom, newest at the top) every frame.
//! - Tunables live in [`NotificationConfig`]; override before adding the plugin.
//!
//! **Controls:**
//! - **1** — info toast   **2** — warning toast   **3** — error toast
//! - **SPACE** — generic notification
//!
//! # Example
//! ```no_run
//! use bevy::prelude::*;
//! use notification_system::NotificationSystemPlugin;
//!
//! App::new()
//!     .add_plugins(DefaultPlugins)
//!     .add_plugins(NotificationSystemPlugin)
//!     .run();
//! ```

use bevy::prelude::*;

/// Bundles every system, message, and resource for the notification feature.
///
/// Add it with `app.add_plugins(NotificationSystemPlugin)`. It does **not** add
/// `DefaultPlugins`, so the host app stays in control of window and rendering
/// setup.
pub struct NotificationSystemPlugin;

impl Plugin for NotificationSystemPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<NotificationConfig>()
            .add_message::<NotifyMessage>()
            .add_systems(Startup, setup)
            .add_systems(
                Update,
                (
                    handle_input,
                    spawn_toasts,
                    tick_notifications,
                    layout_notifications,
                )
                    .chain(),
            );
    }
}

// ── Configuration ─────────────────────────────────────────────────────────────

/// Tunable parameters for the feature. Override before adding the plugin, e.g.
/// `app.insert_resource(NotificationConfig { lifetime: 5.0, ..default() })`.
#[derive(Resource, Clone, Copy, Debug)]
pub struct NotificationConfig {
    /// How long a toast lives, in seconds, before it despawns.
    pub lifetime: f32,
    /// Length of the fade-out window at the end of a toast's life, in seconds.
    pub fade_secs: f32,
    /// Toast width in pixels.
    pub notif_w: f32,
    /// Toast height in pixels.
    pub notif_h: f32,
    /// Vertical gap between stacked toasts, in pixels.
    pub stack_gap: f32,
    /// Distance from the bottom of the screen to the first toast, in pixels.
    pub base_y: f32,
}

impl Default for NotificationConfig {
    fn default() -> Self {
        Self {
            lifetime: 3.5,
            fade_secs: 0.8,
            notif_w: 340.0,
            notif_h: 42.0,
            stack_gap: 6.0,
            base_y: 48.0,
        }
    }
}

// ── Pure helpers ──────────────────────────────────────────────────────────────

/// Returns the opacity `[0, 1]` for a notification at time `elapsed`.
///
/// Full opacity until `lifetime − fade_secs`, then linearly fades to zero.
pub fn notification_alpha(elapsed: f32, lifetime: f32, fade_secs: f32) -> f32 {
    let fade_start = lifetime - fade_secs;
    if elapsed < fade_start {
        1.0
    } else {
        ((lifetime - elapsed) / fade_secs).clamp(0.0, 1.0)
    }
}

/// Returns how far through the lifetime `elapsed` is, clamped to `[0, 1]`.
pub fn elapsed_fraction(elapsed: f32, lifetime: f32) -> f32 {
    (elapsed / lifetime).clamp(0.0, 1.0)
}

// ── Messages ──────────────────────────────────────────────────────────────────

/// Fires to request a new toast notification with the given text.
#[derive(Message)]
pub struct NotifyMessage {
    /// Text to display in the toast.
    pub text: String,
    /// Text color of the toast.
    pub color: Color,
}

// ── Components ────────────────────────────────────────────────────────────────

/// Tracks a notification's lifetime state.
#[derive(Component)]
pub struct Notification {
    /// Seconds elapsed since the notification was spawned.
    pub elapsed: f32,
    /// Total lifetime of the notification, in seconds.
    pub lifetime: f32,
}

/// Marks the text child inside a notification background node.
#[derive(Component)]
pub struct NotificationText;

// ── Setup ─────────────────────────────────────────────────────────────────────

/// Spawns the camera and the key-binding hint label.
fn setup(mut commands: Commands) {
    commands.spawn(Camera2d);

    commands.spawn((
        Text::new("1 = info   2 = warning   3 = error   SPACE = generic"),
        TextFont {
            font_size: 15.0,
            ..default()
        },
        TextColor(Color::srgb(0.55, 0.55, 0.55)),
        Node {
            position_type: PositionType::Absolute,
            top: Val::Px(12.0),
            left: Val::Px(12.0),
            ..default()
        },
    ));
}

/// Writes [`NotifyMessage`] events based on key input.
fn handle_input(input: Res<ButtonInput<KeyCode>>, mut writer: MessageWriter<NotifyMessage>) {
    if input.just_pressed(KeyCode::Digit1) {
        writer.write(NotifyMessage {
            text: "INFO: Player joined the session".into(),
            color: Color::srgb(0.5, 0.85, 1.0),
        });
    }
    if input.just_pressed(KeyCode::Digit2) {
        writer.write(NotifyMessage {
            text: "WARN: Health is critically low!".into(),
            color: Color::srgb(1.0, 0.75, 0.2),
        });
    }
    if input.just_pressed(KeyCode::Digit3) {
        writer.write(NotifyMessage {
            text: "ERROR: Connection to server lost".into(),
            color: Color::srgb(1.0, 0.35, 0.35),
        });
    }
    if input.just_pressed(KeyCode::Space) {
        writer.write(NotifyMessage {
            text: "Notification triggered".into(),
            color: Color::WHITE,
        });
    }
}

/// Reads pending [`NotifyMessage`]s and spawns notification UI entities.
fn spawn_toasts(
    mut commands: Commands,
    mut reader: MessageReader<NotifyMessage>,
    config: Res<NotificationConfig>,
) {
    for msg in reader.read() {
        commands
            .spawn((
                Node {
                    position_type: PositionType::Absolute,
                    width: Val::Px(config.notif_w),
                    height: Val::Px(config.notif_h),
                    padding: UiRect::all(Val::Px(10.0)),
                    align_items: AlignItems::Center,
                    ..default()
                },
                BackgroundColor(Color::srgba(0.1, 0.1, 0.15, 0.92)),
                Notification {
                    elapsed: 0.0,
                    lifetime: config.lifetime,
                },
            ))
            .with_children(|parent| {
                parent.spawn((
                    Text::new(msg.text.clone()),
                    TextFont {
                        font_size: 14.0,
                        ..default()
                    },
                    TextColor(msg.color),
                    NotificationText,
                ));
            });
    }
}

/// Advances each notification's timer, fades its background, and despawns expired ones.
fn tick_notifications(
    time: Res<Time>,
    mut commands: Commands,
    config: Res<NotificationConfig>,
    mut query: Query<(Entity, &mut Notification, &mut BackgroundColor, &Children)>,
    mut text_colors: Query<&mut TextColor, With<NotificationText>>,
) {
    for (entity, mut notif, mut bg, children) in &mut query {
        notif.elapsed += time.delta_secs();
        let alpha = notification_alpha(notif.elapsed, notif.lifetime, config.fade_secs);
        bg.0 = Color::srgba(0.1, 0.1, 0.15, alpha * 0.92);
        for &child in children {
            if let Ok(mut tc) = text_colors.get_mut(child) {
                let Color::Srgba(s) = tc.0 else { continue };
                tc.0 = Color::srgba(s.red, s.green, s.blue, alpha);
            }
        }
        if notif.elapsed >= notif.lifetime {
            commands.entity(entity).despawn();
        }
    }
}

/// Repositions surviving toasts into a vertical stack (oldest first at bottom).
fn layout_notifications(
    config: Res<NotificationConfig>,
    mut query: Query<(Entity, &Notification, &mut Node)>,
) {
    let mut order: Vec<(Entity, f32)> = query
        .iter()
        .map(|(e, notif, _)| (e, notif.elapsed))
        .collect();
    order.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap()); // oldest at bottom

    for (i, (entity, _)) in order.iter().enumerate() {
        if let Ok((_, _, mut node)) = query.get_mut(*entity) {
            node.bottom = Val::Px(config.base_y + i as f32 * (config.notif_h + config.stack_gap));
            node.right = Val::Px(16.0);
        }
    }
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn notification_alpha_full_before_fade_window() {
        assert_eq!(notification_alpha(0.0, 3.5, 0.8), 1.0);
        assert_eq!(notification_alpha(2.0, 3.5, 0.8), 1.0);
    }

    #[test]
    fn notification_alpha_zero_at_lifetime_end() {
        assert!((notification_alpha(3.5, 3.5, 0.8) - 0.0).abs() < 1e-5);
    }

    #[test]
    fn notification_alpha_half_at_fade_midpoint() {
        // fade_start = 2.7, midpoint = 2.7 + 0.4 = 3.1
        let a = notification_alpha(3.1, 3.5, 0.8);
        assert!((a - 0.5).abs() < 1e-5);
    }

    #[test]
    fn notification_alpha_clamps_below_zero() {
        // elapsed > lifetime should not go negative
        assert_eq!(notification_alpha(5.0, 3.5, 0.8), 0.0);
    }

    #[test]
    fn elapsed_fraction_at_start_is_zero() {
        assert_eq!(elapsed_fraction(0.0, 3.5), 0.0);
    }

    #[test]
    fn elapsed_fraction_at_end_is_one() {
        assert_eq!(elapsed_fraction(3.5, 3.5), 1.0);
    }

    #[test]
    fn elapsed_fraction_halfway() {
        assert!((elapsed_fraction(1.75, 3.5) - 0.5).abs() < 1e-5);
    }

    #[test]
    fn elapsed_fraction_clamps_over_lifetime() {
        assert_eq!(elapsed_fraction(10.0, 3.5), 1.0);
    }

    #[test]
    fn config_default_is_valid() {
        let c = NotificationConfig::default();
        assert!(c.lifetime > 0.0);
        assert!(c.fade_secs > 0.0);
        assert!(c.fade_secs < c.lifetime);
        assert!(c.notif_h > 0.0);
        assert!(c.stack_gap >= 0.0);
    }

    // --- ECS setup test ---

    #[test]
    fn spawning_a_notify_message_creates_a_toast() {
        let mut app = App::new();
        app.add_plugins((MinimalPlugins, NotificationSystemPlugin));
        // handle_input reads keyboard input, absent under MinimalPlugins.
        app.insert_resource(ButtonInput::<KeyCode>::default());
        // First frame runs Startup + one Update pass.
        app.update();

        app.world_mut().write_message(NotifyMessage {
            text: "hello".into(),
            color: Color::WHITE,
        });
        app.update();

        let mut q = app.world_mut().query::<&Notification>();
        assert_eq!(q.iter(app.world()).count(), 1);
    }
}
