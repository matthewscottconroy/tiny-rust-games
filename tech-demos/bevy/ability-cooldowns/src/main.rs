//! Ability Cooldowns — multiple player abilities with independent timers and charge bars.
//!
//! Key ideas:
//! - Each ability is a plain struct with a `cooldown_max` and `cooldown_remaining`.
//! - `is_ready`, `cooldown_fraction`, and `use_ability` are pure functions that
//!   require no Bevy types and can be tested in isolation.
//! - Three abilities (Dash, Shield, Nova) have different cooldown lengths.
//!   Each has a visual fill-bar that shows charge progress from 0 % to 100 %.
//! - Pressing a key while an ability is on cooldown does nothing; the bar gives
//!   clear feedback without any additional state.
//!
//! **Controls:** Q — Dash (1 s)   W — Shield (3 s)   E — Nova (8 s)

use bevy::prelude::*;

// ── Pure ability model ────────────────────────────────────────────────────────

#[derive(Clone, Debug)]
pub struct Ability {
    pub name: &'static str,
    pub cooldown_max: f32,
    pub cooldown_remaining: f32,
    pub color: Color,
    pub key_label: &'static str,
}

/// True when the ability has finished charging.
pub fn is_ready(a: &Ability) -> bool { a.cooldown_remaining <= 0.0 }

/// Charge fraction: 0.0 = just used, 1.0 = fully charged.
pub fn cooldown_fraction(a: &Ability) -> f32 {
    if a.cooldown_max <= 0.0 { return 1.0; }
    1.0 - (a.cooldown_remaining / a.cooldown_max).clamp(0.0, 1.0)
}

/// Advance the cooldown timer by `dt` seconds.
pub fn tick_ability(a: &mut Ability, dt: f32) {
    a.cooldown_remaining = (a.cooldown_remaining - dt).max(0.0);
}

/// Attempt to use the ability. Returns `true` and starts the cooldown if ready.
pub fn use_ability(a: &mut Ability) -> bool {
    if !is_ready(a) { return false; }
    a.cooldown_remaining = a.cooldown_max;
    true
}

// ── ECS ───────────────────────────────────────────────────────────────────────

#[derive(Resource)]
struct Abilities([Ability; 3]);

/// Marker on the ability UI slot entity (indexes match `Abilities`).
#[derive(Component)]
struct AbilitySlot(usize);

/// Inner fill bar for each slot.
#[derive(Component)]
struct FillBar(usize);

/// Floating label for ready/cooldown state.
#[derive(Component)]
struct CooldownLabel(usize);

/// Flash overlay shown when ability fires.
#[derive(Component)]
struct FlashOverlay { timer: f32, ability: usize }

/// Player sprite used for the ability flash effect.
#[derive(Component)]
struct Player;

fn main() {
    App::new()
        .add_plugins(DefaultPlugins.set(WindowPlugin {
            primary_window: Some(Window {
                title: "Ability Cooldowns".into(),
                resolution: (800u32, 500u32).into(),
                ..default()
            }),
            ..default()
        }))
        .insert_resource(Abilities([
            Ability { name: "Dash",   cooldown_max: 1.0, cooldown_remaining: 0.0, color: Color::srgb(0.3, 0.8, 1.0), key_label: "Q" },
            Ability { name: "Shield", cooldown_max: 3.0, cooldown_remaining: 0.0, color: Color::srgb(0.4, 1.0, 0.4), key_label: "W" },
            Ability { name: "Nova",   cooldown_max: 8.0, cooldown_remaining: 0.0, color: Color::srgb(1.0, 0.6, 0.2), key_label: "E" },
        ]))
        .add_systems(Startup, setup)
        .add_systems(Update, (tick_cooldowns, handle_input, update_ui, tick_flashes).chain())
        .run();
}

fn setup(mut commands: Commands, abilities: Res<Abilities>) {
    commands.spawn(Camera2d);

    // Player sprite centre stage.
    commands.spawn((
        Player,
        Sprite { color: Color::srgb(0.5, 0.5, 0.9), custom_size: Some(Vec2::splat(36.0)), ..default() },
        Transform::from_translation(Vec3::new(0.0, 60.0, 1.0)),
    ));

    // Three ability slot panels at the bottom.
    let slot_y = -180.0;
    let slot_w = 160.0;
    let slot_h = 80.0;
    let gap = 20.0;
    let total_w = 3.0 * slot_w + 2.0 * gap;
    let start_x = -total_w / 2.0 + slot_w / 2.0;

    for (i, ability) in abilities.0.iter().enumerate() {
        let x = start_x + i as f32 * (slot_w + gap);

        // Background panel.
        commands.spawn((
            AbilitySlot(i),
            Sprite { color: Color::srgb(0.15, 0.15, 0.18), custom_size: Some(Vec2::new(slot_w, slot_h)), ..default() },
            Transform::from_translation(Vec3::new(x, slot_y, 0.0)),
        ));

        // Fill bar (width driven by cooldown_fraction).
        commands.spawn((
            FillBar(i),
            Sprite { color: ability.color, custom_size: Some(Vec2::new(slot_w - 4.0, slot_h - 4.0)), ..default() },
            Transform::from_translation(Vec3::new(x, slot_y, 0.5)),
        ));
    }

    commands.spawn((
        Text::new("Q — Dash (1s)   W — Shield (3s)   E — Nova (8s)"),
        TextFont { font_size: 14.0, ..default() },
        TextColor(Color::srgba(1.0, 1.0, 1.0, 0.55)),
        Node { position_type: PositionType::Absolute, bottom: Val::Px(10.0), left: Val::Px(10.0), ..default() },
    ));
}

fn tick_cooldowns(time: Res<Time>, mut abilities: ResMut<Abilities>) {
    for a in &mut abilities.0 { tick_ability(a, time.delta_secs()); }
}

fn handle_input(
    keys: Res<ButtonInput<KeyCode>>,
    mut abilities: ResMut<Abilities>,
    mut commands: Commands,
    player_q: Query<Entity, With<Player>>,
) {
    let keys_map = [KeyCode::KeyQ, KeyCode::KeyW, KeyCode::KeyE];
    let Ok(player_e) = player_q.single() else { return };
    for (i, &key) in keys_map.iter().enumerate() {
        if keys.just_pressed(key) && use_ability(&mut abilities.0[i]) {
            commands.entity(player_e).insert(FlashOverlay { timer: 0.18, ability: i });
        }
    }
}

fn update_ui(
    abilities: Res<Abilities>,
    mut fill_q: Query<(&FillBar, &mut Sprite, &mut Transform)>,
) {
    let slot_w = 160.0;
    let slot_h = 80.0;
    let gap = 20.0;
    let total_w = 3.0 * slot_w + 2.0 * gap;
    let start_x = -total_w / 2.0 + slot_w / 2.0;
    let slot_y = -180.0;

    for (bar, mut sprite, mut tf) in &mut fill_q {
        let ability = &abilities.0[bar.0];
        let frac = cooldown_fraction(ability);
        let bar_w = (slot_w - 4.0) * frac;
        let x = start_x + bar.0 as f32 * (slot_w + gap);
        // Anchor fill bar to left edge of slot.
        tf.translation.x = x - (slot_w - 4.0) / 2.0 + bar_w / 2.0;
        tf.translation.y = slot_y;
        sprite.custom_size = Some(Vec2::new(bar_w.max(0.001), slot_h - 4.0));
        sprite.color = if is_ready(ability) {
            ability.color
        } else {
            Color::srgb(0.25, 0.25, 0.28)
        };
    }
}

fn tick_flashes(
    time: Res<Time>,
    mut commands: Commands,
    abilities: Res<Abilities>,
    mut q: Query<(Entity, &mut FlashOverlay, &mut Sprite), With<Player>>,
) {
    let Ok((entity, mut flash, mut sprite)) = q.single_mut() else { return };
    flash.timer -= time.delta_secs();
    if flash.timer <= 0.0 {
        sprite.color = Color::srgb(0.5, 0.5, 0.9);
        commands.entity(entity).remove::<FlashOverlay>();
    } else {
        sprite.color = abilities.0[flash.ability].color;
    }
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    fn ability(max: f32, remaining: f32) -> Ability {
        Ability { name: "Test", cooldown_max: max, cooldown_remaining: remaining, color: Color::WHITE, key_label: "X" }
    }

    #[test]
    fn ready_when_remaining_is_zero() {
        assert!(is_ready(&ability(3.0, 0.0)));
    }

    #[test]
    fn not_ready_during_cooldown() {
        assert!(!is_ready(&ability(3.0, 1.5)));
    }

    #[test]
    fn fraction_zero_when_just_used() {
        assert!((cooldown_fraction(&ability(3.0, 3.0)) - 0.0).abs() < 1e-5);
    }

    #[test]
    fn fraction_one_when_ready() {
        assert!((cooldown_fraction(&ability(3.0, 0.0)) - 1.0).abs() < 1e-5);
    }

    #[test]
    fn use_ability_fails_during_cooldown() {
        let mut a = ability(3.0, 1.5);
        assert!(!use_ability(&mut a));
        assert!((a.cooldown_remaining - 1.5).abs() < 1e-5);
    }

    #[test]
    fn use_ability_starts_cooldown_when_ready() {
        let mut a = ability(3.0, 0.0);
        assert!(use_ability(&mut a));
        assert!((a.cooldown_remaining - 3.0).abs() < 1e-5);
    }
}
