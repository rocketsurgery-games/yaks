//! Render a headless frame to a self-contained, LLM-legible **SVG**.
//!
//! The text snapshot ([`crate::SnapshotEncoder`]) is the cheap, diffable channel
//! and deliberately carries no colour. This module is the *visual* counterpart:
//! it walks a ratatui [`Buffer`] cell-by-cell into a standalone SVG that embeds
//! anywhere (docs, a browser, GitHub markdown) and rasterises cleanly to PNG.
//!
//! The markup is deliberately lean and row-ordered so the SVG *source* reads
//! top-to-bottom, left-to-right, close to the text snapshot:
//!
//! - shared attributes (`font-*`, `white-space:pre`, the default `fill`) are
//!   hoisted to the root / a wrapping `<g>` and inherit;
//! - every distinct colour or modifier becomes a short `<style>` class rather
//!   than a repeated inline `fill=…`/`font-weight=…`;
//! - contiguous same-background cells coalesce into one `<rect>`;
//! - each terminal row is a single `<text>` whose runs are `<tspan>`s pinned by
//!   an absolute `x`. Chrome honours `x` on a tspan but ignores `textLength`
//!   there, and a row-level `textLength` would stretch each row independently
//!   and break column alignment — so we pin run starts and break a run after any
//!   wide glyph (emoji) so it can't shove the rest of the row out of column. The
//!   only cost is sub-pixel drift *within* a long run; columns stay put.

use ratatui::buffer::Buffer;
use ratatui::style::{Color, Modifier};
use unicode_width::UnicodeWidthStr;

use crate::{HeadlessApp, render_to_buffer};

// -- geometry -------------------------------------------------------------

const CW: f32 = 8.4; // cell width  (px)
const CH: f32 = 18.0; // cell height (px)
const PAD: f32 = 10.0; // gutter around the grid
const FONT: f32 = 14.0; // glyph size
const BASE: f32 = 13.5; // text baseline within a cell row

/// Tomorrow-Night-ish palette so the SVG reads like a real dark terminal.
const BG: &str = "#1d1f21";
const FG: &str = "#c5c8c6";

/// Render a [`HeadlessApp`] at the given size straight to an SVG document.
pub fn render_to_svg<A: HeadlessApp>(app: &A, w: u16, h: u16) -> String {
    buffer_to_svg(&render_to_buffer(app, w, h))
}

// -- color mapping --------------------------------------------------------

/// Map a ratatui [`Color`] to a hex string. `Reset` returns `None` (meaning
/// "use the surface default" — no bg rect, default fg).
fn hex(c: Color) -> Option<String> {
    let s = match c {
        Color::Reset => return None,
        Color::Black => "#1d1f21",
        Color::Red => "#cc6666",
        Color::Green => "#b5bd68",
        Color::Yellow => "#f0c674",
        Color::Blue => "#81a2be",
        Color::Magenta => "#b294bb",
        Color::Cyan => "#8abeb7",
        Color::Gray => "#c5c8c6",
        Color::DarkGray => "#969896",
        Color::LightRed => "#d54e53",
        Color::LightGreen => "#b9ca4a",
        Color::LightYellow => "#e7c547",
        Color::LightBlue => "#7aa6da",
        Color::LightMagenta => "#c397d8",
        Color::LightCyan => "#70c0b1",
        Color::White => "#eaeaea",
        Color::Rgb(r, g, b) => return Some(format!("#{r:02x}{g:02x}{b:02x}")),
        Color::Indexed(i) => return Some(indexed(i)),
    };
    Some(s.to_string())
}

/// Resolve an xterm-256 palette index to hex (16 base + 6×6×6 cube + grayscale).
fn indexed(i: u8) -> String {
    const BASE16: [&str; 16] = [
        "#1d1f21", "#cc6666", "#b5bd68", "#f0c674", "#81a2be", "#b294bb", "#8abeb7", "#c5c8c6",
        "#969896", "#d54e53", "#b9ca4a", "#e7c547", "#7aa6da", "#c397d8", "#70c0b1", "#eaeaea",
    ];
    match i {
        0..=15 => BASE16[i as usize].to_string(),
        16..=231 => {
            let n = i - 16;
            let steps = [0u32, 95, 135, 175, 215, 255];
            let r = steps[(n / 36) as usize];
            let g = steps[((n / 6) % 6) as usize];
            let b = steps[(n % 6) as usize];
            format!("#{r:02x}{g:02x}{b:02x}")
        }
        232..=255 => {
            let v = 8 + (i as u32 - 232) * 10;
            format!("#{v:02x}{v:02x}{v:02x}")
        }
    }
}

fn xml_escape(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
}

// -- style classes --------------------------------------------------------

/// Deduplicated CSS classes: distinct backgrounds and text styles each earn a
/// short class name (`b0`, `s0`, …) emitted once in a `<style>` block, so the
/// per-element markup carries a 2–3 char `class=` instead of a full inline
/// `fill=…`/`font-weight=…`/`opacity=…` string.
#[derive(Default)]
struct Classes {
    bg: Vec<String>,   // index -> fill hex      ; class name = b{index}
    text: Vec<String>, // index -> CSS decl body ; class name = s{index}
}

impl Classes {
    fn bg_class(&mut self, fill: &str) -> String {
        let i = self.bg.iter().position(|f| f == fill).unwrap_or_else(|| {
            self.bg.push(fill.to_string());
            self.bg.len() - 1
        });
        format!("b{i}")
    }

    fn text_class(&mut self, decl: &str) -> String {
        let i = self.text.iter().position(|d| d == decl).unwrap_or_else(|| {
            self.text.push(decl.to_string());
            self.text.len() - 1
        });
        format!("s{i}")
    }

    fn style_block(&self) -> String {
        // `white-space:pre` (a CSS property, so it *inherits* — unlike
        // `xml:space`, which Chrome does not inherit from the root) keeps the
        // multi-space gaps that column alignment depends on.
        let mut s = String::from("<style>\ntext{white-space:pre}\n");
        for (i, fill) in self.bg.iter().enumerate() {
            s.push_str(&format!(".b{i}{{fill:{fill}}}\n"));
        }
        for (i, decl) in self.text.iter().enumerate() {
            s.push_str(&format!(".s{i}{{{decl}}}\n"));
        }
        s.push_str("</style>\n");
        s
    }
}

/// Build the CSS declaration body for a text run's style. Returns `None` for the
/// surface default (fg == [`FG`], no modifiers) — those runs need no class and
/// inherit `fill` from the wrapping `<g>`.
fn text_decl(fill: &str, modi: Modifier) -> Option<String> {
    let mut parts = Vec::new();
    if fill != FG {
        parts.push(format!("fill:{fill}"));
    }
    if modi.contains(Modifier::BOLD) {
        parts.push("font-weight:bold".into());
    }
    if modi.contains(Modifier::ITALIC) {
        parts.push("font-style:italic".into());
    }
    if modi.contains(Modifier::DIM) {
        parts.push("opacity:.6".into());
    }
    if modi.contains(Modifier::UNDERLINED) {
        parts.push("text-decoration:underline".into());
    }
    (!parts.is_empty()).then(|| parts.join(";"))
}

// -- buffer -> svg --------------------------------------------------------

/// Serialize one rendered [`Buffer`] to a standalone SVG document. See the
/// module docs for the layout strategy.
pub fn buffer_to_svg(buf: &Buffer) -> String {
    let area = buf.area;
    let (cols, rows) = (area.width, area.height);
    let w = PAD * 2.0 + cols as f32 * CW;
    let h = PAD * 2.0 + rows as f32 * CH;

    let mut classes = Classes::default();

    // Background: coalesce each maximal run of same-color cells into one rect.
    let mut rects = String::new();
    for y in 0..rows {
        let mut x = 0u16;
        while x < cols {
            let Some(c) = hex(buf[(x, y)].bg) else {
                x += 1;
                continue;
            };
            let start = x;
            while x < cols && hex(buf[(x, y)].bg).as_deref() == Some(c.as_str()) {
                x += 1;
            }
            let cls = classes.bg_class(&c);
            let px = PAD + start as f32 * CW;
            let py = PAD + y as f32 * CH;
            let width = (x - start) as f32 * CW + 0.5;
            rects.push_str(&format!(
                "<rect class=\"{cls}\" x=\"{px:.1}\" y=\"{py:.1}\" \
                 width=\"{width:.1}\" height=\"{CH:.1}\"/>\n"
            ));
        }
    }

    // Text: one `<text>` per row, each run a `<tspan>` pinned by absolute `x`;
    // break a run after any wide glyph so the next column re-pins.
    let mut texts = String::new();
    for y in 0..rows {
        let mut body = String::new();
        let mut any = false;
        let mut x = 0u16;
        while x < cols {
            let cell = &buf[(x, y)];
            if cell.symbol().trim().is_empty() && cell.modifier.is_empty() {
                x += 1;
                continue;
            }
            let fg = cell.fg;
            let modi = cell.modifier;
            let start = x;
            let mut text = String::new();
            while x < cols {
                let c = &buf[(x, y)];
                if c.fg != fg || c.modifier != modi {
                    break;
                }
                let s = c.symbol();
                let sym = if s.is_empty() { " " } else { s };
                text.push_str(sym);
                x += 1;
                if sym.width() > 1 {
                    break; // re-pin the next column after a wide glyph
                }
            }
            let trimmed = text.trim_end();
            if trimmed.is_empty() {
                continue;
            }
            any = true;
            let px = PAD + start as f32 * CW;
            let fill = hex(fg).unwrap_or_else(|| FG.to_string());
            let class_attr = text_decl(&fill, modi)
                .map(|d| format!(" class=\"{}\"", classes.text_class(&d)))
                .unwrap_or_default();
            body.push_str(&format!(
                "<tspan x=\"{px:.1}\"{class_attr}>{}</tspan>",
                xml_escape(trimmed)
            ));
        }
        if any {
            let py = PAD + y as f32 * CH + BASE;
            texts.push_str(&format!("<text y=\"{py:.1}\">{body}</text>\n"));
        }
    }

    // Assemble: shared text attrs live on the root; default fill on the `<g>`.
    let mut out = String::new();
    out.push_str(&format!(
        "<svg xmlns=\"http://www.w3.org/2000/svg\" width=\"{w:.0}\" height=\"{h:.0}\" \
         viewBox=\"0 0 {w:.0} {h:.0}\" font-family=\"ui-monospace,'SF Mono','DejaVu Sans Mono',\
         Menlo,Consolas,monospace\" font-size=\"{FONT}\">\n"
    ));
    out.push_str(&classes.style_block());
    out.push_str(&format!(
        "<rect x=\"0\" y=\"0\" width=\"{w:.0}\" height=\"{h:.0}\" rx=\"8\" fill=\"{BG}\"/>\n"
    ));
    out.push_str(&rects);
    out.push_str(&format!("<g fill=\"{FG}\">\n{texts}</g>\n"));
    out.push_str("</svg>\n");
    out
}
