//! Behavior Tree — composable AI decisions using Sequence, Selector, and Leaf nodes.
//!
//! A guard runs a behavior tree every frame. The tree drives patrol, chase, and attack
//! based on the player's distance. Move close to watch the guard react; back away to
//! see it return to patrol.
//!
//! **Controls:** WASD / Arrow keys — move the player.

use bevy::prelude::*;

const WINDOW_W: f32 = 800.0;
const WINDOW_H: f32 = 500.0;
const PLAYER_SPEED: f32 = 160.0;
const GUARD_SPEED: f32 = 90.0;
const SIGHT_RANGE: f32 = 190.0;
const ATTACK_RANGE: f32 = 44.0;
const PATROL_RADIUS: f32 = 110.0;

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

#[derive(Component)]
struct Player;

#[derive(Component)]
struct SightDisc;

#[derive(Component)]
struct Guard {
    bt: BtNode,
    patrol_origin: Vec2,
    patrol_angle: f32,
    action: LeafKind,
}

#[derive(Component)]
struct StatusLabel;

// ── App ───────────────────────────────────────────────────────────────────────

fn main() {
    App::new()
        .add_plugins(DefaultPlugins.set(WindowPlugin {
            primary_window: Some(Window {
                title: "Behavior Tree".into(),
                resolution: (800u32, 500u32).into(),
                ..default()
            }),
            ..default()
        }))
        .add_systems(Startup, setup)
        .add_systems(Update, (move_player, tick_guard, sync_disc, refresh_label).chain())
        .run();
}

fn setup(mut commands: Commands) {
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
    mut q: Query<&mut Transform, With<Player>>,
) {
    let mut dir = Vec2::ZERO;
    if keys.pressed(KeyCode::KeyW) || keys.pressed(KeyCode::ArrowUp)    { dir.y += 1.0; }
    if keys.pressed(KeyCode::KeyS) || keys.pressed(KeyCode::ArrowDown)  { dir.y -= 1.0; }
    if keys.pressed(KeyCode::KeyA) || keys.pressed(KeyCode::ArrowLeft)  { dir.x -= 1.0; }
    if keys.pressed(KeyCode::KeyD) || keys.pressed(KeyCode::ArrowRight) { dir.x += 1.0; }
    if dir == Vec2::ZERO { return; }
    let Ok(mut tf) = q.single_mut() else { return };
    tf.translation += (dir.normalize() * PLAYER_SPEED * time.delta_secs()).extend(0.0);
    tf.translation.x = tf.translation.x.clamp(-WINDOW_W / 2.0 + 14.0, WINDOW_W / 2.0 - 14.0);
    tf.translation.y = tf.translation.y.clamp(-WINDOW_H / 2.0 + 14.0, WINDOW_H / 2.0 - 14.0);
}

fn tick_guard(
    time: Res<Time>,
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
            tf.translation += (dir * GUARD_SPEED * dt).extend(0.0);
            sprite.color = Color::srgb(1.0, 0.55, 0.0);
        }
        LeafKind::Attack => {
            sprite.color = Color::srgb(1.0, 0.08, 0.08);
        }
        LeafKind::Patrol => {
            guard.patrol_angle += dt * 0.75;
            let target = guard.patrol_origin + Vec2::from_angle(guard.patrol_angle) * PATROL_RADIUS;
            let dir = (target - guard_pos).normalize_or_zero();
            tf.translation += (dir * GUARD_SPEED * 0.55 * dt).extend(0.0);
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
}
