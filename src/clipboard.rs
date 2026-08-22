//! System clipboard helpers via `arboard`.
//!
//! Best-effort and cross-platform: calls return a bool so the TUI degrades
//! gracefully (a notification) when no clipboard is available (e.g. a headless
//! session or a machine with no display server). Image/PNG support will land
//! alongside the artifact-attach work.

/// Copy `text` to the system clipboard. Returns `true` on success.
pub fn copy_text(text: &str) -> bool {
    match arboard::Clipboard::new() {
        Ok(mut cb) => cb.set_text(text.to_string()).is_ok(),
        Err(_) => false,
    }
}
