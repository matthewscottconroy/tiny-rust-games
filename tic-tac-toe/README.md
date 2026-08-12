# Tic-Tac-Toe

Tic-tac-toe implemented once as an engine-agnostic library, with a frontend per
engine — a working example of this repository's core heuristic: keep game logic
independent of any game library.

| Crate | What it is |
|-------|------------|
| `tic-tac-toe-lib` | The rules: board, turn order, move validation, win/draw detection, winner lookup. No engine dependencies, unit tested. |
| `tic-tac-toe-cli` | An interactive two-player game played in the terminal via stdin. |
| `tic-tac-toe-brackets` | A mouse-driven version rendered with [bracket-lib](https://github.com/amethyst/bracket-lib). |

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

**A frontend owns only input and drawing.** Neither frontend re-implements a
rule, and neither infers the winner from turn parity — they ask `status()`. If a
frontend ever needs to work something out about the rules for itself, that logic
belongs in the library instead.

That is what lets the same `TicTacToeGame` drive a stdin loop and a mouse-driven
console without the two ever disagreeing.

## Playing

```bash
# Terminal version — enter "row column" (zero-indexed), or "q" to quit
cd tic-tac-toe-cli
cargo run

# Bracket-lib (windowed) version — click a cell to play
cd tic-tac-toe-brackets
cargo run
```

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
```

The CLI's game loop is generic over `BufRead`/`Write`, so its tests play whole
games against in-memory buffers — no terminal required.
