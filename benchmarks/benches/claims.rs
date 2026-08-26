//! Benchmarks that check the demos' own performance claims.
//!
//! Several demos assert something about performance in their documentation —
//! "O(1) neighbour queries", "recycled rather than spawned". Those were the last
//! category of claim in this repository that nothing verified: `missing_docs`
//! keeps the docs present, the catalogue check keeps them current, and the tests
//! keep the behaviour right, but none of that says whether the fast path is
//! actually fast.
//!
//! These benchmarks measure the claim against the naive implementation it says
//! it beats, at several sizes. The interesting output is not "the optimisation
//! wins" — it is *where it starts winning*, because a demo that shows when not
//! to use itself is worth more than one that only shows its best case.
//!
//! ```bash
//! cargo bench --manifest-path benchmarks/Cargo.toml
//! ```

// `criterion_main!` expands to an undocumented `pub fn main`, which the
// workspace's `missing_docs` lint would reject under CI's `-D warnings`. The
// lint is right about library code and wrong about a generated benchmark entry
// point, so it is disabled here rather than repository-wide.
#![allow(missing_docs)]

use std::collections::HashMap;
use std::hint::black_box;

use criterion::{BenchmarkId, Criterion, criterion_group, criterion_main};

use bevy::math::{IVec2, Vec2};
use flow_field_pathfinding::compute_flow_field;
use snake_lib::{Direction, SnakeGame};
use spatial_partitioning::{brute_pairs, cell_of, neighbour_cells};
use tic_tac_toe_lib::{Board, Player, TicTacToeGame};

/// A deterministic scatter of points, so every run measures the same work.
fn points(n: usize, spread: f32) -> Vec<Vec2> {
    // A small LCG rather than a dependency; the exact distribution does not
    // matter, only that it is the same every run.
    let mut state = 0x2545F491_4F6CDD1Du64;
    let mut next = || {
        state = state.wrapping_mul(6364136223846793005).wrapping_add(1);
        ((state >> 33) as f32 / u32::MAX as f32) * spread - spread / 2.0
    };
    (0..n).map(|_| Vec2::new(next(), next())).collect()
}

/// Every close pair, checked the obvious way: compare all N*(N-1)/2 pairs.
fn brute_force(points: &[Vec2], radius: f32) -> usize {
    let r2 = radius * radius;
    let mut hits = 0;
    for i in 0..points.len() {
        for j in (i + 1)..points.len() {
            if points[i].distance_squared(points[j]) <= r2 {
                hits += 1;
            }
        }
    }
    hits
}

/// The same answer via the demo's grid: bucket by cell, then compare only
/// against the 3x3 patch around each point.
///
/// Uses the demo's own `cell_of` and `neighbour_cells`, so this measures the
/// technique the demo actually teaches rather than a private reimplementation.
fn grid_bucketed(points: &[Vec2], radius: f32, cell_size: f32) -> usize {
    let r2 = radius * radius;
    let mut grid: HashMap<IVec2, Vec<usize>> = HashMap::new();
    for (i, p) in points.iter().enumerate() {
        grid.entry(cell_of(*p, cell_size)).or_default().push(i);
    }

    let mut hits = 0;
    for (i, p) in points.iter().enumerate() {
        for cell in neighbour_cells(cell_of(*p, cell_size)) {
            let Some(bucket) = grid.get(&cell) else {
                continue;
            };
            for &j in bucket {
                // `j > i` counts each pair once, matching brute_force.
                if j > i && p.distance_squared(points[j]) <= r2 {
                    hits += 1;
                }
            }
        }
    }
    hits
}

/// Does bucketing actually beat comparing every pair, and from what size?
fn spatial_partitioning(c: &mut Criterion) {
    const RADIUS: f32 = 24.0;
    const CELL: f32 = 48.0;
    const SPREAD: f32 = 900.0;

    let mut group = c.benchmark_group("spatial_partitioning");
    for n in [16usize, 64, 256, 1024, 4096] {
        let pts = points(n, SPREAD);

        // Sanity: the two must agree, or the comparison is meaningless.
        assert_eq!(
            brute_force(&pts, RADIUS),
            grid_bucketed(&pts, RADIUS, CELL),
            "grid and brute force disagree at n={n}"
        );

        group.bench_with_input(BenchmarkId::new("brute_force", n), &pts, |b, pts| {
            b.iter(|| black_box(brute_force(pts, RADIUS)))
        });
        group.bench_with_input(BenchmarkId::new("grid", n), &pts, |b, pts| {
            b.iter(|| black_box(grid_bucketed(pts, RADIUS, CELL)))
        });
    }
    group.finish();
}

/// The pair count the demo's HUD reports, which grows quadratically.
///
/// Cheap to compute, but worth pinning: it is the number the demo uses to claim
/// its savings, so it should not itself be a cost.
fn pair_counting(c: &mut Criterion) {
    c.bench_function("brute_pairs(10_000)", |b| {
        b.iter(|| black_box(brute_pairs(black_box(10_000))))
    });
}

/// How much headroom does the Snake simulation have?
///
/// `snake-lib` deliberately does no work per frame — only per tick — so this
/// answers "could the simulation ever be the bottleneck?" rather than tuning it.
fn snake_step(c: &mut Criterion) {
    let mut group = c.benchmark_group("snake_step");
    for (w, h) in [(20i32, 15i32), (80, 60)] {
        group.bench_function(BenchmarkId::new("step", format!("{w}x{h}")), |b| {
            b.iter_batched(
                || SnakeGame::new(w, h, 42),
                |mut game| {
                    // Steer in a slow spiral so the snake keeps eating and the
                    // body grows, rather than dying on the first wall.
                    for i in 0..200 {
                        if i % 7 == 0 {
                            game.queue_turn(match (i / 7) % 4 {
                                0 => Direction::Right,
                                1 => Direction::Down,
                                2 => Direction::Left,
                                _ => Direction::Up,
                            });
                        }
                        if game.is_over() {
                            break;
                        }
                        black_box(game.step());
                    }
                },
                criterion::BatchSize::SmallInput,
            )
        });
    }
    group.finish();
}

/// `winner()` rescans the whole board every call. How expensive is that as the
/// board grows?
///
/// Frontends call `status()` every frame, so this is the one place in
/// `tic-tac-toe-lib` where the straightforward implementation could bite.
fn tic_tac_toe_winner(c: &mut Criterion) {
    let mut group = c.benchmark_group("tic_tac_toe_winner");
    for size in [3usize, 9, 19] {
        // A full board with no winner is the worst case: every cell is scanned
        // along all four axes and nothing short-circuits.
        let mut game = TicTacToeGame::new(
            Board::new(size, size),
            vec![Player::new("X".into(), 'X'), Player::new("O".into(), 'O')],
            size + 1, // unreachable run length, so no game ever ends early
        );
        for row in 0..size {
            for col in 0..size {
                let _ = game.take_turn(row, col);
            }
        }

        group.bench_function(BenchmarkId::new("winner", format!("{size}x{size}")), |b| {
            b.iter(|| black_box(game.winner().is_some()))
        });
    }
    group.finish();
}

// ── flow-field-pathfinding ───────────────────────────────────────────────────
//
// The demo says agents look up a direction in O(1) "instead of running A* per
// agent". The first half is a statement about a array index and needs no
// defending. The second half is a comparison, and comparisons are what the
// `spatial-partitioning` measurement showed can be confidently wrong.
//
// The honest question is not whether one lookup beats one search — of course it
// does — but how many agents it takes before precomputing the whole field is
// worth it at all. Below that count the demo's own advice is to do the simple
// thing.
//
// Both sides use BFS, so the comparison isolates *sharing the work* rather than
// smuggling in a difference between search algorithms.

/// A grid with a diagonal scattering of walls, deterministic across runs.
fn walled_grid(width: usize, height: usize) -> Vec<bool> {
    let mut walls = vec![false; width * height];
    for y in 0..height {
        for x in 0..width {
            // A repeating broken-diagonal pattern: enough obstruction to make
            // the search do real work, never enough to seal the goal off.
            if (x + y * 3) % 11 == 0 && x % 4 != 0 {
                walls[y * width + x] = true;
            }
        }
    }
    walls[0] = false;
    walls[width * height - 1] = false;
    walls
}

/// One agent searching for itself: BFS from its own cell to the goal.
///
/// The baseline the demo says it beats. Returns the first step to take.
fn search_from(
    walls: &[bool],
    width: usize,
    height: usize,
    start: (usize, usize),
    goal: (usize, usize),
) -> Option<(i32, i32)> {
    let mut came: Vec<Option<(usize, usize)>> = vec![None; width * height];
    let mut seen = vec![false; width * height];
    let mut queue = std::collections::VecDeque::new();
    seen[start.1 * width + start.0] = true;
    queue.push_back(start);

    while let Some((cx, cy)) = queue.pop_front() {
        if (cx, cy) == goal {
            // Walk back to the step adjacent to the start.
            let (mut px, mut py) = (cx, cy);
            while let Some(prev) = came[py * width + px] {
                if prev == start {
                    return Some((px as i32 - start.0 as i32, py as i32 - start.1 as i32));
                }
                (px, py) = prev;
            }
            return None;
        }
        for (dx, dy) in [(1i32, 0i32), (-1, 0), (0, 1), (0, -1)] {
            let (nx, ny) = (cx as i32 + dx, cy as i32 + dy);
            if nx < 0 || ny < 0 || nx >= width as i32 || ny >= height as i32 {
                continue;
            }
            let (nx, ny) = (nx as usize, ny as usize);
            let ni = ny * width + nx;
            if walls[ni] || seen[ni] {
                continue;
            }
            seen[ni] = true;
            came[ni] = Some((cx, cy));
            queue.push_back((nx, ny));
        }
    }
    None
}

/// Agent starting cells, spread deterministically over the grid.
fn agent_cells(count: usize, width: usize, height: usize) -> Vec<(usize, usize)> {
    (0..count)
        .map(|i| ((i * 7) % width, (i * 13) % height))
        .collect()
}

fn bench_flow_field(c: &mut Criterion) {
    // The demo's own grid.
    let (width, height) = (flow_field_pathfinding::COLS, flow_field_pathfinding::ROWS);
    let walls = walled_grid(width, height);
    let goal = (width - 1, height - 1);

    let mut group = c.benchmark_group("flow_field_vs_search_per_agent");
    for count in [1usize, 2, 4, 8, 16, 50, 200, 1000] {
        let agents = agent_cells(count, width, height);

        // Worst case for the field: rebuilt every frame, as it is whenever the
        // goal moves. Steady state (no rebuild) is just the lookups.
        group.bench_function(BenchmarkId::new("flow_field_rebuilt", count), |b| {
            b.iter(|| {
                let field = compute_flow_field(black_box(&walls), width, height, goal);
                let mut sum = 0i32;
                for &(x, y) in &agents {
                    if let Some((dx, dy)) = field[y * width + x] {
                        sum += dx + dy;
                    }
                }
                black_box(sum)
            });
        });

        group.bench_function(BenchmarkId::new("search_per_agent", count), |b| {
            b.iter(|| {
                let mut sum = 0i32;
                for &start in &agents {
                    if let Some((dx, dy)) =
                        search_from(black_box(&walls), width, height, start, goal)
                    {
                        sum += dx + dy;
                    }
                }
                black_box(sum)
            });
        });
    }
    group.finish();
}

criterion_group!(
    benches,
    spatial_partitioning,
    pair_counting,
    snake_step,
    tic_tac_toe_winner,
    bench_flow_field,
);
criterion_main!(benches);
