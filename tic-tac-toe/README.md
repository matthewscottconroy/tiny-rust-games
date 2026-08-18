# Tic-Tac-Toe

Tic-tac-toe implemented once as an engine-agnostic library, with a frontend per
engine — a working example of this repository's core heuristic: keep game logic
independent of any game library.

| Crate | What it is |
|-------|------------|
| `tic-tac-toe-lib` | The rules: board, turn order, move validation, win/draw detection, winner lookup. No engine dependencies, unit tested. |
| `tic-tac-toe-cli` | An interactive two-player game played in the terminal via stdin. |
| `tic-tac-toe-brackets` | A mouse-driven version rendered with [bracket-lib](https://github.com/amethyst/bracket-lib). |
| `tic-tac-toe-bevy` | The same game in [Bevy](https://bevyengine.org/), driven by ECS systems. |
| `tic-tac-toe-godot` | The same game in [Godot](https://godotengine.org/), driven by scene-tree callbacks. |
| `tic-tac-toe-web` | The same game with **no engine at all**: a 2D canvas and a click handler, via `wasm-bindgen`. |

## The frontend with no engine

`tic-tac-toe-web` is the one that makes the argument hardest to wave away. A
rules crate that works under four engines might just be a crate that abstracts
over four engines. This one works under *none*: it draws to a raw
`CanvasRenderingContext2d` and installs a single click listener, and
`tic-tac-toe-lib` did not change by a line to allow it.

It also puts a number on what an engine costs. The same game, the same rules:

| Build | WebAssembly shipped |
|-------|--------------------|
| `tic-tac-toe-bevy` | ~25 MB |
| `tic-tac-toe-web` | 28 KB |

That is not an argument against Bevy — the 25 MB buys an ECS, a renderer, asset
loading, input abstraction and a great deal more, and Snake and Breakout use
things this page has no equivalent for. It is an argument for knowing which one
a given game actually needs, which you can only weigh if the rules are not
welded to either answer.

Note what *is* absent from `tic-tac-toe-web`: there is no game loop. A
turn-based game does not need one — the browser calls the listener when
something is clicked and nothing changes in between. Snake and Breakout would
need `requestAnimationFrame` and a `Ticker`. The shape of the frontend follows
from the shape of the game, not from the engine.

Five frontends over one set of rules. They span three genuinely different
architectures — a blocking `read_line` loop, an ECS where systems run every
frame, and a scene tree that calls you back — which is the point: if the
boundary were wrong, one of them would have forced a rule out of the library.

The library supports arbitrary board sizes, any number of players, and a
configurable run length needed to win (e.g. 4-in-a-row on a 5×5 board).

## Where the boundary sits

The split is the point of this example, so it is worth being precise about it.

**The library owns every rule.** `take_turn` is the only way to mutate the
board, and it returns `Result<(), MoveError>` — rejecting moves off the board,
onto an occupied cell, or after the game has ended. Outcomes come from
`status()`, which returns one exhaustive value:

```rust
match game.status() {
    GameStatus::Won(player) => println!("{} wins!", player.name()),
    GameStatus::Draw        => println!("It's a draw!"),
    GameStatus::InProgress  => { /* keep playing */ }
}
```

**A frontend owns only input and drawing.** No frontend re-implements a rule,
and none infers the winner from turn parity — they all ask `status()`. If a
frontend ever needs to work something out about the rules for itself, that logic
belongs in the library instead.

Each frontend's whole job reduces to three things: map an input to a cell, call
`take_turn`, and render what `status()` reports. The Bevy and Godot versions
call `take_turn` without pre-checking anything, because a rejected move leaves
the game untouched — validation is the library's job, not theirs.

That is what lets one `TicTacToeGame` drive a terminal, an ASCII console, an
ECS, and a scene tree without any of them ever disagreeing.

## Playing

```bash
# Terminal version — enter "row column" (zero-indexed), or "q" to quit
cd tic-tac-toe-cli
cargo run

# Bracket-lib (windowed) version — click a cell to play
cd tic-tac-toe-brackets
cargo run

# Bevy version — click a cell to play, R to restart
cargo run --manifest-path ../tech-demos/bevy/Cargo.toml -p tic-tac-toe-bevy

# Godot version — build the extension, then open the project in Godot 4.3+
cd tic-tac-toe-godot && cargo build && godot4 --editor .
```

`tic-tac-toe-bevy` is a member of the Bevy demo workspace (see its `Cargo.toml`)
so Bevy compiles once for the whole repository rather than a second time for
this game. That is why it is run through that workspace's manifest.

The terminal version draws rulers along the top and left edges so the
coordinates to type can be read straight off the board:

```
    0 1 2
  0  | |
    -+-+-
  1  |X|
    -+-+-
  2  | |

Olive's turn (O). Enter "row column", or "q" to quit.
>
```

## Testing

```bash
cargo test --manifest-path tic-tac-toe-lib/Cargo.toml       # rules
cargo test --manifest-path tic-tac-toe-cli/Cargo.toml       # input parsing + scripted games
cargo test --manifest-path tic-tac-toe-brackets/Cargo.toml  # screen/board coordinate mapping
cargo test --manifest-path tic-tac-toe-godot/Cargo.toml     # click/cell mapping, status text
cargo test --manifest-path ../tech-demos/bevy/Cargo.toml -p tic-tac-toe-bevy
```

The CLI's game loop is generic over `BufRead`/`Write`, so its tests play whole
games against in-memory buffers — no terminal required.
