//! Doc "screenshots": drive the headless [`App`] to a handful of representative
//! states, render each to a ratatui [`Buffer`], and serialize the buffer to a
//! self-contained **color SVG** that embeds directly in `docs/tui.md`.
//!
//! This is not a normal test — it is a code-gated asset generator. It lives in a
//! `#[cfg(test)]` child module of `tui` (like `headless`) so it can construct an
//! `App` and reach the same private drive path the snapshot tests use, without a
//! `lib` target (yaks is a bin crate, so an `examples/` file cannot `use` it).
//!
//! Regenerate the assets with:
//!
//! ```sh
//! cargo test -p yaks docshots -- --ignored
//! ```
//!
//! The renderer walks the buffer cell-by-cell: a rounded backing rect for the
//! terminal, one `<rect>` per non-default cell background, and one `<text>` run
//! per maximal same-style span on each row (with `textLength` pinning each run
//! to an exact column multiple so alignment never drifts with the viewer font).

use ratatui::buffer::Buffer;
use ratatui::crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use ratatui::style::{Color, Modifier};
use toque::{HeadlessApp, render_to_buffer};

use crate::model::{Status, Task};
use crate::tui::{App, handle_key};

// -- geometry -------------------------------------------------------------

const CW: f32 = 8.4; // cell width  (px)
const CH: f32 = 18.0; // cell height (px)
const PAD: f32 = 10.0; // gutter around the grid
const FONT: f32 = 14.0; // glyph size
const BASE: f32 = 13.5; // text baseline within a cell row

/// Tomorrow-Night-ish palette so the SVG reads like a real dark terminal.
const BG: &str = "#1d1f21";
const FG: &str = "#c5c8c6";

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

// -- buffer -> svg --------------------------------------------------------

/// Serialize one rendered buffer to a standalone SVG document.
fn svg_of(buf: &Buffer) -> String {
    let area = buf.area;
    let (cols, rows) = (area.width, area.height);
    let w = PAD * 2.0 + cols as f32 * CW;
    let h = PAD * 2.0 + rows as f32 * CH;

    let mut out = String::new();
    out.push_str(&format!(
        "<svg xmlns=\"http://www.w3.org/2000/svg\" width=\"{w:.0}\" height=\"{h:.0}\" \
         viewBox=\"0 0 {w:.0} {h:.0}\" font-family=\"ui-monospace,'SF Mono','DejaVu Sans Mono',\
         Menlo,Consolas,monospace\" font-size=\"{FONT}\">\n"
    ));
    // Terminal backing.
    out.push_str(&format!(
        "<rect x=\"0\" y=\"0\" width=\"{w:.0}\" height=\"{h:.0}\" rx=\"8\" fill=\"{BG}\"/>\n"
    ));

    // Per-cell background rects (skip the default surface).
    for y in 0..rows {
        for x in 0..cols {
            let cell = &buf[(x, y)];
            if let Some(c) = hex(cell.bg) {
                let px = PAD + x as f32 * CW;
                let py = PAD + y as f32 * CH;
                out.push_str(&format!(
                    "<rect x=\"{px:.1}\" y=\"{py:.1}\" width=\"{:.1}\" height=\"{:.1}\" \
                     fill=\"{c}\"/>\n",
                    CW + 0.5,
                    CH
                ));
            }
        }
    }

    // Text: one run per maximal same-style span on a row.
    for y in 0..rows {
        let mut x = 0u16;
        while x < cols {
            let cell = &buf[(x, y)];
            let sym = cell.symbol();
            // Skip empty/space runs that carry no ink.
            if sym.trim().is_empty() && cell.modifier.is_empty() {
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
                text.push_str(if s.is_empty() { " " } else { s });
                x += 1;
            }
            let trimmed = text.trim_end();
            if trimmed.is_empty() {
                continue;
            }
            let run_cols = trimmed.chars().count() as f32;
            let px = PAD + start as f32 * CW;
            let py = PAD + y as f32 * CH + BASE;
            let fill = hex(fg).unwrap_or_else(|| FG.to_string());
            let mut attrs = format!(
                "x=\"{px:.1}\" y=\"{py:.1}\" fill=\"{fill}\" textLength=\"{:.1}\" \
                 lengthAdjust=\"spacingAndGlyphs\" xml:space=\"preserve\"",
                run_cols * CW
            );
            if modi.contains(Modifier::BOLD) {
                attrs.push_str(" font-weight=\"bold\"");
            }
            if modi.contains(Modifier::ITALIC) {
                attrs.push_str(" font-style=\"italic\"");
            }
            if modi.contains(Modifier::DIM) {
                attrs.push_str(" opacity=\"0.6\"");
            }
            if modi.contains(Modifier::UNDERLINED) {
                attrs.push_str(" text-decoration=\"underline\"");
            }
            out.push_str(&format!("<text {attrs}>{}</text>\n", xml_escape(trimmed)));
        }
    }

    out.push_str("</svg>\n");
    out
}

// -- sample herd ----------------------------------------------------------

fn t(id: &str, title: &str, kind: &str, pri: u8, status: Status, parent: Option<&str>) -> Task {
    Task {
        id: id.into(),
        title: title.into(),
        kind: kind.into(),
        priority: pri,
        status,
        created: Some("2026-09-01T10:00:00Z".into()),
        updated: Some("2026-09-06T12:00:00Z".into()),
        parent: parent.map(String::from),
        labels: vec![],
        depends_on: vec![],
        source: None,
        needs: None,
        verify: None,
        extra: Vec::new(),
        body: String::new(),
    }
}

/// A small, realistic herd: an umbrella with children across statuses, labels,
/// a dependency, and one child awaiting a human (the ⏳ needs badge).
fn herd() -> Vec<Task> {
    let mut coord = t(
        "yaks-3901",
        "Coordination substrate for parallel yak-shaving",
        "task",
        1,
        Status::Shaving,
        None,
    );
    coord.labels = vec!["coord".into(), "git".into()];

    let mut inbox = t(
        "yaks-b517",
        "HITL: needs-human frontmatter + inbox surfacing",
        "feature",
        2,
        Status::Shaving,
        Some("yaks-3901"),
    );
    inbox.labels = vec!["cli".into(), "ui".into()];
    inbox.needs = Some("human".into());
    inbox.body = "Add a `needs:` block agents raise via `ask` and a human clears via \
                  `answer`; surface blocked yaks in an Inbox view.\n\n\
                  ---\n▸ 2026-09-06T11:40:00Z [wtA]\nProposal drafted — see the drawer \
                  `inbox` chip. Which glyph should mark a blocked row?\n\n\
                  ---\n▸ 2026-09-06T12:00:00Z [joel]\nGo with the hourglass; ship it."
        .into();

    let mut diff = t(
        "yaks-70e5",
        "yaks diff <refA> <refB>: ref-generic herd diff",
        "feature",
        3,
        Status::Hairy,
        Some("yaks-3901"),
    );
    diff.labels = vec!["cli".into()];
    diff.depends_on = vec!["yaks-b517".into()];

    let mut shot = t(
        "yaks-3677",
        "TUI doc screenshots via headless buffer->SVG",
        "feature",
        3,
        Status::Shaving,
        Some("yaks-3901"),
    );
    shot.labels = vec!["ui".into(), "docs".into()];
    shot.body = "Walk the ratatui Buffer's per-cell symbol+fg/bg into an SVG grid so \
                 docs render in color."
        .into();

    let done = t(
        "yaks-865d",
        "Normalize skill names to yaks[-*]",
        "task",
        2,
        Status::Shorn,
        Some("yaks-3901"),
    );

    let mut solo = t(
        "yaks-10fb",
        "Coordination follow-ups (post-3901)",
        "idea",
        3,
        Status::Hairy,
        None,
    );
    solo.labels = vec!["agent".into()];

    vec![coord, inbox, diff, shot, done, solo]
}

fn app_at(w: u16, h: u16) -> App {
    let mut app = App::new(herd());
    HeadlessApp::on_resize(&mut app, w, h);
    app
}

fn press(app: &mut App, c: char) {
    handle_key(app, KeyEvent::new(KeyCode::Char(c), KeyModifiers::NONE));
}

fn press_key(app: &mut App, code: KeyCode) {
    handle_key(app, KeyEvent::new(code, KeyModifiers::NONE));
}

/// Switch to a built-in view by its stable key (e.g. `status:shaving`, `inbox`).
fn view_to(app: &mut App, key: &str) {
    let i = app
        .views
        .iter()
        .position(|v| v.key == key)
        .unwrap_or_else(|| panic!("view {key:?} present"));
    app.set_view(i);
}

fn write_svg(name: &str, app: &App, w: u16, h: u16) {
    let buf = render_to_buffer(app, w, h);
    let svg = svg_of(&buf);
    let path = format!("docs/assets/{name}.svg");
    std::fs::create_dir_all("docs/assets").unwrap();
    std::fs::write(&path, svg).unwrap();
    eprintln!("wrote {path}");
}

/// Regenerate every SVG in `docs/assets/`. Ignored by default; run explicitly:
/// `cargo test -p yaks docshots -- --ignored`.
#[test]
#[ignore = "asset generator; run with --ignored to refresh docs/assets/*.svg"]
fn docshots() {
    let (w, h) = (96u16, 20u16);

    // 1. List view (tree): the Shaving tab shows the umbrella + children across
    //    statuses, labels, a dependency, and the ⏳ needs badge on the HITL yak.
    let mut app = app_at(w, h);
    view_to(&mut app, "status:shaving");
    write_svg("tui-list", &app, w, h);

    // 2. Detail pane: open the HITL yak (only member of the Inbox), showing the
    //    Needs: field and attributed [wtA]/[joel] notes. Taller so the
    //    description + both comments fit under the metadata block.
    let dh = 34u16;
    let mut app = app_at(w, dh);
    view_to(&mut app, "inbox");
    press_key(&mut app, KeyCode::Enter); // open detail on the sole inbox row
    write_svg("tui-detail", &app, w, dh);

    // 3. Inbox view: flat list of every yak awaiting a human, across statuses.
    let mut app = app_at(w, h);
    view_to(&mut app, "inbox");
    write_svg("tui-inbox", &app, w, h);

    // 4. Filter drawer: the chip facets (status/type/priority/deps incl. inbox).
    let mut app = app_at(w, h);
    press(&mut app, 'f');
    write_svg("tui-drawer", &app, w, h);
}
