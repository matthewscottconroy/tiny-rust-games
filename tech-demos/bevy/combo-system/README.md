# Combo System

A demo of **input-buffer sequence matching**: the game records the player's last several key presses and checks whether their tail matches any known combo pattern. When a match is found, the combo is announced and the buffer is cleared.

Enter directional inputs with **WASD** or arrow keys and watch for combo activations.

---

## What This Is Illustrating

Input buffering and temporal sequence recognition — the mechanism behind special moves in fighting games, magic incantations in RPGs, gesture shortcuts in mobile games, and cheat codes in retro games. The key insight is that game input is not just *what* the player presses but *in what order*.

---

## Game Design Principle: Expressive Input Through Sequences

Simple button presses (attack, jump, block) are easy to learn but shallow. Sequences add depth: the player can perform powerful or special actions only by demonstrating intentionality through a specific input order. This creates a skill ceiling — mastering sequences rewards practice — without complicating the basic control scheme.

**The input buffer pattern:**

A fixed-length queue stores the most recent N key presses (here, N = 8). On every key press the new input is appended and the oldest entry is discarded if the buffer is full. To detect a combo, the game checks whether the *tail* of the buffer — the last `len(pattern)` entries — matches the pattern exactly. Matching the tail rather than the head means combos fire the moment the final input is pressed, which feels immediate.

**Design tradeoffs:**
- A larger buffer gives the player more time between inputs (forgiving), but also makes accidental triggers more likely (noisy).
- Clearing the buffer on a successful match prevents combos from overlapping with each other.
- A timeout-based buffer (clearing inputs that are too old) is the production approach; this demo uses a fixed-size queue for clarity.

---

## How Bevy Achieves It

**Generic pure function.** `matches_sequence<T: PartialEq>(buffer: &[T], pattern: &[T]) -> bool` is completely generic — it works on any `PartialEq` type. In the game it is called with `KeyCode` slices; in unit tests it is called with `u8` slices. No Bevy types appear in the function at all.

**`VecDeque<KeyCode>` as the `InputBuffer` resource.** A double-ended queue provides O(1) push-back and pop-front — exactly what a ring buffer needs. The `trim_to_max` helper enforces the capacity limit after every insertion.

**Input capture runs before combo detection.** Bevy's `.chain()` scheduler (applied via `.add_systems(Update, (collect_input, check_combos, tick_flash, update_hud).chain())`) guarantees that `collect_input` runs before `check_combos` within the same frame. This means a combo triggered by the last input in a sequence is detected in the same frame it is pressed, not the next.

**Flash timer via a resource.** `ComboFlash` stores the combo name, its display colour, and a countdown timer. `tick_flash` decrements it each frame; `update_hud` reads it to set text content and alpha. The alpha fades linearly from 1.0 to 0.0 over `FLASH_DURATION` seconds by dividing `timer / FLASH_DURATION`, giving a smooth fade-out without a separate animation state machine.

---

## Running

```
cargo run
```

Controls: **Arrow keys / WASD** — enter inputs.  
Combos: `↑↑` Double Up · `↑→↓←` Spin · `←→←` Zigzag · `↓↓` Dive.
