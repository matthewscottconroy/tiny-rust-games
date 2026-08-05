//! Object pooling — a reusable Bevy plugin that recycles bullet entities.
//!
//! This crate is a *building block*: drop [`ObjectPoolingPlugin`] into any Bevy
//! app with `app.add_plugins(ObjectPoolingPlugin)` and it manages a fixed pool
//! of bullets that are activated on fire and recycled when they leave the
//! screen, rather than spawned and despawned each shot.
//!
//! Key ideas:
//! - All bullets are spawned at startup as invisible/inactive entities.
//!   Shooting does *not* spawn a new entity; it activates the next available
//!   slot in the pool. Despawning is replaced by deactivation (hide and freeze
//!   the bullet).
//! - [`find_inactive`] is a pure function that scans a slice of booleans and
//!   returns the first `false` index, mirroring what the ECS system does.
//! - [`is_off_screen`] is a pure function that decides when to deactivate a
//!   bullet by comparing its position to the window half-extents.
//! - The HUD shows how many pool slots are currently active vs. the total pool
//!   size, making the recycling behaviour visible.
//! - Tunables live in [`PoolConfig`]; override before adding the plugin.
//!
//! **Controls:** SPACE or Left-Click — fire a bullet.
//!
//! # Example
//! ```no_run
//! use bevy::prelude::*;
//! use object_pooling::ObjectPoolingPlugin;
//!
//! App::new()
//!     .add_plugins(DefaultPlugins)
//!     .add_plugins(ObjectPoolingPlugin)
//!     .run();
//! ```

use bevy::prelude::*;

/// Bundles every system and resource for the object-pooling feature.
///
/// Add it with `app.add_plugins(ObjectPoolingPlugin)`. It does **not** add
/// `DefaultPlugins`, so the host app stays in control of window and rendering
/// setup.
pub struct ObjectPoolingPlugin;

impl Plugin for ObjectPoolingPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<PoolConfig>()
            .insert_resource(FireCooldown(0.0))
            .add_systems(Startup, setup)
            .add_systems(Update, (tick_cooldown, handle_fire, move_bullets, update_hud));
    }
}

// ─── Configuration ───────────────────────────────────────────────────────────

/// Tunable parameters for the feature. Override before adding the plugin, e.g.
/// `app.insert_resource(PoolConfig { pool_size: 64, ..default() })`.
#[derive(Resource, Clone, Copy, Debug)]
pub struct PoolConfig {
    /// Number of bullets pre-spawned into the pool at startup.
    pub pool_size: usize,
    /// Bullet travel speed in pixels per second.
    pub bullet_speed: f32,
    /// Window width used for off-screen recycling, in pixels.
    pub window_w: f32,
    /// Window height used for off-screen recycling, in pixels.
    pub window_h: f32,
    /// Minimum delay between shots, in seconds.
    pub fire_cooldown: f32,
}

impl Default for PoolConfig {
    fn default() -> Self {
        Self {
            pool_size: 30,
            bullet_speed: 500.0,
            window_w: 800.0,
            window_h: 500.0,
            fire_cooldown: 0.08,
        }
    }
}

// ─── Pure helpers ────────────────────────────────────────────────────────────

/// Returns the index of the first `false` entry in `slots`, or `None` if all
/// are `true` (pool exhausted).
pub fn find_inactive(slots: &[bool]) -> Option<usize> {
    slots.iter().position(|&active| !active)
}

/// Returns `true` when `pos` is outside the rectangular region
/// `[-half.x, half.x] × [-half.y, half.y]` (with a small margin).
pub fn is_off_screen(pos: Vec2, half: Vec2) -> bool {
    let margin = 20.0;
    pos.x < -half.x - margin
        || pos.x > half.x + margin
        || pos.y < -half.y - margin
        || pos.y > half.y + margin
}

// ─── Components & resources ──────────────────────────────────────────────────

/// Bullet pool slot: tracks logical state independent of visibility.
#[derive(Component)]
pub struct PooledBullet {
    /// Whether this slot is currently in flight.
    pub active: bool,
    /// Current velocity of the bullet.
    pub velocity: Vec2,
}

/// Pool entity list so systems can index into it by slot number.
#[derive(Resource)]
pub struct BulletPool(pub Vec<Entity>);

/// Cooldown timer: prevents firing every frame.
#[derive(Resource)]
pub struct FireCooldown(pub f32);

/// Player ship marker.
#[derive(Component)]
pub struct PlayerShip;

/// Distinguishes the HUD text nodes.
#[derive(Component)]
pub enum HudLabel {
    /// The "Pool: N / M active" counter.
    Pool,
    /// The static instructions line.
    Instructions,
}

// ─── Systems ─────────────────────────────────────────────────────────────────

fn setup(mut commands: Commands, config: Res<PoolConfig>) {
    commands.spawn(Camera2d);

    // Player ship (triangle approximated as a rectangle for simplicity)
    commands.spawn((
        Sprite { color: Color::srgb(0.3, 0.85, 1.0), custom_size: Some(Vec2::new(30.0, 18.0)), ..default() },
        Transform::from_translation(Vec3::new(0.0, -180.0, 1.0)),
        PlayerShip,
    ));

    // Pre-spawn bullet pool — all hidden, inactive.
    let mut pool = Vec::with_capacity(config.pool_size);
    for _ in 0..config.pool_size {
        let e = commands.spawn((
            Sprite {
                color: Color::srgb(1.0, 0.95, 0.3),
                custom_size: Some(Vec2::new(4.0, 12.0)),
                ..default()
            },
            Transform::from_translation(Vec3::new(9999.0, 9999.0, 0.0)),
            Visibility::Hidden,
            PooledBullet { active: false, velocity: Vec2::ZERO },
        )).id();
        pool.push(e);
    }
    commands.insert_resource(BulletPool(pool));

    // HUD
    commands.spawn((
        Text::new(format!("Pool: 0 / {} active", config.pool_size)),
        TextFont { font_size: 20.0, ..default() },
        TextColor(Color::WHITE),
        Node {
            position_type: PositionType::Absolute,
            top: Val::Px(10.0),
            left: Val::Px(10.0),
            ..default()
        },
        HudLabel::Pool,
    ));
    commands.spawn((
        Text::new("SPACE / Left-click — fire"),
        TextFont { font_size: 16.0, ..default() },
        TextColor(Color::srgb(0.7, 0.7, 0.7)),
        Node {
            position_type: PositionType::Absolute,
            bottom: Val::Px(8.0),
            left: Val::Px(8.0),
            ..default()
        },
        HudLabel::Instructions,
    ));

    // Starfield background
    let positions: &[(f32, f32)] = &[
        (-350.0, 200.0), (120.0, 180.0), (270.0, -100.0), (-200.0, 130.0),
        (50.0, 210.0), (-100.0, -190.0), (310.0, 90.0), (-280.0, -170.0),
        (160.0, -240.0), (-55.0, 160.0), (240.0, -150.0), (-180.0, 250.0),
    ];
    for &(x, y) in positions {
        commands.spawn((
            Sprite { color: Color::srgba(1.0, 1.0, 1.0, 0.4), custom_size: Some(Vec2::splat(2.0)), ..default() },
            Transform::from_translation(Vec3::new(x, y, 0.0)),
        ));
    }
}

fn tick_cooldown(time: Res<Time>, mut cd: ResMut<FireCooldown>) {
    cd.0 = (cd.0 - time.delta_secs()).max(0.0);
}

/// Fires a bullet from the pool when SPACE is pressed or mouse is clicked.
fn handle_fire(
    input: Res<ButtonInput<KeyCode>>,
    mouse: Res<ButtonInput<MouseButton>>,
    config: Res<PoolConfig>,
    mut cd: ResMut<FireCooldown>,
    pool: Res<BulletPool>,
    ship_q: Query<&Transform, With<PlayerShip>>,
    mut bullets: Query<(&mut PooledBullet, &mut Transform, &mut Visibility), Without<PlayerShip>>,
) {
    let fire = input.pressed(KeyCode::Space) || mouse.pressed(MouseButton::Left);
    if !fire || cd.0 > 0.0 {
        return;
    }
    let Ok(ship_t) = ship_q.single() else { return };

    // Collect active flags to find a free slot.
    let active_flags: Vec<bool> = pool.0.iter().filter_map(|&e| {
        bullets.get(e).ok().map(|(b, _, _)| b.active)
    }).collect();

    let Some(slot) = find_inactive(&active_flags) else { return };
    let entity = pool.0[slot];
    if let Ok((mut bullet, mut t, mut vis)) = bullets.get_mut(entity) {
        bullet.active = true;
        bullet.velocity = Vec2::new(0.0, config.bullet_speed);
        t.translation = ship_t.translation + Vec3::new(0.0, 14.0, 0.5);
        *vis = Visibility::Visible;
    }
    cd.0 = config.fire_cooldown;
}

/// Moves active bullets; deactivates those that leave the screen.
fn move_bullets(
    time: Res<Time>,
    config: Res<PoolConfig>,
    mut bullets: Query<(&mut PooledBullet, &mut Transform, &mut Visibility)>,
) {
    let half = Vec2::new(config.window_w / 2.0, config.window_h / 2.0);
    for (mut bullet, mut t, mut vis) in &mut bullets {
        if !bullet.active {
            continue;
        }
        t.translation += (bullet.velocity * time.delta_secs()).extend(0.0);
        if is_off_screen(t.translation.truncate(), half) {
            bullet.active = false;
            bullet.velocity = Vec2::ZERO;
            t.translation = Vec3::new(9999.0, 9999.0, 0.0);
            *vis = Visibility::Hidden;
        }
    }
}

fn update_hud(
    config: Res<PoolConfig>,
    bullets: Query<&PooledBullet>,
    mut hud_q: Query<(&mut Text, &HudLabel)>,
) {
    let active = bullets.iter().filter(|b| b.active).count();
    for (mut text, label) in &mut hud_q {
        if matches!(label, HudLabel::Pool) {
            text.0 = format!("Pool: {} / {} active", active, config.pool_size);
        }
    }
}

// ─── Tests ───────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn find_inactive_returns_first_false() {
        let slots = [true, true, false, true, false];
        assert_eq!(find_inactive(&slots), Some(2));
    }

    #[test]
    fn find_inactive_all_active_returns_none() {
        let slots = [true, true, true];
        assert_eq!(find_inactive(&slots), None);
    }

    #[test]
    fn find_inactive_empty_returns_none() {
        assert_eq!(find_inactive(&[]), None);
    }

    #[test]
    fn find_inactive_first_slot_free() {
        let slots = [false, true, true];
        assert_eq!(find_inactive(&slots), Some(0));
    }

    #[test]
    fn is_off_screen_inside_returns_false() {
        let half = Vec2::new(400.0, 250.0);
        assert!(!is_off_screen(Vec2::new(0.0, 0.0), half));
        assert!(!is_off_screen(Vec2::new(390.0, 240.0), half));
    }

    #[test]
    fn is_off_screen_outside_returns_true() {
        let half = Vec2::new(400.0, 250.0);
        assert!(is_off_screen(Vec2::new(500.0, 0.0), half));
        assert!(is_off_screen(Vec2::new(0.0, -400.0), half));
    }

    #[test]
    fn config_default_is_valid() {
        let c = PoolConfig::default();
        assert!(c.pool_size > 0);
        assert!(c.bullet_speed > 0.0);
        assert!(c.fire_cooldown >= 0.0);
    }

    // --- ECS setup test ---

    #[test]
    fn setup_prespawns_full_pool() {
        let mut app = App::new();
        app.add_plugins((MinimalPlugins, ObjectPoolingPlugin));
        // handle_fire reads keyboard + mouse input, absent under MinimalPlugins.
        app.insert_resource(ButtonInput::<KeyCode>::default());
        app.insert_resource(ButtonInput::<MouseButton>::default());
        app.update();

        let expected = app.world().resource::<PoolConfig>().pool_size;
        let mut q = app.world_mut().query::<&PooledBullet>();
        assert_eq!(q.iter(app.world()).count(), expected);
        // All slots start inactive.
        assert!(q.iter(app.world()).all(|b| !b.active));
    }
}
