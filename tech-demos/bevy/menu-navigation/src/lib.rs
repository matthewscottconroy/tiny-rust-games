//! Menu-navigation — a reusable Bevy plugin for an arrow-key navigable menu.
//!
//! This crate is a *building block*: drop [`MenuNavigationPlugin`] into any
//! Bevy app with `app.add_plugins(MenuNavigationPlugin)` and it renders a
//! keyboard-navigable main menu with state transitions into a placeholder
//! in-game scene.
//!
//! Key ideas:
//! - [`MenuState`] resource tracks the highlighted index; it wraps around at
//!   the top and bottom so the user can hold a direction continuously.
//! - Each `MenuItem` entity carries its `index` so the color-sync system does
//!   not need to maintain a separate lookup.
//! - Pressing ENTER on "Start Game" pushes the [`AppState::InGame`] state; ESC
//!   from InGame returns to the menu.
//! - All menu entities are tagged with `OnMainMenu` and despawned on exit to
//!   keep state transitions clean.
//!
//! Tune the menu labels and highlight colors through the
//! [`MenuNavigationConfig`] resource without editing the plugin's internals.
//!
//! **Controls:** Up/Down arrows to navigate; ENTER to select; ESC to return.
//!
//! # Example
//! ```no_run
//! use bevy::prelude::*;
//! use menu_navigation::MenuNavigationPlugin;
//!
//! App::new()
//!     .add_plugins(DefaultPlugins)
//!     .add_plugins(MenuNavigationPlugin)
//!     .run();
//! ```

use bevy::prelude::*;

/// Bundles every system, state, and resource for the menu-navigation feature.
///
/// Add it with `app.add_plugins(MenuNavigationPlugin)`. It does **not** add
/// `DefaultPlugins`, so the host app stays in control of window and rendering
/// setup.
pub struct MenuNavigationPlugin;

impl Plugin for MenuNavigationPlugin {
    fn build(&self, app: &mut App) {
        let config = MenuNavigationConfig::default();
        let item_count = config.items.len();
        app.init_state::<AppState>()
            .insert_resource(config)
            .insert_resource(MenuState {
                selected: 0,
                item_count,
            })
            .add_systems(OnEnter(AppState::MainMenu), setup_menu)
            .add_systems(OnExit(AppState::MainMenu), teardown_menu)
            .add_systems(OnEnter(AppState::InGame), setup_game)
            .add_systems(OnExit(AppState::InGame), teardown_game)
            .add_systems(Update, navigate_menu.run_if(in_state(AppState::MainMenu)))
            .add_systems(Update, return_to_menu.run_if(in_state(AppState::InGame)));
    }
}

/// Menu item labels in display order.
pub const MENU_ITEMS: [&str; 3] = ["Start Game", "Options", "Quit"];

/// Highlighted color for the selected menu item.
pub const HIGHLIGHT_COLOR: Color = Color::srgb(1.0, 0.9, 0.2);
/// Default color for unselected items.
pub const NORMAL_COLOR: Color = Color::srgb(0.8, 0.8, 0.8);

// --- Configuration ---

/// Tunable parameters for the feature. Override before adding the plugin.
///
/// Holds a `Vec` so it is `Clone` but not `Copy`.
#[derive(Resource, Clone, Debug)]
pub struct MenuNavigationConfig {
    /// Menu item labels in display order.
    pub items: Vec<&'static str>,
    /// Color of the currently selected item.
    pub highlight_color: Color,
    /// Color of unselected items.
    pub normal_color: Color,
}

impl Default for MenuNavigationConfig {
    fn default() -> Self {
        Self {
            items: MENU_ITEMS.to_vec(),
            highlight_color: HIGHLIGHT_COLOR,
            normal_color: NORMAL_COLOR,
        }
    }
}

/// Application-level state.
#[derive(Debug, Clone, Eq, PartialEq, Hash, Default, States)]
pub enum AppState {
    /// The menu is showing and accepting navigation keys.
    #[default]
    MainMenu,
    /// The game is running; Escape returns to the menu.
    InGame,
}

/// Tracks which menu item is currently highlighted.
#[derive(Resource)]
pub struct MenuState {
    /// Index of the highlighted item.
    pub selected: usize,
    /// Total number of items (used for wrapping).
    pub item_count: usize,
}

impl MenuState {
    /// Moves selection up (wrapping).
    pub fn move_up(&mut self) {
        if self.selected == 0 {
            self.selected = self.item_count.saturating_sub(1);
        } else {
            self.selected -= 1;
        }
    }

    /// Moves selection down (wrapping).
    pub fn move_down(&mut self) {
        self.selected = (self.selected + 1) % self.item_count.max(1);
    }
}

/// Marks a menu item entity; `index` matches its position in the config's items.
#[derive(Component)]
pub struct MenuItem {
    /// Position of this item in the menu.
    pub index: usize,
}

/// Marks entities that belong to the menu scene (despawned on exit).
#[derive(Component)]
pub struct OnMainMenu;

/// Marks entities that belong to the in-game scene.
#[derive(Component)]
pub struct OnInGame;

/// Spawns the menu items and a title.
fn setup_menu(mut commands: Commands, menu: Res<MenuState>, config: Res<MenuNavigationConfig>) {
    commands.spawn((Camera2d, OnMainMenu));

    commands.spawn((
        Text::new("DEMO GAME"),
        TextFont {
            font_size: 48.0,
            ..default()
        },
        TextColor(Color::WHITE),
        Node {
            position_type: PositionType::Absolute,
            top: Val::Px(80.0),
            left: Val::Px(0.0),
            right: Val::Px(0.0),
            justify_self: JustifySelf::Center,
            ..default()
        },
        OnMainMenu,
    ));

    for (i, label) in config.items.iter().enumerate() {
        let color = if i == menu.selected {
            config.highlight_color
        } else {
            config.normal_color
        };
        commands.spawn((
            Text::new(*label),
            TextFont {
                font_size: 32.0,
                ..default()
            },
            TextColor(color),
            Node {
                position_type: PositionType::Absolute,
                top: Val::Px(200.0 + i as f32 * 55.0),
                left: Val::Px(0.0),
                right: Val::Px(0.0),
                justify_self: JustifySelf::Center,
                ..default()
            },
            MenuItem { index: i },
            OnMainMenu,
        ));
    }

    commands.spawn((
        Text::new("^v navigate   ENTER select"),
        TextFont {
            font_size: 18.0,
            ..default()
        },
        TextColor(Color::srgba(0.7, 0.7, 0.7, 1.0)),
        Node {
            position_type: PositionType::Absolute,
            bottom: Val::Px(16.0),
            left: Val::Px(0.0),
            right: Val::Px(0.0),
            justify_self: JustifySelf::Center,
            ..default()
        },
        OnMainMenu,
    ));
}

/// Despawns all menu entities.
fn teardown_menu(mut commands: Commands, query: Query<Entity, With<OnMainMenu>>) {
    for entity in &query {
        commands.entity(entity).despawn();
    }
}

/// Handles arrow-key navigation and ENTER to select.
fn navigate_menu(
    input: Res<ButtonInput<KeyCode>>,
    config: Res<MenuNavigationConfig>,
    mut menu: ResMut<MenuState>,
    mut next_state: ResMut<NextState<AppState>>,
    mut item_query: Query<(&MenuItem, &mut TextColor)>,
) {
    if input.just_pressed(KeyCode::ArrowUp) {
        menu.move_up();
    }
    if input.just_pressed(KeyCode::ArrowDown) {
        menu.move_down();
    }

    // Sync highlight colours.
    for (item, mut color) in &mut item_query {
        color.0 = if item.index == menu.selected {
            config.highlight_color
        } else {
            config.normal_color
        };
    }

    if input.just_pressed(KeyCode::Enter) {
        match menu.selected {
            0 => next_state.set(AppState::InGame),
            2 => std::process::exit(0),
            _ => {}
        }
    }
}

/// Spawns a minimal in-game scene.
fn setup_game(mut commands: Commands) {
    commands.spawn((Camera2d, OnInGame));
    commands.spawn((
        Text::new("In Game!  Press ESC to return to menu."),
        TextFont {
            font_size: 28.0,
            ..default()
        },
        TextColor(Color::WHITE),
        Node {
            position_type: PositionType::Absolute,
            top: Val::Px(200.0),
            left: Val::Px(0.0),
            right: Val::Px(0.0),
            justify_self: JustifySelf::Center,
            ..default()
        },
        OnInGame,
    ));
}

/// Despawns all in-game entities.
fn teardown_game(mut commands: Commands, query: Query<Entity, With<OnInGame>>) {
    for entity in &query {
        commands.entity(entity).despawn();
    }
}

/// Returns to the main menu on ESC.
fn return_to_menu(input: Res<ButtonInput<KeyCode>>, mut next_state: ResMut<NextState<AppState>>) {
    if input.just_pressed(KeyCode::Escape) {
        next_state.set(AppState::MainMenu);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_menu(selected: usize, count: usize) -> MenuState {
        MenuState {
            selected,
            item_count: count,
        }
    }

    #[test]
    fn move_down_advances_selection() {
        let mut m = make_menu(0, 3);
        m.move_down();
        assert_eq!(m.selected, 1);
    }

    #[test]
    fn move_down_wraps_at_end() {
        let mut m = make_menu(2, 3);
        m.move_down();
        assert_eq!(m.selected, 0);
    }

    #[test]
    fn move_up_goes_backward() {
        let mut m = make_menu(2, 3);
        m.move_up();
        assert_eq!(m.selected, 1);
    }

    #[test]
    fn move_up_wraps_at_start() {
        let mut m = make_menu(0, 3);
        m.move_up();
        assert_eq!(m.selected, 2);
    }

    #[test]
    fn move_down_then_up_is_identity() {
        let mut m = make_menu(1, 3);
        m.move_down();
        m.move_up();
        assert_eq!(m.selected, 1);
    }

    #[test]
    fn full_cycle_down_returns_to_start() {
        let count = MENU_ITEMS.len();
        let mut m = make_menu(0, count);
        for _ in 0..count {
            m.move_down();
        }
        assert_eq!(m.selected, 0);
    }

    #[test]
    fn menu_items_count_matches_resource() {
        let menu = make_menu(0, MENU_ITEMS.len());
        assert_eq!(menu.item_count, MENU_ITEMS.len());
    }

    #[test]
    fn app_state_default_is_main_menu() {
        assert_eq!(AppState::default(), AppState::MainMenu);
    }

    #[test]
    fn config_default_matches_menu_items() {
        let c = MenuNavigationConfig::default();
        assert_eq!(c.items, MENU_ITEMS.to_vec());
        assert_eq!(c.highlight_color, HIGHLIGHT_COLOR);
        assert_eq!(c.normal_color, NORMAL_COLOR);
    }
}
