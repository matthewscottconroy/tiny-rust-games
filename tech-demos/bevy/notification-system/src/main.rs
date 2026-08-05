//! Notification system — timed UI toast messages spawned by game events.
//!
//! Key ideas:
//! - Any system sends a [`NotifyMessage`] to queue a new toast.
//! - Each notification is an ECS entity tagged [`Notification`]; it is
//!   despawned automatically when its elapsed time exceeds its lifetime.
//! - [`notification_alpha`] computes the opacity, fading from 1 to 0 over the
//!   last [`FADE_SECS`] seconds of the lifetime.
//! - [`layout_notifications`] repositions surviving toasts in a vertical stack
//!   (oldest at the bottom, newest at the top) every frame.
//!
//! **Controls:**
//! - **1** — info toast   **2** — warning toast   **3** — error toast
//! - **SPACE** — generic notification

use bevy::prelude::*;

// ── Constants ─────────────────────────────────────────────────────────────────

const LIFETIME: f32 = 3.5;
const FADE_SECS: f32 = 0.8;
const NOTIF_W: f32 = 340.0;
const NOTIF_H: f32 = 42.0;
const STACK_GAP: f32 = 6.0;
const BASE_Y: f32 = 48.0;

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
struct NotifyMessage {
    text: String,
    color: Color,
}

// ── Components ────────────────────────────────────────────────────────────────

/// Tracks a notification's lifetime state.
#[derive(Component)]
struct Notification {
    elapsed: f32,
    lifetime: f32,
}

/// Marks the text child inside a notification background node.
#[derive(Component)]
struct NotificationText;

// ── App ───────────────────────────────────────────────────────────────────────

fn main() {
    App::new()
        .add_plugins(DefaultPlugins.set(WindowPlugin {
            primary_window: Some(Window {
                title: "Notification System".into(),
                resolution: (700u32, 420u32).into(),
                ..default()
            }),
            ..default()
        }))
        .add_message::<NotifyMessage>()
        .add_systems(Startup, setup)
        .add_systems(
            Update,
            (handle_input, spawn_toasts, tick_notifications, layout_notifications).chain(),
        )
        .run();
}

/// Spawns the camera and the key-binding hint label.
fn setup(mut commands: Commands) {
    commands.spawn(Camera2d);

    commands.spawn((
        Text::new("1 = info   2 = warning   3 = error   SPACE = generic"),
        TextFont { font_size: 15.0, ..default() },
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
fn spawn_toasts(mut commands: Commands, mut reader: MessageReader<NotifyMessage>) {
    for msg in reader.read() {
        commands
            .spawn((
                Node {
                    position_type: PositionType::Absolute,
                    width: Val::Px(NOTIF_W),
                    height: Val::Px(NOTIF_H),
                    padding: UiRect::all(Val::Px(10.0)),
                    align_items: AlignItems::Center,
                    ..default()
                },
                BackgroundColor(Color::srgba(0.1, 0.1, 0.15, 0.92)),
                Notification { elapsed: 0.0, lifetime: LIFETIME },
            ))
            .with_children(|parent| {
                parent.spawn((
                    Text::new(msg.text.clone()),
                    TextFont { font_size: 14.0, ..default() },
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
    mut query: Query<(Entity, &mut Notification, &mut BackgroundColor, &Children)>,
    mut text_colors: Query<&mut TextColor, With<NotificationText>>,
) {
    for (entity, mut notif, mut bg, children) in &mut query {
        notif.elapsed += time.delta_secs();
        let alpha = notification_alpha(notif.elapsed, notif.lifetime, FADE_SECS);
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
fn layout_notifications(mut query: Query<(Entity, &Notification, &mut Node)>) {
    let mut order: Vec<(Entity, f32)> = query
        .iter()
        .map(|(e, notif, _)| (e, notif.elapsed))
        .collect();
    order.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap()); // oldest at bottom

    for (i, (entity, _)) in order.iter().enumerate() {
        if let Ok((_, _, mut node)) = query.get_mut(*entity) {
            node.bottom = Val::Px(BASE_Y + i as f32 * (NOTIF_H + STACK_GAP));
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
    fn constants_are_valid() {
        assert!(LIFETIME > 0.0);
        assert!(FADE_SECS > 0.0);
        assert!(FADE_SECS < LIFETIME);
        assert!(NOTIF_H > 0.0);
        assert!(STACK_GAP >= 0.0);
    }
}
