//! HTTP Request GDExtension demo — drives Godot's built-in `HttpRequest` node
//! from Rust, connecting the `request_completed` signal and parsing the response.
//!
//! Teaches:
//!
//! - Fetching an `HttpRequest` child via `get_node_as` in `ready()`.
//! - Connecting the `request_completed` signal to a Rust `#[func]` method.
//! - Sending a GET request with `http.request(url)`.
//! - Converting `PackedByteArray` body bytes to a UTF-8 `String`.
//! - Tracking in-flight state to avoid concurrent requests.
//! - Pure response-parsing helpers fully covered by unit tests.

use godot::classes::{HttpRequest, INode, Label, Node};
use godot::prelude::*;

// ─── Extension entry point ────────────────────────────────────────────────────

struct HttpRequestExt;

#[gdextension]
unsafe impl ExtensionLibrary for HttpRequestExt {}

// ─── Pure functions ───────────────────────────────────────────────────────────

/// Returns a short human-readable label for an HTTP status code.
///
/// # Examples
/// ```
/// assert_eq!(http_request::parse_status_from_response(200), "OK");
/// assert_eq!(http_request::parse_status_from_response(404), "Not Found");
/// assert_eq!(http_request::parse_status_from_response(500), "Server Error");
/// assert_eq!(http_request::parse_status_from_response(0), "Unknown");
/// ```
pub fn parse_status_from_response(code: i64) -> &'static str {
    match code {
        200 => "OK",
        201 => "Created",
        204 => "No Content",
        301 | 302 => "Redirect",
        400 => "Bad Request",
        401 => "Unauthorized",
        403 => "Forbidden",
        404 => "Not Found",
        500..=599 => "Server Error",
        _ => "Unknown",
    }
}

/// Returns up to `max_len` bytes of `body` as a `String`, appending `"…"` if
/// truncated.
///
/// # Examples
/// ```
/// assert_eq!(http_request::truncate_body("hello world", 5), "hello…");
/// assert_eq!(http_request::truncate_body("hi", 10), "hi");
/// ```
pub fn truncate_body(body: &str, max_len: usize) -> String {
    if body.len() <= max_len {
        body.to_string()
    } else {
        // Truncate at a char boundary to avoid panic.
        let truncated = &body[..body
            .char_indices()
            .take_while(|(i, _)| *i < max_len)
            .last()
            .map(|(i, c)| i + c.len_utf8())
            .unwrap_or(0)];
        format!("{truncated}…")
    }
}

/// Formats a status string for the in-flight / completed state.
///
/// # Examples
/// ```
/// let s = http_request::format_request_state(true, 0);
/// assert!(s.contains("In flight"));
/// let s = http_request::format_request_state(false, 200);
/// assert!(s.contains("200"));
/// ```
pub fn format_request_state(in_flight: bool, code: i64) -> String {
    if in_flight {
        "In flight…".to_string()
    } else if code == 0 {
        "Idle".to_string()
    } else {
        format!("Done — {} ({})", code, parse_status_from_response(code))
    }
}

/// Returns `true` for 2xx HTTP status codes.
///
/// # Examples
/// ```
/// assert!(http_request::is_success_code(200));
/// assert!(http_request::is_success_code(201));
/// assert!(!http_request::is_success_code(404));
/// assert!(!http_request::is_success_code(0));
/// ```
pub fn is_success_code(code: i64) -> bool {
    (200..300).contains(&code)
}

// ─── HttpDemo node ────────────────────────────────────────────────────────────

/// A `Node` that uses Godot's `HttpRequest` child to fetch a URL and display
/// the response on a `Label`.
///
/// Expected scene layout:
/// ```text
/// HttpDemo (this class, Node)
/// ├── HttpRequest
/// └── Label
/// ```
#[derive(GodotClass)]
#[class(base=Node)]
pub struct HttpDemo {
    /// URL to fetch when `send_request()` is called.
    #[export]
    url: GString,

    /// Whether a request is currently in flight.
    request_in_flight: bool,

    /// The last received response body (first 200 chars).
    last_response: String,

    /// The last HTTP status code received.
    last_code: i64,

    base: Base<Node>,
}

#[godot_api]
impl INode for HttpDemo {
    fn init(base: Base<Node>) -> Self {
        Self {
            url: GString::from("https://httpbin.org/get"),
            request_in_flight: false,
            last_response: String::new(),
            last_code: 0,
            base,
        }
    }

    fn ready(&mut self) {
        let callable = self.base().callable("on_request_completed");
        let mut http = self.base().get_node_as::<HttpRequest>("HttpRequest");
        http.connect("request_completed", &callable);

        godot_print!("[HttpDemo] Ready — url={}", self.url);
    }
}

#[godot_api]
impl HttpDemo {
    /// Sends an HTTP GET request to `self.url`. Does nothing if a request is
    /// already in flight.
    #[func]
    pub fn send_request(&mut self) {
        if self.request_in_flight {
            godot_print!("[HttpDemo] Request already in flight — ignoring.");
            return;
        }

        let url = self.url.to_string();

        if let Some(mut label) = self.base().try_get_node_as::<Label>("Label") {
            label.set_text("Requesting\u{2026}");
        }

        let mut http = self.base().get_node_as::<HttpRequest>("HttpRequest");
        let _err = http.request(url.as_str());

        self.request_in_flight = true;
        godot_print!("[HttpDemo] Sending request to {}", self.url);
    }

    /// Signal handler for `request_completed`. Stores the body and updates the
    /// label. Signature must match Godot's signal: (result, response_code,
    /// headers, body).
    #[func]
    pub fn on_request_completed(
        &mut self,
        _result: i64,
        response_code: i64,
        _headers: PackedStringArray,
        body: PackedByteArray,
    ) {
        self.request_in_flight = false;
        self.last_code = response_code;

        let body_str = String::from_utf8_lossy(body.as_slice()).to_string();
        let preview = truncate_body(&body_str, 200);
        self.last_response = preview.clone();

        let state = format_request_state(false, response_code);
        let display = format!("{state}\n{preview}");

        if let Some(mut label) = self.base().try_get_node_as::<Label>("Label") {
            label.set_text(display.as_str());
        }

        godot_print!(
            "[HttpDemo] Response {} — {} bytes",
            response_code,
            body.len()
        );
    }

    /// Returns the last response body preview as a `GString`.
    #[func]
    pub fn get_last_response(&self) -> GString {
        GString::from(self.last_response.as_str())
    }
}

// ─── Tests ────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    // parse_status_from_response ───────────────────────────────────────────────

    #[test]
    fn status_200_is_ok() {
        assert_eq!(parse_status_from_response(200), "OK");
    }

    #[test]
    fn status_404_is_not_found() {
        assert_eq!(parse_status_from_response(404), "Not Found");
    }

    #[test]
    fn status_500_is_server_error() {
        assert_eq!(parse_status_from_response(500), "Server Error");
    }

    #[test]
    fn status_0_is_unknown() {
        assert_eq!(parse_status_from_response(0), "Unknown");
    }

    #[test]
    fn status_201_is_created() {
        assert_eq!(parse_status_from_response(201), "Created");
    }

    // truncate_body ────────────────────────────────────────────────────────────

    #[test]
    fn truncate_body_short_string_unchanged() {
        assert_eq!(truncate_body("hi", 10), "hi");
    }

    #[test]
    fn truncate_body_exact_length_unchanged() {
        assert_eq!(truncate_body("hello", 5), "hello");
    }

    #[test]
    fn truncate_body_long_string_gets_ellipsis() {
        let result = truncate_body("hello world", 5);
        assert!(result.ends_with('…'), "expected ellipsis: {result}");
        assert!(result.starts_with("hello"), "expected 'hello': {result}");
    }

    #[test]
    fn truncate_body_empty_string() {
        assert_eq!(truncate_body("", 10), "");
    }

    // format_request_state ─────────────────────────────────────────────────────

    #[test]
    fn format_request_state_in_flight() {
        let s = format_request_state(true, 0);
        assert!(s.contains("In flight"), "got: {s}");
    }

    #[test]
    fn format_request_state_idle() {
        let s = format_request_state(false, 0);
        assert_eq!(s, "Idle");
    }

    #[test]
    fn format_request_state_done_with_code() {
        let s = format_request_state(false, 200);
        assert!(s.contains("200"), "got: {s}");
        assert!(s.contains("OK"), "got: {s}");
    }

    // is_success_code ──────────────────────────────────────────────────────────

    #[test]
    fn is_success_200() {
        assert!(is_success_code(200));
    }

    #[test]
    fn is_success_201() {
        assert!(is_success_code(201));
    }

    #[test]
    fn not_success_404() {
        assert!(!is_success_code(404));
    }

    #[test]
    fn not_success_0() {
        assert!(!is_success_code(0));
    }

    #[test]
    fn not_success_500() {
        assert!(!is_success_code(500));
    }
}
