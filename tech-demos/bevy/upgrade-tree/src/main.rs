//! Upgrade Tree — branching skill unlock with prerequisite dependencies.
//!
//! Key ideas:
//! - The tree is a flat `Vec<UpgradeNode>` where each node stores its prerequisite
//!   indices. No graph library is required.
//! - `can_unlock` is a pure function: given the current unlocked set and available
//!   points, it checks both prerequisites and cost without touching Bevy.
//! - Nodes are coloured: dark-grey (locked prereqs), yellow (available), green (unlocked).
//! - The player earns skill points with SPACE and spends them by pressing 1–9.
//!
//! **Controls:** SPACE — earn a skill point   1–9 — unlock the matching node

use bevy::prelude::*;
use std::collections::HashSet;

// ── Pure tree model ───────────────────────────────────────────────────────────

pub struct UpgradeNode {
    pub name: &'static str,
    pub desc: &'static str,
    pub cost: u32,
    pub prereqs: &'static [usize],
}

/// True when `idx` is unlockable: prereqs satisfied, cost affordable, not yet unlocked.
pub fn can_unlock(idx: usize, nodes: &[UpgradeNode], unlocked: &HashSet<usize>, points: u32) -> bool {
    if unlocked.contains(&idx) { return false; }
    let node = &nodes[idx];
    if node.cost > points { return false; }
    node.prereqs.iter().all(|p| unlocked.contains(p))
}

const NODES: &[UpgradeNode] = &[
    // Tier 0
    UpgradeNode { name: "Basics",       desc: "Starting node",          cost: 0, prereqs: &[] },
    // Tier 1
    UpgradeNode { name: "Quick Strike", desc: "Faster attacks",         cost: 1, prereqs: &[0] },
    UpgradeNode { name: "Power Strike", desc: "Heavier blows",          cost: 1, prereqs: &[0] },
    UpgradeNode { name: "Healing",      desc: "Restore HP",             cost: 1, prereqs: &[0] },
    // Tier 2
    UpgradeNode { name: "Flurry",       desc: "Multi-hit combo",        cost: 2, prereqs: &[1] },
    UpgradeNode { name: "Slam",         desc: "Area knockback",         cost: 2, prereqs: &[2] },
    UpgradeNode { name: "Regen",        desc: "Passive HP recovery",    cost: 2, prereqs: &[3] },
    // Tier 3
    UpgradeNode { name: "Whirlwind",    desc: "Spin attack (1+2)",      cost: 3, prereqs: &[4, 5] },
    UpgradeNode { name: "Recovery",     desc: "Instant revive once",    cost: 3, prereqs: &[6] },
];

// Layout positions (world coords) for each node index.
const NODE_POS: [(f32, f32); 9] = [
    (-320.0,    0.0),  // 0 Basics
    (-120.0,  120.0),  // 1 Quick Strike
    (-120.0,    0.0),  // 2 Power Strike
    (-120.0, -120.0),  // 3 Healing
    (  80.0,  120.0),  // 4 Flurry
    (  80.0,    0.0),  // 5 Slam
    (  80.0, -120.0),  // 6 Regen
    ( 280.0,   60.0),  // 7 Whirlwind
    ( 280.0,  -60.0),  // 8 Recovery
];

// ── ECS ───────────────────────────────────────────────────────────────────────

#[derive(Resource)]
struct TreeState { unlocked: HashSet<usize>, points: u32 }

/// Links a sprite to node index.
#[derive(Component)]
struct NodeSprite(usize);

#[derive(Component)]
struct PointsLabel;

fn main() {
    let mut unlocked = HashSet::new();
    unlocked.insert(0); // Basics always unlocked.

    App::new()
        .add_plugins(DefaultPlugins.set(WindowPlugin {
            primary_window: Some(Window {
                title: "Upgrade Tree".into(),
                resolution: (800u32, 500u32).into(),
                ..default()
            }),
            ..default()
        }))
        .insert_resource(TreeState { unlocked, points: 2 })
        .add_systems(Startup, setup)
        .add_systems(Update, (handle_input, refresh_nodes, update_points_label).chain())
        .run();
}

fn setup(mut commands: Commands) {
    commands.spawn(Camera2d);

    // Spawn a sprite for each node.
    for (i, node) in NODES.iter().enumerate() {
        let (x, y) = NODE_POS[i];
        commands.spawn((
            NodeSprite(i),
            Sprite { color: Color::srgb(0.2, 0.2, 0.22), custom_size: Some(Vec2::new(130.0, 52.0)), ..default() },
            Transform::from_translation(Vec3::new(x, y, 0.5)),
        ));
        // Node name label.
        commands.spawn((
            Text::new(format!("[{}] {}", i + 1, node.name)),
            TextFont { font_size: 13.0, ..default() },
            TextColor(Color::WHITE),
            Node {
                position_type: PositionType::Absolute,
                left: Val::Px(x + 400.0 - 60.0),
                top: Val::Px(250.0 - y - 14.0),
                ..default()
            },
        ));
        // Node desc / cost label.
        commands.spawn((
            Text::new(format!("{} ({}pt)", node.desc, node.cost)),
            TextFont { font_size: 10.0, ..default() },
            TextColor(Color::srgba(1.0, 1.0, 1.0, 0.6)),
            Node {
                position_type: PositionType::Absolute,
                left: Val::Px(x + 400.0 - 60.0),
                top: Val::Px(250.0 - y + 2.0),
                ..default()
            },
        ));
    }

    // Thin connector lines between prereq → child.
    let edges: &[(usize, usize)] = &[(0,1),(0,2),(0,3),(1,4),(2,5),(3,6),(4,7),(5,7),(6,8)];
    for &(from, to) in edges {
        let (fx, fy) = NODE_POS[from];
        let (tx, ty) = NODE_POS[to];
        let mid_x = (fx + tx) / 2.0;
        let mid_y = (fy + ty) / 2.0;
        let dx = tx - fx;
        let dy = ty - fy;
        let len = (dx * dx + dy * dy).sqrt();
        let angle = dy.atan2(dx);
        commands.spawn((
            Sprite { color: Color::srgba(1.0, 1.0, 1.0, 0.15), custom_size: Some(Vec2::new(len, 2.0)), ..default() },
            Transform::from_translation(Vec3::new(mid_x, mid_y, 0.1))
                .with_rotation(Quat::from_rotation_z(angle)),
        ));
    }

    commands.spawn((
        PointsLabel,
        Text::new("Skill points: 2"),
        TextFont { font_size: 18.0, ..default() },
        TextColor(Color::srgb(1.0, 0.85, 0.3)),
        Node { position_type: PositionType::Absolute, top: Val::Px(10.0), left: Val::Px(10.0), ..default() },
    ));

    commands.spawn((
        Text::new("SPACE — earn point   1–9 — unlock node   Yellow = available   Green = unlocked"),
        TextFont { font_size: 13.0, ..default() },
        TextColor(Color::srgba(1.0, 1.0, 1.0, 0.5)),
        Node { position_type: PositionType::Absolute, bottom: Val::Px(10.0), left: Val::Px(10.0), ..default() },
    ));
}

fn handle_input(keys: Res<ButtonInput<KeyCode>>, mut state: ResMut<TreeState>) {
    if keys.just_pressed(KeyCode::Space) { state.points += 1; }

    let digit_keys = [
        KeyCode::Digit1, KeyCode::Digit2, KeyCode::Digit3, KeyCode::Digit4,
        KeyCode::Digit5, KeyCode::Digit6, KeyCode::Digit7, KeyCode::Digit8,
        KeyCode::Digit9,
    ];
    for (i, &key) in digit_keys.iter().enumerate() {
        if keys.just_pressed(key) && i < NODES.len() {
            if can_unlock(i, NODES, &state.unlocked, state.points) {
                state.unlocked.insert(i);
                state.points -= NODES[i].cost;
            }
        }
    }
}

fn refresh_nodes(
    state: Res<TreeState>,
    mut q: Query<(&NodeSprite, &mut Sprite)>,
) {
    for (ns, mut sprite) in &mut q {
        let idx = ns.0;
        sprite.color = if state.unlocked.contains(&idx) {
            Color::srgb(0.2, 0.65, 0.25) // unlocked: green
        } else if can_unlock(idx, NODES, &state.unlocked, state.points) {
            Color::srgb(0.75, 0.65, 0.1) // available: yellow
        } else {
            Color::srgb(0.18, 0.18, 0.2) // locked: dark grey
        };
    }
}

fn update_points_label(state: Res<TreeState>, mut q: Query<&mut Text, With<PointsLabel>>) {
    let Ok(mut text) = q.single_mut() else { return };
    text.0 = format!("Skill points: {}", state.points);
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    fn unlocked(ids: &[usize]) -> HashSet<usize> { ids.iter().cloned().collect() }

    #[test]
    fn basics_already_unlocked_cannot_unlock_again() {
        let u = unlocked(&[0]);
        assert!(!can_unlock(0, NODES, &u, 99));
    }

    #[test]
    fn tier1_unlockable_when_basics_unlocked() {
        let u = unlocked(&[0]);
        assert!(can_unlock(1, NODES, &u, 1));
    }

    #[test]
    fn tier1_locked_without_prereq() {
        let u = unlocked(&[]);
        assert!(!can_unlock(1, NODES, &u, 99));
    }

    #[test]
    fn insufficient_points_blocks_unlock() {
        let u = unlocked(&[0]);
        assert!(!can_unlock(1, NODES, &u, 0));
    }

    #[test]
    fn whirlwind_needs_both_prereqs() {
        let only_flurry = unlocked(&[0, 1, 4]);
        assert!(!can_unlock(7, NODES, &only_flurry, 99));
        let both = unlocked(&[0, 1, 2, 4, 5]);
        assert!(can_unlock(7, NODES, &both, 99));
    }

    #[test]
    fn tier2_lockable_when_tier1_missing() {
        let u = unlocked(&[0]);
        assert!(!can_unlock(4, NODES, &u, 99)); // Flurry needs Quick Strike
    }
}
