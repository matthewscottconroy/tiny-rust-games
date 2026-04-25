//! Knockback & Hit-Stop — game-feel mechanics that make combat physically satisfying.
//!
//! Key ideas:
//! - On a successful attack, the target receives a velocity impulse directed away
//!   from the attacker (`knockback_dir` is a pure Vec2 function).
//! - Velocity decays each frame with exponential-style friction (`decay_vel`).
//! - Hit-stop: the game briefly slows enemy physics to ~5 % speed for a fraction of
//!   a second, creating a visceral pause on impact. Implemented as a `HitStop`
//!   resource that scales `effective_dt` — no Bevy time manipulation needed.
//! - Screen shake on the player camera for one second after each hit.
//!
//! **Controls:** WASD / Arrows — move   SPACE — attack nearby enemies

use bevy::prelude::*;

const WINDOW_W: f32 = 800.0;
const WINDOW_H: f32 = 500.0;
const PLAYER_SPEED: f32 = 160.0;
const ATTACK_RANGE: f32 = 65.0;
const KNOCKBACK_FORCE: f32 = 400.0;
const VEL_DECAY: f32 = 320.0;
const HITSTOP_DUR: f32 = 0.12;
const HITSTOP_SCALE: f32 = 0.05;
const ATTACK_CD: f32 = 0.35;

// ── Pure helpers ──────────────────────────────────────────────────────────────

/// Direction of knockback impulse away from `attacker` toward `target`.
pub fn knockback_dir(attacker: Vec2, target: Vec2) -> Vec2 {
    (target - attacker).normalize_or_zero()
}

/// Reduce velocity magnitude by `decay` units per second; clamp to zero.
pub fn decay_vel(vel: Vec2, decay: f32, dt: f32) -> Vec2 {
    let speed = (vel.length() - decay * dt).max(0.0);
    vel.normalize_or_zero() * speed
}

// ── ECS ───────────────────────────────────────────────────────────────────────

#[derive(Component)]
struct Player { attack_cd: f32 }

#[derive(Component)]
struct Enemy { vel: Vec2, wander_timer: f32, hp: i32 }

#[derive(Component)]
struct ShakeCamera { trauma: f32 }

#[derive(Resource, Default)]
struct HitStop { remaining: f32 }

#[derive(Component)]
struct HudText;

fn main() {
    App::new()
        .add_plugins(DefaultPlugins.set(WindowPlugin {
            primary_window: Some(Window {
                title: "Knockback & Hit-Stop".into(),
                resolution: (800u32, 500u32).into(),
                ..default()
            }),
            ..default()
        }))
        .insert_resource(HitStop::default())
        .add_systems(Startup, setup)
        .add_systems(Update, (
            move_player,
            wander_enemies,
            handle_attack,
            apply_knockback,
            tick_hitstop,
            shake_camera,
            update_hud,
        ).chain())
        .run();
}

fn setup(mut commands: Commands) {
    commands.spawn((
        ShakeCamera { trauma: 0.0 },
        Camera2d,
    ));

    commands.spawn((
        Player { attack_cd: 0.0 },
        Sprite { color: Color::srgb(0.3, 0.6, 1.0), custom_size: Some(Vec2::splat(22.0)), ..default() },
        Transform::from_translation(Vec3::new(0.0, 0.0, 1.0)),
    ));

    let positions = [(-200.0f32, 80.0), (180.0, -60.0), (50.0, 160.0)];
    for (x, y) in positions {
        commands.spawn((
            Enemy { vel: Vec2::ZERO, wander_timer: 0.0, hp: 5 },
            Sprite { color: Color::srgb(0.85, 0.25, 0.25), custom_size: Some(Vec2::splat(26.0)), ..default() },
            Transform::from_translation(Vec3::new(x, y, 1.0)),
        ));
    }

    commands.spawn((
        HudText,
        Text::new(""),
        TextFont { font_size: 16.0, ..default() },
        TextColor(Color::WHITE),
        Node { position_type: PositionType::Absolute, top: Val::Px(10.0), left: Val::Px(10.0), ..default() },
    ));

    commands.spawn((
        Text::new("WASD / Arrows — move   SPACE — attack"),
        TextFont { font_size: 13.0, ..default() },
        TextColor(Color::srgba(1.0, 1.0, 1.0, 0.5)),
        Node { position_type: PositionType::Absolute, bottom: Val::Px(10.0), left: Val::Px(10.0), ..default() },
    ));
}

fn move_player(
    keys: Res<ButtonInput<KeyCode>>,
    time: Res<Time>,
    mut q: Query<(&mut Player, &mut Transform)>,
) {
    let Ok((mut player, mut tf)) = q.single_mut() else { return };
    player.attack_cd = (player.attack_cd - time.delta_secs()).max(0.0);
    let mut dir = Vec2::ZERO;
    if keys.pressed(KeyCode::KeyW) || keys.pressed(KeyCode::ArrowUp)    { dir.y += 1.0; }
    if keys.pressed(KeyCode::KeyS) || keys.pressed(KeyCode::ArrowDown)  { dir.y -= 1.0; }
    if keys.pressed(KeyCode::KeyA) || keys.pressed(KeyCode::ArrowLeft)  { dir.x -= 1.0; }
    if keys.pressed(KeyCode::KeyD) || keys.pressed(KeyCode::ArrowRight) { dir.x += 1.0; }
    if dir == Vec2::ZERO { return; }
    tf.translation += (dir.normalize() * PLAYER_SPEED * time.delta_secs()).extend(0.0);
    tf.translation.x = tf.translation.x.clamp(-WINDOW_W / 2.0 + 14.0, WINDOW_W / 2.0 - 14.0);
    tf.translation.y = tf.translation.y.clamp(-WINDOW_H / 2.0 + 14.0, WINDOW_H / 2.0 - 14.0);
}

fn wander_enemies(time: Res<Time>, hitstop: Res<HitStop>, mut q: Query<(&mut Enemy, &mut Transform)>) {
    let scale = if hitstop.remaining > 0.0 { HITSTOP_SCALE } else { 1.0 };
    let dt = time.delta_secs() * scale;
    let hw = WINDOW_W / 2.0 - 16.0;
    let hh = WINDOW_H / 2.0 - 16.0;
    for (mut enemy, mut tf) in &mut q {
        if enemy.hp <= 0 { continue; }
        enemy.wander_timer -= dt;
        if enemy.wander_timer <= 0.0 {
            let angle = tf.translation.x.sin() * 12.34 + tf.translation.y.cos() * 7.89;
            enemy.vel = Vec2::from_angle(angle) * 55.0;
            enemy.wander_timer = 1.2 + (tf.translation.y.abs() % 0.8);
        }
        enemy.vel = decay_vel(enemy.vel, VEL_DECAY, dt);
        tf.translation += (enemy.vel * dt).extend(0.0);
        if tf.translation.x.abs() > hw { enemy.vel.x *= -1.0; tf.translation.x = tf.translation.x.clamp(-hw, hw); }
        if tf.translation.y.abs() > hh { enemy.vel.y *= -1.0; tf.translation.y = tf.translation.y.clamp(-hh, hh); }
    }
}

fn handle_attack(
    keys: Res<ButtonInput<KeyCode>>,
    mut player_q: Query<(&mut Player, &Transform)>,
    enemy_q: Query<(Entity, &Transform), With<Enemy>>,
    mut hitstop: ResMut<HitStop>,
    mut commands: Commands,
    mut shake_q: Query<&mut ShakeCamera>,
) {
    let Ok((mut player, ptf)) = player_q.single_mut() else { return };
    if !keys.just_pressed(KeyCode::Space) || player.attack_cd > 0.0 { return; }

    let player_pos = ptf.translation.truncate();
    let mut hit = false;
    for (entity, etf) in &enemy_q {
        if ptf.translation.distance(etf.translation) <= ATTACK_RANGE {
            let dir = knockback_dir(player_pos, etf.translation.truncate());
            commands.entity(entity).insert(KnockbackImpulse(dir * KNOCKBACK_FORCE));
            hit = true;
        }
    }
    if hit {
        player.attack_cd = ATTACK_CD;
        hitstop.remaining = HITSTOP_DUR;
        if let Ok(mut cam) = shake_q.single_mut() { cam.trauma = 1.0; }
    }
}

#[derive(Component)]
struct KnockbackImpulse(Vec2);

fn apply_knockback(
    mut commands: Commands,
    hitstop: Res<HitStop>,
    time: Res<Time>,
    mut q: Query<(Entity, &mut Enemy, &mut Sprite, Option<&KnockbackImpulse>)>,
) {
    let scale = if hitstop.remaining > 0.0 { HITSTOP_SCALE } else { 1.0 };
    let dt = time.delta_secs() * scale;
    for (entity, mut enemy, mut sprite, impulse) in &mut q {
        if let Some(imp) = impulse {
            enemy.vel = imp.0;
            enemy.hp -= 1;
            commands.entity(entity).remove::<KnockbackImpulse>();
        }
        sprite.color = if enemy.hp <= 0 {
            Color::srgb(0.3, 0.3, 0.3)
        } else if hitstop.remaining > 0.0 {
            Color::WHITE
        } else {
            Color::srgb(0.85, 0.25, 0.25)
        };
        let _ = dt; // velocity decay applied in wander_enemies
    }
}

fn tick_hitstop(time: Res<Time>, mut hitstop: ResMut<HitStop>) {
    hitstop.remaining = (hitstop.remaining - time.delta_secs()).max(0.0);
}

fn shake_camera(
    time: Res<Time>,
    mut q: Query<(&mut ShakeCamera, &mut Transform), With<Camera2d>>,
) {
    let Ok((mut cam, mut tf)) = q.single_mut() else { return };
    cam.trauma = (cam.trauma - time.delta_secs() * 2.0).max(0.0);
    let shake = cam.trauma * cam.trauma;
    let t = time.elapsed_secs();
    tf.translation.x = (t * 43.0).sin() * shake * 12.0;
    tf.translation.y = (t * 57.0).cos() * shake * 12.0;
}

fn update_hud(
    hitstop: Res<HitStop>,
    enemy_q: Query<&Enemy>,
    mut text_q: Query<&mut Text, With<HudText>>,
) {
    let Ok(mut text) = text_q.single_mut() else { return };
    let alive = enemy_q.iter().filter(|e| e.hp > 0).count();
    let ts = if hitstop.remaining > 0.0 { format!("{:.0}%", HITSTOP_SCALE * 100.0) } else { "100%".to_string() };
    text.0 = format!("Time scale: {ts}  |  Enemies alive: {alive}/3  |  SPACE to attack");
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn knockback_dir_away_from_attacker() {
        let dir = knockback_dir(Vec2::ZERO, Vec2::new(1.0, 0.0));
        assert!((dir - Vec2::new(1.0, 0.0)).length() < 1e-5);
    }

    #[test]
    fn knockback_dir_normalised() {
        let dir = knockback_dir(Vec2::ZERO, Vec2::new(3.0, 4.0));
        assert!((dir.length() - 1.0).abs() < 1e-5);
    }

    #[test]
    fn knockback_dir_same_position_returns_zero() {
        let dir = knockback_dir(Vec2::ZERO, Vec2::ZERO);
        assert_eq!(dir, Vec2::ZERO);
    }

    #[test]
    fn decay_vel_reduces_speed() {
        let vel = Vec2::new(100.0, 0.0);
        let decayed = decay_vel(vel, 50.0, 1.0);
        assert!((decayed.length() - 50.0).abs() < 1e-4);
    }

    #[test]
    fn decay_vel_clamps_to_zero() {
        let vel = Vec2::new(10.0, 0.0);
        let decayed = decay_vel(vel, 100.0, 1.0);
        assert_eq!(decayed, Vec2::ZERO);
    }
}
