//! System clipboard helpers (text + PNG image) via `arboard`.
//!
//! Best-effort and cross-platform: calls return a bool/Option so the TUI
//! degrades gracefully (a notification) when no clipboard is available (e.g. a
//! headless session or a machine with no display server).

/// Copy `text` to the system clipboard. Returns `true` on success.
pub fn copy_text(text: &str) -> bool {
    match arboard::Clipboard::new() {
        Ok(mut cb) => cb.set_text(text.to_string()).is_ok(),
        Err(_) => false,
    }
}

/// Read an image from the clipboard and encode it as PNG bytes. Returns `None`
/// when the clipboard holds no image or no clipboard is available. Used to
/// paste a screenshot straight onto a yak (artifact attach).
pub fn read_png() -> Option<Vec<u8>> {
    let mut cb = arboard::Clipboard::new().ok()?;
    let img = cb.get_image().ok()?;
    let mut out = Vec::new();
    {
        let mut enc = png::Encoder::new(&mut out, img.width as u32, img.height as u32);
        enc.set_color(png::ColorType::Rgba);
        enc.set_depth(png::BitDepth::Eight);
        let mut writer = enc.write_header().ok()?;
        writer.write_image_data(&img.bytes).ok()?;
    }
    Some(out)
}
