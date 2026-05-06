//! Rich Text Label demo — BBCode-style formatted text driven from Rust using
//! the `RichTextLabel` push/pop API.
//!
//! Teaches: accessing a child node with `get_node_as`, calling `push_color`,
//! `push_bold`, `append_text`, `pop`, `newline`, and `clear` on a
//! `RichTextLabel` child; extracting pure text-processing helpers that are
//! fully unit-testable without Godot.

use godot::classes::{INode, Node, RichTextLabel};
use godot::prelude::*;

// ---------------------------------------------------------------------------
// Extension entry-point
// ---------------------------------------------------------------------------

struct RichTextLabelExtension;
#[gdextension]
unsafe impl ExtensionLibrary for RichTextLabelExtension {}

// ---------------------------------------------------------------------------
// RichTextDemo node
// ---------------------------------------------------------------------------

/// Scene root that drives a child `RichTextLabel` with colored, bold, and
/// normal text segments — all pushed from Rust via the push/pop API.
#[derive(GodotClass)]
#[class(base=Node)]
pub struct RichTextDemo {
    base: Base<Node>,
}

#[godot_api]
impl INode for RichTextDemo {
    fn init(base: Base<Node>) -> Self {
        Self { base }
    }

    fn ready(&mut self) {
        // Demonstrate several styled segments on startup.
        self.append_colored(
            GString::from("Rich Text Demo"),
            1.0,
            0.8,
            0.0,
        );
        self.newline();
        self.append_bold(GString::from("Bold subtitle — gdext 0.5"));
        self.newline();
        self.append_normal(GString::from(
            "Normal body text: push/pop controls formatting scope.",
        ));
        self.newline();
        self.append_colored(GString::from("Red accent"), 1.0, 0.2, 0.2);
        self.append_normal(GString::from(" — "));
        self.append_colored(GString::from("Green accent"), 0.2, 1.0, 0.2);
        self.append_normal(GString::from(" — "));
        self.append_colored(GString::from("Blue accent"), 0.3, 0.5, 1.0);
        godot_print!("[RichTextDemo] ready — label populated");
    }
}

#[godot_api]
impl RichTextDemo {
    /// Appends `text` in the given RGB colour, then pops the colour context.
    #[func]
    pub fn append_colored(&mut self, text: GString, r: f32, g: f32, b: f32) {
        let mut label = self.base().get_node_as::<RichTextLabel>("RichTextLabel");
        label.push_color(Color::from_rgb(r, g, b));
        label.append_text(&text);
        label.pop();
    }

    /// Appends `text` in bold, then pops the bold context.
    #[func]
    pub fn append_bold(&mut self, text: GString) {
        let mut label = self.base().get_node_as::<RichTextLabel>("RichTextLabel");
        // push_bold() pushes the bold font variant; pop() restores normal.
        label.push_bold();
        label.append_text(&text);
        label.pop();
    }

    /// Appends `text` with no additional formatting.
    #[func]
    pub fn append_normal(&mut self, text: GString) {
        let mut label = self.base().get_node_as::<RichTextLabel>("RichTextLabel");
        label.append_text(&text);
    }

    /// Inserts a line break into the label.
    #[func]
    pub fn newline(&mut self) {
        let mut label = self.base().get_node_as::<RichTextLabel>("RichTextLabel");
        label.newline();
    }

    /// Clears all content from the label.
    #[func]
    pub fn clear_text(&mut self) {
        let mut label = self.base().get_node_as::<RichTextLabel>("RichTextLabel");
        label.clear();
    }
}

// ---------------------------------------------------------------------------
// Pure helper functions
// ---------------------------------------------------------------------------

/// Truncates `text` to at most `max_len` characters. Returns the original
/// slice unchanged if it is already within the limit.
///
/// # Examples
/// ```
/// assert_eq!(rich_text_label::truncate_text("hello world", 5), "hello");
/// assert_eq!(rich_text_label::truncate_text("hi", 10), "hi");
/// ```
pub fn truncate_text(text: &str, max_len: usize) -> &str {
    if text.len() <= max_len {
        text
    } else {
        // Respect character boundaries.
        match text.char_indices().nth(max_len) {
            Some((idx, _)) => &text[..idx],
            None => text,
        }
    }
}

/// Counts the number of whitespace-delimited words in `text`.
///
/// # Examples
/// ```
/// assert_eq!(rich_text_label::word_count("hello world"), 2);
/// assert_eq!(rich_text_label::word_count("  "), 0);
/// assert_eq!(rich_text_label::word_count("one"), 1);
/// ```
pub fn word_count(text: &str) -> usize {
    text.split_whitespace().count()
}

/// Converts linear RGB float components (each in `[0.0, 1.0]`) to a
/// CSS-style hex string like `"#ff8000"`.
///
/// Components are clamped before conversion.
///
/// # Examples
/// ```
/// assert_eq!(rich_text_label::rgb_to_hex(1.0, 0.0, 0.0), "#ff0000");
/// assert_eq!(rich_text_label::rgb_to_hex(0.0, 1.0, 0.0), "#00ff00");
/// assert_eq!(rich_text_label::rgb_to_hex(0.0, 0.0, 0.0), "#000000");
/// ```
pub fn rgb_to_hex(r: f32, g: f32, b: f32) -> String {
    let ri = (r.clamp(0.0, 1.0) * 255.0).round() as u8;
    let gi = (g.clamp(0.0, 1.0) * 255.0).round() as u8;
    let bi = (b.clamp(0.0, 1.0) * 255.0).round() as u8;
    format!("#{:02x}{:02x}{:02x}", ri, gi, bi)
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    // truncate_text -----------------------------------------------------------

    #[test]
    fn truncate_text_longer_than_limit() {
        assert_eq!(truncate_text("hello world", 5), "hello");
    }

    #[test]
    fn truncate_text_exactly_at_limit() {
        assert_eq!(truncate_text("hello", 5), "hello");
    }

    #[test]
    fn truncate_text_shorter_than_limit() {
        assert_eq!(truncate_text("hi", 10), "hi");
    }

    #[test]
    fn truncate_text_empty_string() {
        assert_eq!(truncate_text("", 5), "");
    }

    #[test]
    fn truncate_text_zero_limit() {
        assert_eq!(truncate_text("hello", 0), "");
    }

    // word_count --------------------------------------------------------------

    #[test]
    fn word_count_two_words() {
        assert_eq!(word_count("hello world"), 2);
    }

    #[test]
    fn word_count_only_whitespace() {
        assert_eq!(word_count("   "), 0);
    }

    #[test]
    fn word_count_single_word() {
        assert_eq!(word_count("one"), 1);
    }

    #[test]
    fn word_count_empty_string() {
        assert_eq!(word_count(""), 0);
    }

    #[test]
    fn word_count_multiple_spaces_between_words() {
        assert_eq!(word_count("a   b   c"), 3);
    }

    // rgb_to_hex --------------------------------------------------------------

    #[test]
    fn rgb_to_hex_red() {
        assert_eq!(rgb_to_hex(1.0, 0.0, 0.0), "#ff0000");
    }

    #[test]
    fn rgb_to_hex_green() {
        assert_eq!(rgb_to_hex(0.0, 1.0, 0.0), "#00ff00");
    }

    #[test]
    fn rgb_to_hex_black() {
        assert_eq!(rgb_to_hex(0.0, 0.0, 0.0), "#000000");
    }

    #[test]
    fn rgb_to_hex_white() {
        assert_eq!(rgb_to_hex(1.0, 1.0, 1.0), "#ffffff");
    }

    #[test]
    fn rgb_to_hex_clamps_above_one() {
        assert_eq!(rgb_to_hex(2.0, 0.0, 0.0), "#ff0000");
    }

    #[test]
    fn rgb_to_hex_clamps_below_zero() {
        assert_eq!(rgb_to_hex(-1.0, 0.5, 0.0), "#008000");
    }
}
