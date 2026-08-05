//! Behavior Tree — a reusable Bevy plugin for composable AI decisions.
//!
//! This crate is a *building block*: add [`BehaviorTreePlugin`] to any Bevy app
//! and it spawns a guard that runs a behavior tree every frame. The tree drives
//! patrol, chase, and attack based on the player's distance. Move close to watch
//! the guard react; back away to see it return to patrol.
//!
//! Key ideas:
//! - The tree is plain data ([`BtNode`]) built by [`build_guard_bt`] and ticked
//!   by the pure [`tick_node`] / [`eval_leaf`] functions — fully unit-testable
//!   without a World.
//! - [`NodeStatus`] and [`LeafKind`] form the node contract; `Sequence` and
//!   `Selector` compose leaves into decisions.
//! - Sensing thresholds live in [`SIGHT_RANGE`] / [`ATTACK_RANGE`]; movement and
//!   window tunables live in [`BehaviorTreeConfig`].
//!
//! **Controls:** WASD / Arrow keys — move the player.
//!
//! # Example
//! ```no_run
//! use bevy::prelude::*;
//! use behavior_tree::BehaviorTreePlugin;
//!
//! App::new()
//!     .add_plugins(DefaultPlugins)
//!     .add_plugins(BehaviorTreePlugin)
//!     .run();
//! ```

use bevy::prelude::*;

/// Bundles the behavior-tree demo's setup and per-frame systems.
///
/// Add it with `app.add_plugins(BehaviorTreePlugin)`. It does **not** add
/// `DefaultPlugins`, so the host app owns window and rendering setup.
pub struct BehaviorTreePlugin;

impl Plugin for BehaviorTreePlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<BehaviorTreeConfig>()
            .add_systems(Startup, setup)
            .add_systems(Update, (move_player, tick_guard, sync_disc, refresh_label).chain());
    }
}

// ── Sensing thresholds ──────────────────────────────────────────────────────

/// Distance within which the guard can see the player.
pub const SIGHT_RANGE: f32 = 190.0;
/// Distance within which the guard is considered adjacent (can attack).
pub const ATTACK_RANGE: f32 = 44.0;

// ── Config ──────────────────────────────────────────────────────────────────

/// Tunable movement and layout parameters. Override before adding the plugin,
/// e.g. `app.insert_resource(BehaviorTreeConfig { guard_speed: 120.0, ..default() })`.
#[derive(Resource, Clone, Copy, Debug)]
pub struct BehaviorTreeConfig {
    /// Window width, used to clamp the player inside the view.
    pub window_w: f32,
    /// Window height, used to clamp the player inside the view.
    pub window_h: f32,
    /// Player movement speed (pixels/second).
    pub player_speed: f32,
    /// Guard movement speed (pixels/second).
    pub guard_speed: f32,
    /// Radius of the guard's circular patrol path.
    pub patrol_radius: f32,
}

impl Default for BehaviorTreeConfig {
    fn default() -> Self {
        Self {
            window_w: 800.0,
            window_h: 500.0,
            player_speed: 160.0,
            guard_speed: 90.0,
            patrol_radius: 110.0,
        }
    }
}

// ── BT primitives ─────────────────────────────────────────────────────────────

/// Result of ticking one BT node.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum NodeStatus { Running, Success, Failure }

/// The leaf actions and conditions the guard can execute or test.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum LeafKind { CanSeePlayer, IsNearPlayer, Chase, Attack, Patrol }

/// A node in the behavior tree.
#[derive(Clone, Debug)]
pub enum BtNode {
    Sequence(Vec<BtNode>),
    Selector(Vec<BtNode>),
    Leaf(LeafKind),
}

/// Shared read-only snapshot passed to every node tick.
pub struct BtCtx {
    pub guard_pos: Vec2,
    pub player_pos: Vec2,
}

/// Evaluate a single leaf against the current context.
pub fn eval_leaf(kind: LeafKind, ctx: &BtCtx) -> NodeStatus {
    let dist = ctx.guard_pos.distance(ctx.player_pos);
    match kind {
        LeafKind::CanSeePlayer => if dist <= SIGHT_RANGE { NodeStatus::Success } else { NodeStatus::Failure },
        LeafKind::IsNearPlayer => if dist <= ATTACK_RANGE { NodeStatus::Success } else { NodeStatus::Failure },
        LeafKind::Chase  => NodeStatus::Running,
        LeafKind::Attack => NodeStatus::Running,
        LeafKind::Patrol => NodeStatus::Running,
    }
}

/// Tick a node recursively. Returns status and the active leaf (if any).
pub fn tick_node(node: &BtNode, ctx: &BtCtx) -> (NodeStatus, Option<LeafKind>) {
    match node {
        BtNode::Sequence(children) => {
            for child in children {
                let (s, action) = tick_node(child, ctx);
                if s == NodeStatus::Failure { return (NodeStatus::Failure, None); }
                if s == NodeStatus::Running  { return (NodeStatus::Running, action); }
            }
            (NodeStatus::Success, None)
        }
        BtNode::Selector(children) => {
            for child in children {
                let (s, action) = tick_node(child, ctx);
                if s != NodeStatus::Failure { return (s, action); }
            }
            (NodeStatus::Failure, None)
        }
        BtNode::Leaf(kind) => {
            let s = eval_leaf(*kind, ctx);
            (s, if s != NodeStatus::Failure { Some(*kind) } else { None })
        }
    }
}

/// Selector [ Sequence[CanSee, Selector[Sequence[IsNear, Attack], Chase]], Patrol ]
pub fn build_guard_bt() -> BtNode {
    BtNode::Selector(vec![
        BtNode::Sequence(vec![
            BtNode::Leaf(LeafKind::CanSeePlayer),
            BtNode::Selector(vec![
                BtNode::Sequence(vec![
                    BtNode::Leaf(LeafKind::IsNearPlayer),
                    BtNode::Leaf(LeafKind::Attack),
                ]),
                BtNode::Leaf(LeafKind::Chase),
            ]),
        ]),
        BtNode::Leaf(LeafKind::Patrol),
    ])
}

// ── ECS components / resources ────────────────────────────────────────────────

/// Marker for the player-controlled sprite.
#[derive(Component)]
pub struct Player;

#[derive(Component)]
struct SightDisc;

/// The guard: carries its behavior tree, patrol state, and current action.
#[derive(Component)]
pub struct Guard {
    /// The guard's behavior tree.
    pub bt: BtNode,
    /// Center of the circular patrol path.
    pub patrol_origin: Vec2,
    /// Current patrol angle (radians).
    pub patrol_angle: f32,
    /// The leaf action chosen on the last tick.
    pub action: LeafKind,
}

#[derive(Component)]
struct StatusLabel;

// ── Systems ─────────────────────────────────────────────────────────────────

fn setup(mut commands: Commands, config: Res<BehaviorTreeConfig>) {
    commands.spawn(Camera2d);

    commands.spawn((
        Player,
        Sprite { color: Color::srgb(0.25, 0.60, 1.0), custom_size: Some(Vec2::splat(20.0)), ..default() },
        Transform::from_translation(Vec3::new(-240.0, 0.0, 1.0)),
    ));

    let origin = Vec2::new(120.0, 0.0);
    commands.spawn((
        Guard { bt: build_guard_bt(), patrol_origin: origin, patrol_angle: 0.0, action: LeafKind::Patrol },
        Sprite { color: Color::srgb(0.9, 0.3, 0.3), custom_size: Some(Vec2::splat(24.0)), ..default() },
        Transform::from_translation(Vec3::new(origin.x, origin.y, 1.0)),
    ));

    commands.spawn((
        SightDisc,
        Sprite { color: Color::srgba(1.0, 0.35, 0.35, 0.07), custom_size: Some(Vec2::splat(SIGHT_RANGE * 2.0)), ..default() },
        Transform::from_translation(Vec3::new(origin.x, origin.y, 0.0)),
    ));

    commands.spawn((
        StatusLabel,
        Text::new("Guard: Patrolling"),
        TextFont { font_size: 20.0, ..default() },
        TextColor(Color::WHITE),
        Node { position_type: PositionType::Absolute, top: Val::Px(12.0), left: Val::Px(12.0), ..default() },
    ));

    commands.spawn((
        Text::new("WASD / Arrows — move player   approach the guard to trigger its BT"),
        TextFont { font_size: 13.0, ..default() },
        TextColor(Color::srgba(1.0, 1.0, 1.0, 0.5)),
        Node { position_type: PositionType::Absolute, bottom: Val::Px(12.0), left: Val::Px(12.0), ..default() },
    ));
}

fn move_player(
    keys: Res<ButtonInput<KeyCode>>,
    time: Res<Time>,
    config: Res<BehaviorTreeConfig>,
    mut q: Query<&mut Transform, With<Player>>,
) {
    let mut dir = Vec2::ZERO;
    if keys.pressed(KeyCode::KeyW) || keys.pressed(KeyCode::ArrowUp)    { dir.y += 1.0; }
    if keys.pressed(KeyCode::KeyS) || keys.pressed(KeyCode::ArrowDown)  { dir.y -= 1.0; }
    if keys.pressed(KeyCode::KeyA) || keys.pressed(KeyCode::ArrowLeft)  { dir.x -= 1.0; }
    if keys.pressed(KeyCode::KeyD) || keys.pressed(KeyCode::ArrowRight) { dir.x += 1.0; }
    if dir == Vec2::ZERO { return; }
    let Ok(mut tf) = q.single_mut() else { return };
    tf.translation += (dir.normalize() * config.player_speed * time.delta_secs()).extend(0.0);
    tf.translation.x = tf.translation.x.clamp(-config.window_w / 2.0 + 14.0, config.window_w / 2.0 - 14.0);
    tf.translation.y = tf.translation.y.clamp(-config.window_h / 2.0 + 14.0, config.window_h / 2.0 - 14.0);
}

fn tick_guard(
    time: Res<Time>,
    config: Res<BehaviorTreeConfig>,
    player_q: Query<&Transform, With<Player>>,
    mut guard_q: Query<(&mut Guard, &mut Transform, &mut Sprite), (Without<Player>, Without<SightDisc>)>,
) {
    let Ok(ptf) = player_q.single() else { return };
    let Ok((mut guard, mut tf, mut sprite)) = guard_q.single_mut() else { return };
    let guard_pos = tf.translation.truncate();
    let player_pos = ptf.translation.truncate();
    let dt = time.delta_secs();

    let bt = guard.bt.clone();
    let (_, action) = tick_node(&bt, &BtCtx { guard_pos, player_pos });
    let active = action.unwrap_or(LeafKind::Patrol);
    guard.action = active;

    match active {
        LeafKind::Chase => {
            let dir = (player_pos - guard_pos).normalize_or_zero();
            tf.translation += (dir * config.guard_speed * dt).extend(0.0);
            sprite.color = Color::srgb(1.0, 0.55, 0.0);
        }
        LeafKind::Attack => {
            sprite.color = Color::srgb(1.0, 0.08, 0.08);
        }
        LeafKind::Patrol => {
            guard.patrol_angle += dt * 0.75;
            let target = guard.patrol_origin + Vec2::from_angle(guard.patrol_angle) * config.patrol_radius;
            let dir = (target - guard_pos).normalize_or_zero();
            tf.translation += (dir * config.guard_speed * 0.55 * dt).extend(0.0);
            sprite.color = Color::srgb(0.9, 0.3, 0.3);
        }
        _ => {}
    }
}

fn sync_disc(
    guard_q: Query<&Transform, (With<Guard>, Without<SightDisc>)>,
    mut disc_q: Query<&mut Transform, (With<SightDisc>, Without<Guard>)>,
) {
    let Ok(gtf) = guard_q.single() else { return };
    let Ok(mut dtf) = disc_q.single_mut() else { return };
    dtf.translation = Vec3::new(gtf.translation.x, gtf.translation.y, 0.0);
}

fn refresh_label(
    guard_q: Query<&Guard>,
    mut label_q: Query<&mut Text, With<StatusLabel>>,
) {
    let Ok(guard) = guard_q.single() else { return };
    let Ok(mut text) = label_q.single_mut() else { return };
    text.0 = match guard.action {
        LeafKind::Chase  => "Guard: CHASING",
        LeafKind::Attack => "Guard: ATTACKING",
        LeafKind::Patrol => "Guard: Patrolling",
        _                => "Guard: ...",
    }.to_string();
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    fn ctx(guard: Vec2, player: Vec2) -> BtCtx { BtCtx { guard_pos: guard, player_pos: player } }

    #[test]
    fn patrols_when_out_of_sight() {
        let bt = build_guard_bt();
        let (_, action) = tick_node(&bt, &ctx(Vec2::ZERO, Vec2::new(999.0, 0.0)));
        assert_eq!(action, Some(LeafKind::Patrol));
    }

    #[test]
    fn chases_when_visible_but_not_adjacent() {
        let bt = build_guard_bt();
        let (_, action) = tick_node(&bt, &ctx(Vec2::ZERO, Vec2::new(SIGHT_RANGE * 0.5, 0.0)));
        assert_eq!(action, Some(LeafKind::Chase));
    }

    #[test]
    fn attacks_when_adjacent() {
        let bt = build_guard_bt();
        let (_, action) = tick_node(&bt, &ctx(Vec2::ZERO, Vec2::new(ATTACK_RANGE * 0.4, 0.0)));
        assert_eq!(action, Some(LeafKind::Attack));
    }

    #[test]
    fn sequence_short_circuits_on_first_failure() {
        let node = BtNode::Sequence(vec![
            BtNode::Leaf(LeafKind::CanSeePlayer),
            BtNode::Leaf(LeafKind::Attack),
        ]);
        let (status, _) = tick_node(&node, &ctx(Vec2::ZERO, Vec2::new(999.0, 0.0)));
        assert_eq!(status, NodeStatus::Failure);
    }

    #[test]
    fn selector_returns_first_success() {
        let node = BtNode::Selector(vec![
            BtNode::Leaf(LeafKind::CanSeePlayer),
            BtNode::Leaf(LeafKind::Patrol),
        ]);
        let (_, action) = tick_node(&node, &ctx(Vec2::ZERO, Vec2::new(SIGHT_RANGE * 0.5, 0.0)));
        assert_eq!(action, Some(LeafKind::CanSeePlayer));
    }

    #[test]
    fn sight_boundary_exact() {
        let at_edge = ctx(Vec2::ZERO, Vec2::new(SIGHT_RANGE, 0.0));
        assert_eq!(eval_leaf(LeafKind::CanSeePlayer, &at_edge), NodeStatus::Success);
        let just_outside = ctx(Vec2::ZERO, Vec2::new(SIGHT_RANGE + 0.1, 0.0));
        assert_eq!(eval_leaf(LeafKind::CanSeePlayer, &just_outside), NodeStatus::Failure);
    }

    #[test]
    fn config_default_matches_documented_values() {
        let c = BehaviorTreeConfig::default();
        assert_eq!(c.player_speed, 160.0);
        assert_eq!(c.guard_speed, 90.0);
        assert_eq!(c.patrol_radius, 110.0);
    }
}
