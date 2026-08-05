//! Card game — a reusable Bevy plugin.
//!
//! This crate is a *building block*: drop [`CardGamePlugin`] into any Bevy app
//! with `app.add_plugins(CardGamePlugin)` and it manages a shuffled deck with
//! draw/hand/discard piles rendered as clickable card sprites.
//!
//! Key ideas:
//! - A [`Deck`] resource holds three `Vec<Card>` piles: draw, hand, discard.
//! - [`shuffle`] uses an in-place LCG Fisher-Yates to randomise without `rand`.
//! - [`draw_card`] moves the top card from draw → hand (recycling discard when
//!   the draw pile empties).
//! - [`play_card`] moves a card from hand → discard by index.
//! - [`hand_value`] sums the numeric value of cards currently in hand.
//! - Cards are rendered as coloured sprites; the hand fans out across the bottom
//!   of the screen. Clicking a hand card plays it.
//! - Tune the layout and starting seed through the [`CardGameConfig`] resource.
//!
//! **Controls:** SPACE — draw a card;  left-click a hand card — play it.
//!
//! # Example
//! ```no_run
//! use bevy::prelude::*;
//! use card_game::CardGamePlugin;
//!
//! App::new()
//!     .add_plugins(DefaultPlugins)
//!     .add_plugins(CardGamePlugin)
//!     .run();
//! ```

use bevy::prelude::*;

/// Bundles every system and resource for the card-game feature.
///
/// Add it with `app.add_plugins(CardGamePlugin)`. It does **not** add
/// `DefaultPlugins`, so the host app stays in control of window and rendering
/// setup.
pub struct CardGamePlugin;

impl Plugin for CardGamePlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<CardGameConfig>()
            .init_resource::<Deck>()
            .add_systems(Startup, setup)
            .add_systems(Update, (handle_input, sync_hand_display).chain());
    }
}

// --- Configuration ---

/// Tunable parameters for the card-game feature. Override before adding the
/// plugin, e.g. `app.insert_resource(CardGameConfig { seed: 42, ..default() })`.
#[derive(Resource, Clone, Copy, Debug)]
pub struct CardGameConfig {
    /// Seed for the initial deck shuffle and subsequent recycles.
    pub seed: u64,
    /// Width of a rendered hand card in pixels.
    pub card_width: f32,
    /// Height of a rendered hand card in pixels.
    pub card_height: f32,
    /// Vertical position of the fanned-out hand.
    pub hand_y: f32,
    /// Click radius (pixels) used to hit-test hand cards.
    pub click_radius: f32,
}

impl Default for CardGameConfig {
    fn default() -> Self {
        Self { seed: 12345, card_width: 52.0, card_height: 78.0, hand_y: -220.0, click_radius: 36.0 }
    }
}

// ── Pure card model ───────────────────────────────────────────────────────────

/// A single playing card: a suit and a rank in `1..=13`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Card {
    pub suit: Suit,
    pub rank: u8, // 1–13
}

/// The four card suits.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Suit { Clubs, Diamonds, Hearts, Spades }

impl Card {
    pub fn color(self) -> Color {
        match self.suit {
            Suit::Hearts | Suit::Diamonds => Color::srgb(0.88, 0.2, 0.2),
            Suit::Clubs  | Suit::Spades   => Color::srgb(0.15, 0.15, 0.15),
        }
    }
    pub fn label(self) -> String {
        let rank = match self.rank {
            1  => "A".to_string(),
            11 => "J".to_string(),
            12 => "Q".to_string(),
            13 => "K".to_string(),
            n  => n.to_string(),
        };
        let suit = match self.suit {
            Suit::Clubs    => "♣",
            Suit::Diamonds => "♦",
            Suit::Hearts   => "♥",
            Suit::Spades   => "♠",
        };
        format!("{}{}", rank, suit)
    }
    /// Point value: face cards = 10, ace = 11, others = rank.
    pub fn value(self) -> u32 {
        match self.rank {
            1      => 11,
            11..=13 => 10,
            n      => n as u32,
        }
    }
}

/// Minimum LCG step; returns the next state.
pub fn lcg_next(state: u64) -> u64 {
    state.wrapping_mul(6364136223846793005).wrapping_add(1442695040888963407)
}

/// In-place Fisher-Yates shuffle using a seeded LCG.
pub fn shuffle(cards: &mut Vec<Card>, seed: u64) {
    let mut rng = seed;
    for i in (1..cards.len()).rev() {
        rng = lcg_next(rng);
        let j = (rng >> 33) as usize % (i + 1);
        cards.swap(i, j);
    }
}

/// Moves the top card from `draw` to `hand`.  Recycles `discard` into `draw`
/// when the draw pile is empty.  Returns `false` when both piles are empty.
pub fn draw_card(draw: &mut Vec<Card>, hand: &mut Vec<Card>, discard: &mut Vec<Card>, seed: u64) -> bool {
    if draw.is_empty() {
        if discard.is_empty() { return false; }
        std::mem::swap(draw, discard);
        shuffle(draw, seed);
    }
    if let Some(c) = draw.pop() {
        hand.push(c);
        true
    } else {
        false
    }
}

/// Moves `hand[index]` to `discard`.  Returns `false` when `index` is out of range.
pub fn play_card(hand: &mut Vec<Card>, discard: &mut Vec<Card>, index: usize) -> bool {
    if index >= hand.len() { return false; }
    let card = hand.remove(index);
    discard.push(card);
    true
}

/// Sum of [`Card::value`] for all cards in `hand`.
pub fn hand_value(hand: &[Card]) -> u32 {
    hand.iter().map(|c| c.value()).sum()
}

/// Builds a full, ordered 52-card deck.
pub fn full_deck() -> Vec<Card> {
    let suits = [Suit::Clubs, Suit::Diamonds, Suit::Hearts, Suit::Spades];
    suits.iter()
        .flat_map(|&suit| (1..=13).map(move |rank| Card { suit, rank }))
        .collect()
}

// ── ECS ───────────────────────────────────────────────────────────────────────

/// The three card piles plus the running RNG state.
#[derive(Resource)]
pub struct Deck {
    pub draw: Vec<Card>,
    pub hand: Vec<Card>,
    pub discard: Vec<Card>,
    pub rng: u64,
}

impl Deck {
    pub fn new_shuffled(seed: u64) -> Self {
        let mut draw = full_deck();
        shuffle(&mut draw, seed);
        Self { draw, hand: Vec::new(), discard: Vec::new(), rng: seed }
    }
}

impl Default for Deck {
    fn default() -> Self {
        Self::new_shuffled(CardGameConfig::default().seed)
    }
}

/// Tags the N card sprites in the hand display.
#[derive(Component)]
struct HandCard(usize);

/// Tags the status text entity.
#[derive(Component)]
struct StatusText;

fn setup(mut commands: Commands, deck: Res<Deck>) {
    commands.spawn(Camera2d);

    commands.spawn((
        Text::new("SPACE — draw   click — play"),
        TextFont { font_size: 15.0, ..default() },
        TextColor(Color::srgb(0.55, 0.55, 0.55)),
        Node {
            position_type: PositionType::Absolute,
            top: Val::Px(8.0),
            left: Val::Px(10.0),
            ..default()
        },
    ));

    commands.spawn((
        Text::new(format!("Draw: {}  Hand: {}  Discard: {}", deck.draw.len(), 0, 0)),
        TextFont { font_size: 15.0, ..default() },
        TextColor(Color::srgb(0.8, 0.8, 0.8)),
        StatusText,
        Node {
            position_type: PositionType::Absolute,
            top: Val::Px(28.0),
            left: Val::Px(10.0),
            ..default()
        },
    ));
}

fn handle_input(
    keys: Res<ButtonInput<KeyCode>>,
    buttons: Res<ButtonInput<MouseButton>>,
    config: Res<CardGameConfig>,
    camera_q: Query<(&Camera, &GlobalTransform), With<Camera2d>>,
    windows: Query<&Window>,
    card_sprites: Query<(Entity, &HandCard, &Transform)>,
    mut deck: ResMut<Deck>,
) {
    if keys.just_pressed(KeyCode::Space) {
        let rng = deck.rng;
        deck.rng = lcg_next(rng);
        let d = &mut *deck;
        draw_card(&mut d.draw, &mut d.hand, &mut d.discard, rng);
    }

    if buttons.just_pressed(MouseButton::Left) {
        let Ok((camera, cam_tf)) = camera_q.single() else { return };
        let Ok(window) = windows.single() else { return };
        let Some(cursor) = window.cursor_position() else { return };
        let Ok(world) = camera.viewport_to_world_2d(cam_tf, cursor) else { return };

        let mut clicked_index: Option<usize> = None;
        let mut best_dist = config.click_radius;
        for (_, hc, tf) in &card_sprites {
            let d = tf.translation.truncate().distance(world);
            if d < best_dist {
                best_dist = d;
                clicked_index = Some(hc.0);
            }
        }
        if let Some(idx) = clicked_index {
            let d = &mut *deck;
            play_card(&mut d.hand, &mut d.discard, idx);
        }
    }
}

fn sync_hand_display(
    deck: Res<Deck>,
    config: Res<CardGameConfig>,
    mut commands: Commands,
    old_cards: Query<Entity, With<HandCard>>,
    mut status: Query<&mut Text, With<StatusText>>,
) {
    if !deck.is_changed() { return; }

    for e in &old_cards { commands.entity(e).despawn(); }

    let n = deck.hand.len();
    let card_w = config.card_width;
    let spacing = (card_w + 6.0).min(if n > 1 { 500.0 / (n - 1) as f32 } else { 0.0 });
    let start_x = -(spacing * (n.saturating_sub(1)) as f32) / 2.0;

    for (i, card) in deck.hand.iter().enumerate() {
        let x = start_x + i as f32 * spacing;
        let label = card.label();
        commands.spawn((
            Sprite {
                color: Color::srgb(0.96, 0.96, 0.92),
                custom_size: Some(Vec2::new(card_w, config.card_height)),
                ..default()
            },
            Transform::from_xyz(x, config.hand_y, 0.0),
            HandCard(i),
        )).with_children(|p| {
            p.spawn((
                Text2d::new(label),
                TextFont { font_size: 18.0, ..default() },
                TextColor(card.color()),
                Transform::from_xyz(0.0, 0.0, 1.0),
            ));
        });
    }

    let value = hand_value(&deck.hand);
    if let Ok(mut t) = status.single_mut() {
        *t = Text::new(format!(
            "Draw: {}  Hand: {} (val {})  Discard: {}",
            deck.draw.len(), deck.hand.len(), value, deck.discard.len()
        ));
    }
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    fn make_deck() -> (Vec<Card>, Vec<Card>, Vec<Card>) {
        let draw = full_deck();
        (draw, vec![], vec![])
    }

    #[test]
    fn full_deck_has_52_cards() {
        assert_eq!(full_deck().len(), 52);
    }

    #[test]
    fn shuffle_produces_different_order() {
        let mut d = full_deck();
        let original: Vec<Card> = d.clone();
        shuffle(&mut d, 999);
        assert_ne!(d, original);
    }

    #[test]
    fn shuffle_preserves_card_count() {
        let mut d = full_deck();
        shuffle(&mut d, 42);
        assert_eq!(d.len(), 52);
    }

    #[test]
    fn draw_card_moves_to_hand() {
        let (mut draw, mut hand, mut discard) = make_deck();
        assert!(draw_card(&mut draw, &mut hand, &mut discard, 1));
        assert_eq!(hand.len(), 1);
        assert_eq!(draw.len(), 51);
    }

    #[test]
    fn draw_card_recycles_discard() {
        let mut draw = vec![];
        let mut hand = vec![];
        let mut discard = vec![Card { suit: Suit::Hearts, rank: 5 }];
        assert!(draw_card(&mut draw, &mut hand, &mut discard, 1));
        assert_eq!(hand.len(), 1);
        assert!(discard.is_empty());
    }

    #[test]
    fn draw_card_returns_false_when_empty() {
        let (mut draw, mut hand, mut discard) = (vec![], vec![], vec![]);
        assert!(!draw_card(&mut draw, &mut hand, &mut discard, 1));
    }

    #[test]
    fn play_card_moves_to_discard() {
        let mut hand = vec![Card { suit: Suit::Spades, rank: 1 }];
        let mut discard = vec![];
        assert!(play_card(&mut hand, &mut discard, 0));
        assert!(hand.is_empty());
        assert_eq!(discard.len(), 1);
    }

    #[test]
    fn play_card_out_of_range_returns_false() {
        let mut hand = vec![];
        let mut discard = vec![];
        assert!(!play_card(&mut hand, &mut discard, 0));
    }

    #[test]
    fn hand_value_ace_is_eleven() {
        let hand = vec![Card { suit: Suit::Hearts, rank: 1 }];
        assert_eq!(hand_value(&hand), 11);
    }

    #[test]
    fn hand_value_face_card_is_ten() {
        for rank in [11u8, 12, 13] {
            let hand = vec![Card { suit: Suit::Clubs, rank }];
            assert_eq!(hand_value(&hand), 10);
        }
    }

    #[test]
    fn hand_value_sums_correctly() {
        let hand = vec![
            Card { suit: Suit::Hearts, rank: 5 },
            Card { suit: Suit::Spades, rank: 7 },
        ];
        assert_eq!(hand_value(&hand), 12);
    }

    #[test]
    fn config_default_seed_matches() {
        assert_eq!(CardGameConfig::default().seed, 12345);
    }

    #[test]
    fn deck_default_is_shuffled_full_deck() {
        let deck = Deck::default();
        assert_eq!(deck.draw.len(), 52);
        assert!(deck.hand.is_empty());
    }

    #[test]
    fn setup_spawns_status_text() {
        let mut app = App::new();
        app.add_plugins(MinimalPlugins)
            .init_resource::<CardGameConfig>()
            .init_resource::<Deck>()
            .add_systems(Startup, setup);
        app.update();

        let mut q = app.world_mut().query::<&StatusText>();
        assert_eq!(q.iter(app.world()).count(), 1);
    }
}
