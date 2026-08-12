# snake-lib

Engine-agnostic Snake rules, with no dependencies.

The library owns the rules and never the clock: `SnakeGame::step()` advances the
world by exactly one tick and never reads a clock, which keeps it usable from
any engine and deterministic enough to property-test. `Ticker` converts frame
time into whole steps, so a Bevy system, a Godot `process(delta)` callback and a
terminal loop all drive it identically.

```rust
use snake_lib::{Direction, SnakeGame, Ticker};

let mut game = SnakeGame::new(20, 15, 42);
let mut ticker = Ticker::new(8.0);   // eight steps per second

game.queue_turn(Direction::Down);
for _ in 0..ticker.accumulate(0.25) { // a 250 ms frame is worth two steps
    game.step();
}
```

Part of [tiny-rust-games](https://github.com/matthewscottconroy/tiny-rust-games),
where the same rules drive Bevy and Godot frontends.
