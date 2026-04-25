# Dialog System

A demo of a **branching conversation tree** with a character-by-character typing effect: the player reads a dialog node, waits for the text to finish typing (or skips it with SPACE), then selects a choice that advances to the next node. When the conversation ends, SPACE restarts it.

Press **SPACE** to advance or skip typing. Press **1**, **2**, or **3** to choose a response.

---

## What This Is Illustrating

Narrative conversation systems — the mechanism that drives NPC dialogue, quest exposition, tutorial prompts, and interactive storytelling in almost every RPG, adventure game, and visual novel. A dialog system decouples *story content* (the text and branching logic) from *presentation* (typing speed, UI layout, input handling).

---

## Game Design Principle: Structured Narrative with Player Agency

Linear dialog (the player just reads) is exposition. Branching dialog (the player chooses) is narrative agency. When players can make choices in a conversation, they feel ownership over the story — even if many paths converge at the same outcome.

**The dialog tree structure:**

Each node in the tree contains:
- **Speaker** — who is talking.
- **Text** — what they say.
- **Choices** — a list of `(button label, next node index)` pairs.

A node with no choices is a terminal node (end of conversation). A node with one choice is effectively linear text but still passes through the same code path. A node with multiple choices is a branch point.

The tree is just a `Vec<DialogNode>` indexed by `usize`. `advance_dialog(nodes, current_id, choice_index)` returns the next node index with no side effects. This pure function is the complete branching logic — one slice lookup and one tuple read.

**Typing effect:** Revealing text character by character serves two purposes. It paces the player's reading speed to match the character's "speaking" rhythm, and it creates a moment of tension (what is the guard going to say?) before the full sentence appears. Games that skip this effect feel more transactional; games that use it feel more like a conversation.

---

## How Bevy Achieves It

**Tree as a plain `Vec` with integer indices.** `DialogNode` uses `&'static str` for speaker and text and `&'static [(&'static str, usize)]` for choices — all compile-time literals. No heap allocation occurs during tree traversal. The `advance_dialog` function is a two-line slice lookup that returns `Option<usize>`, tested against out-of-range indices and terminal nodes.

**`DialogState` resource tracks runtime progress.** `node_id`, `char_index`, and `typing_timer` are stored in a single resource. `tick_typing` advances `char_index` at `CHARS_PER_SEC` characters per second using accumulated `delta_secs`; `handle_input` reads `char_index` to determine whether typing has finished before showing choices. The `finished: bool` flag avoids recomputing the comparison every frame.

**`visible_text(text, char_count) -> &str` slices safely.** Rust `&str` is a UTF-8 byte slice; you cannot index it by character position directly without risking a panic at a codepoint boundary. `visible_text` uses `char_indices().nth(char_count)` to find the correct byte offset, then slices. This makes the function correct for any Unicode text and is verified in tests.

**`DialogUi` enum component routes updates.** Each UI text entity carries a `DialogUi` variant: `Speaker`, `Body`, `Choice(usize)`, or `Hint`. The `update_ui` system queries all dialog text entities in one pass and uses a `match` on the variant to write the correct content. This avoids maintaining separate references to each text entity and keeps spawning and updating decoupled.

---

## Running

```
cargo run
```

Controls: **SPACE** — advance / skip typing · **1 / 2 / 3** — choose response.
