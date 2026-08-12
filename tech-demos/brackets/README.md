# Bracket-lib Tech Demos

Demos built with [bracket-lib](https://github.com/amethyst/bracket-lib) /
`bracket-terminal`, a simple ASCII/CP437 terminal rendering library well suited
to roguelikes.

## Running a demo

```bash
cd <demo-name>
cargo run
```

## Demos

| Demo | Concept |
|------|---------|
| `mouse-control` | Reading mouse position and clicks in a bracket-terminal console |
| `astar-pathfinding` | `BaseMap` + `Algorithm2D` for `a_star_search`; field-of-view; click to walk |

See also [`tic-tac-toe/tic-tac-toe-brackets`](../../tic-tac-toe/tic-tac-toe-brackets)
for a complete game built on bracket-lib.

## Troubleshooting

If a transitive dependency (`expat-sys`) fails to build with a CMake
compatibility error on CMake ≥ 4.0, build with:

```bash
CMAKE_POLICY_VERSION_MINIMUM=3.5 cargo run
```
