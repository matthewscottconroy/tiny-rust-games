//! AABB collision detection demo.
//!
//! Key ideas:
//! - [`aabb_overlap`] is a pure function — no physics engine needed.
//! - [`mtv`] (Minimum Translation Vector) resolves an overlap by the smallest
//!   possible push along either axis, preventing sprites from tunnelling.
//! - Collision pairs are found with a brute-force O(n²) loop; for larger
//!   entity counts a spatial hash or BVH would replace this.

use bevy::prelude::*;

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .add_systems(Startup, setup)
        .add_systems(Update, (move_boxes, detect_and_resolve_collisions, bounce_walls))
        .run();
}

// --- Components ---

/// Stores the original size and base color of a box so we can restore the
/// color each frame after collision tinting.
#[derive(Component)]
struct Box {
    size: Vec2,
    base_color: Color,
}

/// 2D linear velocity component.
#[derive(Component)]
struct Velocity(Vec2);

// --- Setup ---

fn setup(mut commands: Commands) {
    commands.spawn(Camera2d);

    let boxes: &[(Vec3, Vec2, Vec2, Color)] = &[
        (Vec3::new(-180.0, 120.0, 0.0), Vec2::new(50.0, 32.0), Vec2::new( 130.0,  90.0), Color::srgb(0.3, 0.6, 0.9)),
        (Vec3::new(  80.0,  80.0, 0.0), Vec2::new(64.0, 48.0), Vec2::new( -90.0, 110.0), Color::srgb(0.4, 0.85, 0.4)),
        (Vec3::new(-100.0, -80.0, 0.0), Vec2::new(40.0, 40.0), Vec2::new( 160.0, -70.0), Color::srgb(0.85, 0.7, 0.2)),
        (Vec3::new( 160.0, -60.0, 0.0), Vec2::new(72.0, 36.0), Vec2::new(-110.0, -80.0), Color::srgb(0.7, 0.3, 0.85)),
        (Vec3::new(   0.0,   0.0, 0.0), Vec2::new(48.0, 64.0), Vec2::new(  80.0,-120.0), Color::srgb(0.9, 0.45, 0.2)),
    ];

    for &(pos, size, vel, color) in boxes {
        commands.spawn((
            Sprite { color, custom_size: Some(size), ..default() },
            Transform::from_translation(pos),
            Box { size, base_color: color },
            Velocity(vel),
        ));
    }

    commands.spawn((
        Text::new("Boxes resolve AABB overlaps each frame — orange = colliding"),
        TextFont { font_size: 14.0, ..default() },
        TextColor(Color::srgb(0.65, 0.65, 0.65)),
        Node {
            position_type: PositionType::Absolute,
            bottom: Val::Px(10.0),
            left: Val::Px(10.0),
            ..default()
        },
    ));
}

// --- Pure collision functions ---

/// Returns `true` when two axis-aligned bounding boxes overlap.
///
/// Both boxes are described by their **centre** (`*_pos`) and **half-extents**
/// (`*_half`, i.e. half the width/height).  The test is strict — touching
/// edges (zero penetration depth) are *not* considered a collision.
///
/// # Arguments
/// * `a_pos` — Centre of box A.
/// * `a_half` — Half-extents of box A (half-width, half-height).
/// * `b_pos` / `b_half` — Same for box B.
pub fn aabb_overlap(a_pos: Vec2, a_half: Vec2, b_pos: Vec2, b_half: Vec2) -> bool {
    (a_pos.x - a_half.x) < (b_pos.x + b_half.x)
        && (a_pos.x + a_half.x) > (b_pos.x - b_half.x)
        && (a_pos.y - a_half.y) < (b_pos.y + b_half.y)
        && (a_pos.y + a_half.y) > (b_pos.y - b_half.y)
}

/// Returns the Minimum Translation Vector to push box A out of box B.
///
/// The MTV points along the axis with the **smallest penetration depth**.
/// Apply `+mtv * 0.5` to A and `-mtv * 0.5` to B to separate them equally.
///
/// # Arguments
/// * `a_pos` / `a_half` — Centre and half-extents of box A.
/// * `b_pos` / `b_half` — Centre and half-extents of box B.
pub fn mtv(a_pos: Vec2, a_half: Vec2, b_pos: Vec2, b_half: Vec2) -> Vec2 {
    let dx_right = (b_pos.x + b_half.x) - (a_pos.x - a_half.x);
    let dx_left  = (a_pos.x + a_half.x) - (b_pos.x - b_half.x);
    let dy_up    = (b_pos.y + b_half.y) - (a_pos.y - a_half.y);
    let dy_down  = (a_pos.y + a_half.y) - (b_pos.y - b_half.y);

    let push_x = if dx_right < dx_left { dx_right } else { -dx_left };
    let push_y = if dy_up   < dy_down  { dy_up    } else { -dy_down };

    if push_x.abs() < push_y.abs() {
        Vec2::new(push_x, 0.0)
    } else {
        Vec2::new(0.0, push_y)
    }
}

// --- Systems ---

/// Moves each box by its velocity each frame.
fn move_boxes(time: Res<Time>, mut query: Query<(&mut Transform, &Velocity)>) {
    for (mut transform, vel) in &mut query {
        transform.translation.x += vel.0.x * time.delta_secs();
        transform.translation.y += vel.0.y * time.delta_secs();
    }
}

/// Detects overlapping pairs, resolves them with the MTV, and tints them orange.
fn detect_and_resolve_collisions(
    mut query: Query<(Entity, &mut Transform, &mut Sprite, &Box, &mut Velocity)>,
) {
    // Snapshot positions first so we can read all while mutating individually.
    let snapshot: Vec<(Entity, Vec2, Vec2)> = query
        .iter()
        .map(|(e, t, _, b, _)| (e, t.translation.truncate(), b.size / 2.0))
        .collect();

    for (_, _, mut sprite, b, _) in &mut query {
        sprite.color = b.base_color;
    }

    for i in 0..snapshot.len() {
        for j in (i + 1)..snapshot.len() {
            let (ea, pos_a, half_a) = snapshot[i];
            let (eb, pos_b, half_b) = snapshot[j];

            if !aabb_overlap(pos_a, half_a, pos_b, half_b) {
                continue;
            }

            let push = mtv(pos_a, half_a, pos_b, half_b);

            if let Ok((_, mut ta, mut sa, _, mut va)) = query.get_mut(ea) {
                ta.translation.x += push.x * 0.5;
                ta.translation.y += push.y * 0.5;
                if push.x.abs() > push.y.abs() { va.0.x *= -1.0; } else { va.0.y *= -1.0; }
                sa.color = Color::srgb(1.0, 0.5, 0.1);
            }
            if let Ok((_, mut tb, mut sb, _, mut vb)) = query.get_mut(eb) {
                tb.translation.x -= push.x * 0.5;
                tb.translation.y -= push.y * 0.5;
                if push.x.abs() > push.y.abs() { vb.0.x *= -1.0; } else { vb.0.y *= -1.0; }
                sb.color = Color::srgb(1.0, 0.5, 0.1);
            }
        }
    }
}

/// Reflects each box's velocity when it reaches the arena boundary.
fn bounce_walls(mut query: Query<(&Transform, &mut Velocity, &Box)>) {
    for (transform, mut vel, b) in &mut query {
        let p = transform.translation;
        let hx = b.size.x / 2.0;
        let hy = b.size.y / 2.0;
        if (p.x - hx) < -480.0 || (p.x + hx) > 480.0 { vel.0.x *= -1.0; }
        if (p.y - hy) < -270.0 || (p.y + hy) > 270.0 { vel.0.y *= -1.0; }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // --- aabb_overlap ---

    #[test]
    fn overlapping_boxes_returns_true() {
        let a = Vec2::ZERO;
        let b = Vec2::new(5.0, 0.0);
        let half = Vec2::splat(10.0);
        assert!(aabb_overlap(a, half, b, half));
    }

    #[test]
    fn separated_boxes_returns_false() {
        let a = Vec2::ZERO;
        let b = Vec2::new(50.0, 0.0);
        let half = Vec2::splat(10.0);
        assert!(!aabb_overlap(a, half, b, half));
    }

    #[test]
    fn touching_edges_not_overlap() {
        // Edges just touching: A right edge == B left edge — strict test returns false.
        let a = Vec2::new(-10.0, 0.0);
        let b = Vec2::new(10.0, 0.0);
        let half = Vec2::new(10.0, 10.0);
        assert!(!aabb_overlap(a, half, b, half));
    }

    #[test]
    fn same_center_overlaps() {
        assert!(aabb_overlap(Vec2::ZERO, Vec2::splat(5.0), Vec2::ZERO, Vec2::splat(5.0)));
    }

    #[test]
    fn no_y_overlap_returns_false() {
        let a = Vec2::new(0.0,  20.0);
        let b = Vec2::new(0.0, -20.0);
        let half = Vec2::splat(5.0);
        assert!(!aabb_overlap(a, half, b, half));
    }

    // --- mtv ---

    #[test]
    fn mtv_x_overlap_pushes_horizontally() {
        // A is slightly left of B with a small x overlap.
        let a_pos  = Vec2::new(-3.0, 0.0);
        let b_pos  = Vec2::new( 3.0, 0.0);
        let half   = Vec2::new(5.0, 20.0); // big y overlap, small x overlap
        let push = mtv(a_pos, half, b_pos, half);
        assert_ne!(push.x, 0.0, "should push along x (smaller overlap)");
        assert_eq!(push.y, 0.0);
    }

    #[test]
    fn mtv_y_overlap_pushes_vertically() {
        // A is slightly above B with a small y overlap.
        let a_pos  = Vec2::new(0.0,  3.0);
        let b_pos  = Vec2::new(0.0, -3.0);
        let half   = Vec2::new(20.0, 5.0); // big x overlap, small y overlap
        let push = mtv(a_pos, half, b_pos, half);
        assert_eq!(push.x, 0.0);
        assert_ne!(push.y, 0.0, "should push along y (smaller overlap)");
    }

    #[test]
    fn mtv_push_is_nonzero_for_overlapping() {
        let half = Vec2::splat(10.0);
        let push = mtv(Vec2::ZERO, half, Vec2::new(5.0, 5.0), half);
        assert!(push.length() > 0.0);
    }

    // --- ECS ---

    #[test]
    fn setup_spawns_five_boxes() {
        let mut app = App::new();
        app.add_plugins(MinimalPlugins)
            .add_systems(Startup, setup);
        app.update();

        let mut q = app.world_mut().query::<&Box>();
        assert_eq!(q.iter(app.world()).count(), 5);
    }
}
