//! Y-sort (depth sorting) demo.
//!
//! Key idea: for top-down or isometric views, sprites lower on screen are
//! visually "closer" and should render in front.  Setting `Z = -Y * scale`
//! each frame achieves this with a single line of math.
//!
//! Move the player over the wandering characters to see them pop in front of
//! or behind the player based on their relative vertical positions.

use bevy::prelude::*;

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .add_systems(Startup, setup)
        .add_systems(Update, (move_player, wander_characters, y_sort, update_hud))
        .run();
}

// --- Components ---

/// Marks the player entity.
#[derive(Component)]
struct Player;

/// A wandering non-player character with an autonomous velocity.
#[derive(Component)]
struct Character {
    velocity: Vec2,
}

/// Tags an entity whose Z coordinate should be derived from its Y position.
#[derive(Component)]
struct YSorted;

/// Marks the position / depth display text.
#[derive(Component)]
struct HudText;

// --- Constants ---

const PLAYER_SPEED: f32 = 160.0;

// --- Setup ---

/// Spawns a checkerboard ground, four wandering characters, the player, and HUD.
fn setup(mut commands: Commands) {
    commands.spawn(Camera2d);

    for row in -4..=4 {
        for col in -6..=6 {
            let shade = if (row + col) % 2 == 0 { 0.12 } else { 0.16 };
            commands.spawn((
                Sprite {
                    color: Color::srgb(shade, shade * 1.2, shade),
                    custom_size: Some(Vec2::splat(72.0)),
                    ..default()
                },
                Transform::from_xyz(col as f32 * 72.0, row as f32 * 72.0, -100.0),
            ));
        }
    }

    let chars: &[(Vec3, Vec2, Color)] = &[
        (Vec3::new(-150.0,  60.0, 0.0), Vec2::new( 55.0,  30.0), Color::srgb(0.85, 0.35, 0.2)),
        (Vec3::new(  80.0, -40.0, 0.0), Vec2::new(-40.0,  65.0), Color::srgb(0.2,  0.6,  0.9)),
        (Vec3::new( 180.0, 100.0, 0.0), Vec2::new(-70.0, -45.0), Color::srgb(0.8,  0.75, 0.1)),
        (Vec3::new(-200.0,-100.0, 0.0), Vec2::new( 60.0, -55.0), Color::srgb(0.5,  0.85, 0.5)),
    ];

    for &(pos, vel, color) in chars {
        commands.spawn((
            Sprite { color, custom_size: Some(Vec2::new(22.0, 32.0)), ..default() },
            Transform::from_translation(pos),
            Character { velocity: vel },
            YSorted,
        ));
    }

    commands.spawn((
        Sprite {
            color: Color::srgb(0.95, 0.95, 0.95),
            custom_size: Some(Vec2::new(22.0, 32.0)),
            ..default()
        },
        Transform::default(),
        Player,
        YSorted,
    ));

    commands.spawn((
        Text::new(""),
        TextFont { font_size: 14.0, ..default() },
        TextColor(Color::srgb(0.8, 0.8, 0.8)),
        Node {
            position_type: PositionType::Absolute,
            top: Val::Px(10.0),
            left: Val::Px(10.0),
            ..default()
        },
        HudText,
    ));

    commands.spawn((
        Text::new("WASD — move   walk behind/in-front of colored characters"),
        TextFont { font_size: 14.0, ..default() },
        TextColor(Color::srgb(0.55, 0.55, 0.55)),
        Node {
            position_type: PositionType::Absolute,
            bottom: Val::Px(10.0),
            left: Val::Px(10.0),
            ..default()
        },
    ));
}

// --- Pure helper ---

/// Converts a world-space Y coordinate to a Z depth for Y-sorting.
///
/// Lower Y (further down on screen) yields a higher Z (rendered in front),
/// matching the top-down perspective convention that objects at the bottom of
/// the screen appear closest to the viewer.
pub fn y_to_z(y: f32) -> f32 {
    -y * 0.001
}

// --- Systems ---

/// Reads WASD input and moves the player.
fn move_player(
    time: Res<Time>,
    input: Res<ButtonInput<KeyCode>>,
    mut query: Query<&mut Transform, With<Player>>,
) {
    let Ok(mut t) = query.single_mut() else { return; };
    let mut dir = Vec2::ZERO;
    if input.pressed(KeyCode::KeyW) { dir.y += 1.0; }
    if input.pressed(KeyCode::KeyS) { dir.y -= 1.0; }
    if input.pressed(KeyCode::KeyA) { dir.x -= 1.0; }
    if input.pressed(KeyCode::KeyD) { dir.x += 1.0; }
    if dir != Vec2::ZERO {
        let d = dir.normalize() * PLAYER_SPEED * time.delta_secs();
        t.translation.x += d.x;
        t.translation.y += d.y;
    }
}

/// Moves each character along its velocity and bounces it at arena edges.
fn wander_characters(time: Res<Time>, mut query: Query<(&mut Transform, &mut Character)>) {
    for (mut transform, mut ch) in &mut query {
        transform.translation.x += ch.velocity.x * time.delta_secs();
        transform.translation.y += ch.velocity.y * time.delta_secs();
        if transform.translation.x.abs() > 350.0 { ch.velocity.x *= -1.0; }
        if transform.translation.y.abs() > 240.0 { ch.velocity.y *= -1.0; }
    }
}

/// Overwrites the Z coordinate of every [`YSorted`] entity using [`y_to_z`].
fn y_sort(mut query: Query<&mut Transform, With<YSorted>>) {
    for mut transform in &mut query {
        transform.translation.z = y_to_z(transform.translation.y);
    }
}

/// Updates the HUD with the player's current Y and derived Z.
fn update_hud(
    player_query: Query<&Transform, With<Player>>,
    mut hud_query: Query<&mut Text, With<HudText>>,
) {
    let Ok(t) = player_query.single() else { return; };
    for mut text in &mut hud_query {
        *text = Text::new(format!(
            "Player Y={:.0}  Z={:.4} (higher Z = in front)",
            t.translation.y,
            t.translation.z,
        ));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // --- y_to_z ---

    #[test]
    fn y_to_z_negative_y_gives_positive_z() {
        assert!((y_to_z(-100.0) - 0.1).abs() < 1e-6);
    }

    #[test]
    fn y_to_z_positive_y_gives_negative_z() {
        assert!((y_to_z(100.0) + 0.1).abs() < 1e-6);
    }

    #[test]
    fn y_to_z_zero_gives_zero() {
        assert_eq!(y_to_z(0.0), 0.0);
    }

    #[test]
    fn lower_y_has_higher_z() {
        assert!(y_to_z(-200.0) > y_to_z(200.0),
            "sprite lower on screen should have higher Z");
    }

    #[test]
    fn y_to_z_is_linear() {
        let ratio = y_to_z(50.0) / y_to_z(25.0);
        assert!((ratio - 2.0).abs() < 1e-5);
    }

    // --- ECS ---

    #[test]
    fn setup_spawns_one_player() {
        let mut app = App::new();
        app.add_plugins(MinimalPlugins)
            .add_systems(Startup, setup);
        app.update();

        let mut q = app.world_mut().query::<&Player>();
        assert_eq!(q.iter(app.world()).count(), 1);
    }

    #[test]
    fn setup_spawns_four_characters() {
        let mut app = App::new();
        app.add_plugins(MinimalPlugins)
            .add_systems(Startup, setup);
        app.update();

        let mut q = app.world_mut().query::<&Character>();
        assert_eq!(q.iter(app.world()).count(), 4);
    }

    #[test]
    fn setup_spawns_five_y_sorted_entities() {
        // 4 characters + 1 player
        let mut app = App::new();
        app.add_plugins(MinimalPlugins)
            .add_systems(Startup, setup);
        app.update();

        let mut q = app.world_mut().query::<&YSorted>();
        assert_eq!(q.iter(app.world()).count(), 5);
    }
}
