# tic-tac-toe-lib

Engine-agnostic tic-tac-toe rules, with no dependencies.

Generalised past the classic game: any board size, any number of players, and a
configurable run length needed to win. `take_turn` is the only way to mutate the
board and validates every move, so no frontend can invent its own rules.

```rust
use tic_tac_toe_lib::{Board, GameStatus, Player, TicTacToeGame};

let mut game = TicTacToeGame::new(
    Board::new(3, 3),
    vec![Player::new("X".into(), 'X'), Player::new("O".into(), 'O')],
    3,
);
game.take_turn(0, 0).unwrap();

match game.status() {
    GameStatus::Won(p)     => println!("{} wins", p.name()),
    GameStatus::Draw       => println!("draw"),
    GameStatus::InProgress => println!("{}'s turn", game.current_player().name()),
}
```

Part of [tiny-rust-games](https://github.com/matthewscottconroy/tiny-rust-games),
where the same rules drive terminal, bracket-lib, Bevy and Godot frontends.
