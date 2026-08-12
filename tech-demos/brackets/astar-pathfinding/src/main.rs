//! A* pathfinding demo — click anywhere and walk there through a random cave.
//!
//! Teaches: implementing bracket-lib's [`BaseMap`] and [`Algorithm2D`] traits so
//! `a_star_search` and `field_of_view_set` can navigate your own map type,
//! previewing a path under the mouse, and walking it one step per frame.
//!
//! **Controls:** move the mouse to preview a path; left-click to walk it.
//!
//! The map lives in [`Map`], which knows nothing about rendering — its
//! generation, bounds checking, and neighbour rules are all unit-tested without
//! a terminal. `State` below is only the presentation layer.
//!
//! Diagonal moves cost `1.4` rather than `1.0`, so A* prefers a straight line
//! over a staircase of equal step count.

use bracket_lib::prelude::*;

/// Map width in tiles. The console is 80x50, so the map fills it exactly.
pub const MAP_WIDTH: i32 = 80;
/// Map height in tiles.
pub const MAP_HEIGHT: i32 = 50;
/// How far the player can see, in tiles.
pub const VIEW_RADIUS: i32 = 8;
/// Cost of a cardinal (N/S/E/W) step.
pub const CARDINAL_COST: f32 = 1.0;
/// Cost of a diagonal step — roughly sqrt(2), so diagonals are not free.
pub const DIAGONAL_COST: f32 = 1.4;
/// How many wall tiles the generator scatters inside the border.
pub const WALL_COUNT: usize = 1400;

// Enforced at compile time: if a diagonal ever became as cheap as a cardinal
// step, A* would happily return staircase paths.
const _: () = assert!(DIAGONAL_COST > CARDINAL_COST);

/// What occupies a single map tile.
#[derive(PartialEq, Eq, Copy, Clone, Debug)]
pub enum TileType {
    /// Blocks movement and sight.
    Wall,
    /// Walkable and transparent.
    Floor,
}

/// Whether the player is choosing a destination or walking to one.
#[derive(PartialEq, Eq, Copy, Clone, Debug)]
pub enum Mode {
    /// Previewing the path under the mouse cursor.
    Waiting,
    /// Following a committed path, one tile per frame.
    Moving,
}

/// Converts tile coordinates to an index into [`Map::tiles`].
pub fn xy_idx(x: i32, y: i32) -> usize {
    (y as usize * MAP_WIDTH as usize) + x as usize
}

/// Converts an index into [`Map::tiles`] back to tile coordinates.
pub fn idx_xy(idx: usize) -> (i32, i32) {
    (idx as i32 % MAP_WIDTH, idx as i32 / MAP_WIDTH)
}

/// A grid of tiles with a solid border and randomly scattered interior walls.
///
/// Kept free of any rendering or input concern so the pathfinding rules can be
/// tested directly.
pub struct Map {
    /// Row-major tiles, indexed by [`xy_idx`].
    pub tiles: Vec<TileType>,
}

impl Map {
    /// Builds a map with a solid border and [`WALL_COUNT`] interior walls,
    /// guaranteeing `keep_clear` stays walkable so the player never starts
    /// inside a wall.
    pub fn new_random(rng: &mut RandomNumberGenerator, keep_clear: usize) -> Self {
        let mut map = Self {
            tiles: vec![TileType::Floor; (MAP_WIDTH * MAP_HEIGHT) as usize],
        };

        // Solid border, so pathfinding never has to handle a map edge.
        for x in 0..MAP_WIDTH {
            map.tiles[xy_idx(x, 0)] = TileType::Wall;
            map.tiles[xy_idx(x, MAP_HEIGHT - 1)] = TileType::Wall;
        }
        for y in 0..MAP_HEIGHT {
            map.tiles[xy_idx(0, y)] = TileType::Wall;
            map.tiles[xy_idx(MAP_WIDTH - 1, y)] = TileType::Wall;
        }

        for _ in 0..WALL_COUNT {
            let x = rng.range(1, MAP_WIDTH - 1);
            let y = rng.range(1, MAP_HEIGHT - 1);
            let idx = xy_idx(x, y);
            if idx != keep_clear {
                map.tiles[idx] = TileType::Wall;
            }
        }

        map
    }

    /// Whether the tile at `idx` is a wall.
    pub fn is_wall(&self, idx: usize) -> bool {
        self.tiles[idx] == TileType::Wall
    }

    /// Whether `(x, y)` is inside the map and walkable.
    ///
    /// Returns `false` for out-of-bounds coordinates, so callers can probe
    /// neighbours without bounds-checking first.
    pub fn is_exit_valid(&self, x: i32, y: i32) -> bool {
        // The border is solid, so the playable area excludes row/column 0 and
        // the last row/column.
        if !(1..MAP_WIDTH - 1).contains(&x) || !(1..MAP_HEIGHT - 1).contains(&y) {
            return false;
        }
        self.tiles[xy_idx(x, y)] == TileType::Floor
    }
}

impl BaseMap for Map {
    fn is_opaque(&self, idx: usize) -> bool {
        self.is_wall(idx)
    }

    fn get_available_exits(&self, idx: usize) -> SmallVec<[(usize, f32); 10]> {
        let mut exits = SmallVec::new();
        let (x, y) = idx_xy(idx);

        for (dx, dy, cost) in [
            (-1, 0, CARDINAL_COST),
            (1, 0, CARDINAL_COST),
            (0, -1, CARDINAL_COST),
            (0, 1, CARDINAL_COST),
            (-1, -1, DIAGONAL_COST),
            (1, -1, DIAGONAL_COST),
            (-1, 1, DIAGONAL_COST),
            (1, 1, DIAGONAL_COST),
        ] {
            if self.is_exit_valid(x + dx, y + dy) {
                exits.push((xy_idx(x + dx, y + dy), cost));
            }
        }
        exits
    }

    fn get_pathing_distance(&self, idx1: usize, idx2: usize) -> f32 {
        let (x1, y1) = idx_xy(idx1);
        let (x2, y2) = idx_xy(idx2);
        DistanceAlg::Pythagoras.distance2d(Point::new(x1, y1), Point::new(x2, y2))
    }
}

impl Algorithm2D for Map {
    fn dimensions(&self) -> Point {
        Point::new(MAP_WIDTH, MAP_HEIGHT)
    }
}

/// Everything the frame loop needs: the map, where the player is, and whether
/// a path is being previewed or walked.
struct State {
    map: Map,
    player_position: usize,
    visible: Vec<bool>,
    mode: Mode,
    path: NavigationPath,
}

impl State {
    fn new() -> Self {
        let player_position = xy_idx(MAP_WIDTH / 2, MAP_HEIGHT / 2);
        let mut rng = RandomNumberGenerator::new();
        Self {
            map: Map::new_random(&mut rng, player_position),
            player_position,
            visible: vec![false; (MAP_WIDTH * MAP_HEIGHT) as usize],
            mode: Mode::Waiting,
            path: NavigationPath::new(),
        }
    }

    /// Recomputes which tiles the player can currently see.
    fn update_visibility(&mut self) {
        self.visible.fill(false);
        let origin = self.map.index_to_point2d(self.player_position);
        // In a real game this would only run when the player moves.
        for point in field_of_view_set(origin, VIEW_RADIUS, &self.map) {
            self.visible[xy_idx(point.x, point.y)] = true;
        }
    }

    fn draw_map(&self, batch: &mut DrawBatch) {
        for (idx, tile) in self.map.tiles.iter().enumerate() {
            let (glyph, mut fg) = match tile {
                TileType::Floor => (".", RGB::from_f32(0.5, 0.5, 0.0)),
                TileType::Wall => ("#", RGB::from_f32(0.0, 1.0, 0.0)),
            };
            // Remembered-but-unseen tiles are greyed rather than hidden.
            if !self.visible[idx] {
                fg = fg.to_greyscale();
            }
            let (x, y) = idx_xy(idx);
            batch.print_color(
                Point::new(x, y),
                glyph,
                ColorPair::new(fg, RGB::from_f32(0., 0., 0.)),
            );
        }
    }

    /// Previews the path to the cursor, and commits to it on click.
    fn preview_path(&mut self, batch: &mut DrawBatch) {
        let mouse_pos = INPUT.lock().mouse_tile(0);
        let mouse_idx = self.map.point2d_to_index(mouse_pos);
        batch.print_color(
            mouse_pos,
            "X",
            ColorPair::new(RGB::from_f32(0.0, 1.0, 1.0), RGB::from_f32(0.0, 1.0, 1.0)),
        );

        if self.map.is_wall(mouse_idx) {
            return;
        }
        let path = a_star_search(self.player_position, mouse_idx, &self.map);
        if !path.success {
            return;
        }
        // `steps[0]` is the tile the player already occupies.
        for step in path.steps.iter().skip(1) {
            let (x, y) = idx_xy(*step);
            batch.print_color(
                Point::new(x, y),
                "*",
                ColorPair::new(RGB::from_f32(1., 0., 0.), RGB::from_f32(0., 0., 0.)),
            );
        }
        if INPUT.lock().is_mouse_button_pressed(0) {
            self.mode = Mode::Moving;
            self.path = path;
        }
    }

    /// Advances one tile along the committed path.
    fn advance_along_path(&mut self) {
        if self.path.steps.is_empty() {
            self.mode = Mode::Waiting;
            return;
        }
        self.player_position = self.path.steps.remove(0);
        if self.path.steps.is_empty() {
            self.mode = Mode::Waiting;
        }
    }
}

impl GameState for State {
    fn tick(&mut self, ctx: &mut BTerm) {
        self.update_visibility();

        let mut batch = DrawBatch::new();
        batch.cls();
        self.draw_map(&mut batch);

        match self.mode {
            Mode::Waiting => self.preview_path(&mut batch),
            Mode::Moving => self.advance_along_path(),
        }

        let (px, py) = idx_xy(self.player_position);
        batch.print_color(
            Point::new(px, py),
            "@",
            ColorPair::new(RGB::from_f32(1.0, 1.0, 0.0), RGB::from_f32(0., 0., 0.)),
        );

        batch.submit(0).expect("Batch error");
        render_draw_buffer(ctx).expect("Render error");
    }
}

fn main() -> BError {
    let context = BTermBuilder::simple80x50()
        .with_title("A* Pathfinding — click to walk")
        .build()?;
    main_loop(context, State::new())
}

#[cfg(test)]
mod tests {
    use super::*;

    /// An all-floor map with the standard solid border.
    fn open_map() -> Map {
        let mut map = Map {
            tiles: vec![TileType::Floor; (MAP_WIDTH * MAP_HEIGHT) as usize],
        };
        for x in 0..MAP_WIDTH {
            map.tiles[xy_idx(x, 0)] = TileType::Wall;
            map.tiles[xy_idx(x, MAP_HEIGHT - 1)] = TileType::Wall;
        }
        for y in 0..MAP_HEIGHT {
            map.tiles[xy_idx(0, y)] = TileType::Wall;
            map.tiles[xy_idx(MAP_WIDTH - 1, y)] = TileType::Wall;
        }
        map
    }

    #[test]
    fn coordinates_round_trip_through_indices() {
        for (x, y) in [(0, 0), (1, 1), (79, 49), (40, 25), (12, 3)] {
            assert_eq!(idx_xy(xy_idx(x, y)), (x, y), "failed for ({x}, {y})");
        }
    }

    #[test]
    fn generated_map_has_a_solid_border() {
        let mut rng = RandomNumberGenerator::seeded(42);
        let map = Map::new_random(&mut rng, xy_idx(40, 25));
        for x in 0..MAP_WIDTH {
            assert!(map.is_wall(xy_idx(x, 0)), "top x={x}");
            assert!(map.is_wall(xy_idx(x, MAP_HEIGHT - 1)), "bottom x={x}");
        }
        for y in 0..MAP_HEIGHT {
            assert!(map.is_wall(xy_idx(0, y)), "left y={y}");
            assert!(map.is_wall(xy_idx(MAP_WIDTH - 1, y)), "right y={y}");
        }
    }

    #[test]
    fn the_players_starting_tile_is_never_walled_in() {
        let start = xy_idx(40, 25);
        for seed in [1, 2, 3, 99, 12345] {
            let mut rng = RandomNumberGenerator::seeded(seed);
            let map = Map::new_random(&mut rng, start);
            assert!(!map.is_wall(start), "seed {seed} buried the player");
        }
    }

    #[test]
    fn generation_is_deterministic_for_a_given_seed() {
        let start = xy_idx(40, 25);
        let a = Map::new_random(&mut RandomNumberGenerator::seeded(7), start);
        let b = Map::new_random(&mut RandomNumberGenerator::seeded(7), start);
        assert_eq!(a.tiles, b.tiles);
    }

    #[test]
    fn border_tiles_are_not_valid_exits() {
        let map = open_map();
        assert!(!map.is_exit_valid(0, 25));
        assert!(!map.is_exit_valid(MAP_WIDTH - 1, 25));
        assert!(!map.is_exit_valid(40, 0));
        assert!(!map.is_exit_valid(40, MAP_HEIGHT - 1));
    }

    #[test]
    fn out_of_bounds_coordinates_are_rejected_without_panicking() {
        let map = open_map();
        assert!(!map.is_exit_valid(-1, 25));
        assert!(!map.is_exit_valid(40, -1));
        assert!(!map.is_exit_valid(9999, 25));
        assert!(!map.is_exit_valid(40, 9999));
    }

    #[test]
    fn open_floor_has_eight_exits() {
        let map = open_map();
        let exits = map.get_available_exits(xy_idx(40, 25));
        assert_eq!(exits.len(), 8);
    }

    #[test]
    fn diagonal_exits_cost_more_than_cardinal_ones() {
        let map = open_map();
        let exits = map.get_available_exits(xy_idx(40, 25));
        let cardinals = exits.iter().filter(|(_, c)| *c == CARDINAL_COST).count();
        let diagonals = exits.iter().filter(|(_, c)| *c == DIAGONAL_COST).count();
        assert_eq!((cardinals, diagonals), (4, 4));
    }

    #[test]
    fn a_tile_beside_the_border_loses_its_out_of_bounds_exits() {
        let map = open_map();
        // (1, 1) is the top-left playable tile: only 3 of 8 neighbours are open.
        assert_eq!(map.get_available_exits(xy_idx(1, 1)).len(), 3);
    }

    #[test]
    fn walls_block_movement_and_sight() {
        let mut map = open_map();
        let idx = xy_idx(40, 25);
        map.tiles[idx] = TileType::Wall;
        assert!(map.is_wall(idx));
        assert!(map.is_opaque(idx));
        assert!(!map.is_exit_valid(40, 25));
    }

    #[test]
    fn pathing_distance_is_straight_line_not_step_count() {
        let map = open_map();
        let d = map.get_pathing_distance(xy_idx(10, 10), xy_idx(13, 14));
        assert!((d - 5.0).abs() < 1e-4, "expected 3-4-5 triangle, got {d}");
    }

    #[test]
    fn a_star_finds_a_route_across_open_floor() {
        let map = open_map();
        let path = a_star_search(xy_idx(5, 5), xy_idx(70, 40), &map);
        assert!(path.success);
        assert_eq!(path.steps.first(), Some(&xy_idx(5, 5)));
        assert_eq!(path.steps.last(), Some(&xy_idx(70, 40)));
    }

    #[test]
    fn a_star_fails_when_the_goal_is_sealed_off() {
        let mut map = open_map();
        let goal = xy_idx(40, 25);
        // Wall the goal in on all eight sides.
        for dy in -1..=1 {
            for dx in -1..=1 {
                if (dx, dy) != (0, 0) {
                    map.tiles[xy_idx(40 + dx, 25 + dy)] = TileType::Wall;
                }
            }
        }
        assert!(!a_star_search(xy_idx(5, 5), goal, &map).success);
    }
}
