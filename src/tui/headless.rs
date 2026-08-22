//! Headless driver for the TUI: a second "backend" that renders into an
//! in-memory `TestBackend` buffer instead of a real terminal, driven by a
//! line protocol on stdin and emitting plain-text snapshots on stdout.
//!
//! This reuses the exact same pure `render` + `handle_key` as the live TUI, so
//! what an agent (or a scripted test) sees is faithful to the real UI — minus
//! the VT noise. Each snapshot carries a state header (internal `App` facts),
//! the character grid (layout), and optionally an aligned style grid + legend
//! that encodes the semantic categories colour conveys (selection, focus,
//! links, dimming). Determinism comes for free: the frame is a pure function of
//! `App` + size.
//!
//! Protocol (one action per stdin line; a framed snapshot is emitted after each):
//!   key <name>     press one key: a single char, or a name (Enter, Esc, Tab,
//!                  BackTab, Space, Backspace, Up/Down/Left/Right, Home, End,
//!                  PageUp, PageDown, Delete). Prefix `C-` for Ctrl (e.g. C-c).
//!   type <text>    type each character of the rest of the line verbatim.
//!   snapshot       re-emit the current frame.
//!   resize <w> <h> change the terminal size.
//!   quit           exit.

use std::io::{self, BufRead, Write};

use anyhow::Result;
use ratatui::Terminal;
use ratatui::backend::TestBackend;
use ratatui::buffer::Buffer;
use ratatui::crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use ratatui::style::{Color, Modifier};

use super::{App, handle_key, render};

/// Which style representation the headless snapshot emits for each frame.
/// `Parallel` keeps the char grid then adds an aligned style-id grid; the others
/// were validated as cheaper/robust alternatives (see docs/tui-style-eval.md).
/// `Spans` is the default recommendation for LLM consumers.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum StyleEncoding {
    /// Plain grid, then an aligned grid of style-ids + legend.
    Parallel,
    /// Each text row immediately followed by its style-id row + legend.
    Interleaved,
    /// Each row inline as `id[run text]`; default-styled cells left literal.
    Spans,
}

impl StyleEncoding {
    pub fn parse(s: &str) -> Option<StyleEncoding> {
        match s {
            "parallel" => Some(StyleEncoding::Parallel),
            "interleaved" => Some(StyleEncoding::Interleaved),
            "spans" => Some(StyleEncoding::Spans),
            _ => None,
        }
    }
}

pub struct HeadlessOpts {
    pub width: u16,
    pub height: u16,
    /// `None` emits just the char grid; `Some(enc)` appends style information.
    pub style: Option<StyleEncoding>,
}

pub fn run_headless(app: App, opts: HeadlessOpts) -> Result<()> {
    let out = io::stdout();
    let mut out = out.lock();
    let mut d = Driver {
        app,
        w: opts.width.max(1),
        h: opts.height.max(1),
        style: opts.style,
        frame: 0,
    };
    d.emit(&mut out)?; // initial frame
    let stdin = io::stdin();
    for line in stdin.lock().lines() {
        let line = line?;
        if !d.step(&line, &mut out)? {
            break;
        }
    }
    Ok(())
}

struct Driver {
    app: App,
    w: u16,
    h: u16,
    style: Option<StyleEncoding>,
    frame: usize,
}

impl Driver {
    /// Apply one protocol line. Returns false to stop the loop.
    fn step(&mut self, line: &str, out: &mut impl Write) -> Result<bool> {
        let line = line.trim_end_matches(['\r', '\n']);
        if line.is_empty() {
            return Ok(true);
        }
        if line == "quit" {
            return Ok(false);
        }
        if line == "snapshot" {
            self.emit(out)?;
            return Ok(true);
        }
        if let Some(rest) = line.strip_prefix("resize ") {
            let mut it = rest.split_whitespace();
            if let (Some(w), Some(h)) = (it.next(), it.next()) {
                if let (Ok(w), Ok(h)) = (w.parse::<u16>(), h.parse::<u16>()) {
                    self.w = w.max(1);
                    self.h = h.max(1);
                }
            }
            self.emit(out)?;
            return Ok(true);
        }
        if let Some(rest) = line.strip_prefix("type ") {
            for c in rest.chars() {
                self.press(KeyEvent::new(KeyCode::Char(c), KeyModifiers::NONE));
            }
            self.emit(out)?;
            return Ok(!self.app.quit);
        }
        if let Some(rest) = line.strip_prefix("key ") {
            self.press(parse_key(rest.trim()));
            self.emit(out)?;
            return Ok(!self.app.quit);
        }
        writeln!(out, "! unknown action: {line}")?;
        out.flush()?;
        Ok(true)
    }

    fn press(&mut self, ev: KeyEvent) {
        // Mirror the live loop's paging basis (main area height minus tab+status).
        self.app.page = self.h.saturating_sub(2).max(1);
        handle_key(&mut self.app, ev);
    }

    fn emit(&mut self, out: &mut impl Write) -> Result<()> {
        let buf = render_to_buffer(&self.app, self.w, self.h);
        writeln!(
            out,
            "=== frame {} · {}x{} · {} ===",
            self.frame,
            self.w,
            self.h,
            self.app.state_header()
        )?;
        for line in encode_body(&buf, self.style) {
            writeln!(out, "{line}")?;
        }
        writeln!(out, "=== end ===")?;
        out.flush()?;
        self.frame += 1;
        Ok(())
    }
}

fn render_to_buffer(app: &App, w: u16, h: u16) -> Buffer {
    let mut term = Terminal::new(TestBackend::new(w, h)).expect("test backend");
    term.draw(|f| render(app, f)).expect("render");
    term.backend().buffer().clone()
}

/// Trailing-trimmed display width of a buffer row (index of last non-space + 1).
fn row_width(buf: &Buffer, y: u16) -> u16 {
    let mut w = 0;
    for x in 0..buf.area.width {
        if buf[(x, y)].symbol() != " " {
            w = x + 1;
        }
    }
    w
}

fn plain_grid(buf: &Buffer) -> Vec<String> {
    (0..buf.area.height)
        .map(|y| {
            let w = row_width(buf, y);
            (0..w).map(|x| buf[(x, y)].symbol()).collect()
        })
        .collect()
}

/// Assemble the snapshot body (between the frame header and `=== end ===`) for
/// the chosen style encoding. `None` emits just the plain grid; the others add
/// or interleave style information. See `StyleEncoding` and docs/tui-style-eval.md.
fn encode_body(buf: &Buffer, enc: Option<StyleEncoding>) -> Vec<String> {
    let plain = plain_grid(buf);
    match enc {
        None => plain,
        Some(StyleEncoding::Parallel) => {
            let (rows, legend) = style_layer(buf);
            let mut out = plain;
            out.push("--- styles ---".into());
            out.extend(rows);
            out.push(format!("legend: {}", legend.join("  ")));
            out
        }
        Some(StyleEncoding::Interleaved) => {
            let (rows, legend) = style_layer(buf);
            let mut out = Vec::with_capacity(plain.len() * 2 + 1);
            for (text, ids) in plain.iter().zip(rows.iter()) {
                out.push(text.clone());
                out.push(ids.clone());
            }
            out.push(format!("legend: {}", legend.join("  ")));
            out
        }
        Some(StyleEncoding::Spans) => {
            let (rows, legend) = spans_layer(buf);
            let mut out = rows;
            out.push(format!("legend: {}", legend.join("  ")));
            out
        }
    }
}

/// Inline spans: default-styled cells are emitted literally (whitespace
/// preserved -- load-bearing for column arithmetic); each run of one non-default
/// style becomes `id[run text]`. Returns (rows, legend).
fn spans_layer(buf: &Buffer) -> (Vec<String>, Vec<String>) {
    let mut keys: Vec<StyleKey> = Vec::new();
    let mut rows = Vec::new();
    for y in 0..buf.area.height {
        let w = row_width(buf, y);
        let mut line = String::new();
        let mut x = 0u16;
        while x < w {
            let key = StyleKey::of(&buf[(x, y)]);
            if key.is_default() {
                line.push_str(buf[(x, y)].symbol());
                x += 1;
                continue;
            }
            let start = x;
            while x < w && StyleKey::of(&buf[(x, y)]) == key {
                x += 1;
            }
            let id = match keys.iter().position(|k| *k == key) {
                Some(i) => i,
                None => {
                    keys.push(key);
                    keys.len() - 1
                }
            };
            let text: String = (start..x).map(|c| buf[(c, y)].symbol()).collect();
            line.push_str(&format!("{}[{text}]", base36(id)));
        }
        rows.push(line.trim_end().to_string());
    }
    let legend = keys
        .iter()
        .enumerate()
        .map(|(i, k)| format!("{}={}", base36(i), k.describe()))
        .collect();
    (rows, legend)
}

/// (aligned style-id rows, legend entries). Style ids are base36, assigned in
/// first-appearance order; the legend maps each to a semantic description.
fn style_layer(buf: &Buffer) -> (Vec<String>, Vec<String>) {
    let mut keys: Vec<StyleKey> = Vec::new();
    let mut rows = Vec::new();
    for y in 0..buf.area.height {
        let w = row_width(buf, y);
        let mut row = String::new();
        for x in 0..w {
            let key = StyleKey::of(&buf[(x, y)]);
            let id = match keys.iter().position(|k| *k == key) {
                Some(i) => i,
                None => {
                    keys.push(key);
                    keys.len() - 1
                }
            };
            row.push(base36(id));
        }
        rows.push(row);
    }
    let legend = keys
        .iter()
        .enumerate()
        .map(|(i, k)| format!("{}={}", base36(i), k.describe()))
        .collect();
    (rows, legend)
}

#[derive(PartialEq)]
struct StyleKey {
    fg: Color,
    bg: Color,
    mods: Modifier,
}

impl StyleKey {
    fn of(cell: &ratatui::buffer::Cell) -> StyleKey {
        let s = cell.style();
        StyleKey {
            fg: s.fg.unwrap_or(Color::Reset),
            bg: s.bg.unwrap_or(Color::Reset),
            mods: s.add_modifier,
        }
    }

    fn is_default(&self) -> bool {
        self.fg == Color::Reset && self.bg == Color::Reset && self.mods.is_empty()
    }

    fn describe(&self) -> String {
        if self.fg == Color::Reset && self.bg == Color::Reset && self.mods.is_empty() {
            return "default".into();
        }
        let mut parts = Vec::new();
        if self.fg != Color::Reset {
            parts.push(format!("fg={}", color_name(self.fg)));
        }
        if self.bg != Color::Reset {
            parts.push(format!("bg={}", color_name(self.bg)));
        }
        for (name, flag) in [
            ("bold", Modifier::BOLD),
            ("dim", Modifier::DIM),
            ("italic", Modifier::ITALIC),
            ("underline", Modifier::UNDERLINED),
            ("reversed", Modifier::REVERSED),
        ] {
            if self.mods.contains(flag) {
                parts.push(name.to_string());
            }
        }
        parts.join("+")
    }
}

fn color_name(c: Color) -> String {
    match c {
        Color::Reset => "-".into(),
        Color::Black => "black".into(),
        Color::Red => "red".into(),
        Color::Green => "green".into(),
        Color::Yellow => "yellow".into(),
        Color::Blue => "blue".into(),
        Color::Magenta => "magenta".into(),
        Color::Cyan => "cyan".into(),
        Color::Gray => "gray".into(),
        Color::DarkGray => "dimgray".into(),
        Color::LightRed => "lightred".into(),
        Color::LightGreen => "lightgreen".into(),
        Color::LightYellow => "lightyellow".into(),
        Color::LightBlue => "lightblue".into(),
        Color::LightMagenta => "lightmagenta".into(),
        Color::LightCyan => "lightcyan".into(),
        Color::White => "white".into(),
        Color::Rgb(r, g, b) => format!("#{r:02x}{g:02x}{b:02x}"),
        Color::Indexed(i) => format!("idx{i}"),
    }
}

fn base36(i: usize) -> char {
    match i {
        0..=9 => (b'0' + i as u8) as char,
        10..=35 => (b'a' + (i - 10) as u8) as char,
        _ => '#',
    }
}

fn parse_key(spec: &str) -> KeyEvent {
    let (mods, name) = match spec.strip_prefix("C-") {
        Some(rest) => (KeyModifiers::CONTROL, rest),
        None => (KeyModifiers::NONE, spec),
    };
    let code = match name {
        "Enter" => KeyCode::Enter,
        "Esc" => KeyCode::Esc,
        "Tab" => KeyCode::Tab,
        "BackTab" => KeyCode::BackTab,
        "Space" => KeyCode::Char(' '),
        "Backspace" => KeyCode::Backspace,
        "Up" => KeyCode::Up,
        "Down" => KeyCode::Down,
        "Left" => KeyCode::Left,
        "Right" => KeyCode::Right,
        "Home" => KeyCode::Home,
        "End" => KeyCode::End,
        "PageUp" => KeyCode::PageUp,
        "PageDown" => KeyCode::PageDown,
        "Delete" => KeyCode::Delete,
        s if s.chars().count() == 1 => KeyCode::Char(s.chars().next().unwrap()),
        _ => KeyCode::Null,
    };
    KeyEvent::new(code, mods)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn drive(script: &[&str], style: Option<StyleEncoding>) -> String {
        // A herd-less App renders the default sample-free tree (empty), which is
        // still enough to exercise the protocol + serializer deterministically.
        let app = App::new(vec![
            sample_task("a0", "Root A"),
            sample_task("a1", "Child A1"),
        ]);
        let mut d = Driver {
            app,
            w: 60,
            h: 10,
            style,
            frame: 0,
        };
        let mut out: Vec<u8> = Vec::new();
        d.emit(&mut out).unwrap();
        for line in script {
            d.step(line, &mut out).unwrap();
        }
        String::from_utf8(out).unwrap()
    }

    fn sample_task(id: &str, title: &str) -> crate::model::Task {
        crate::model::Task {
            id: id.into(),
            title: title.into(),
            kind: "task".into(),
            priority: 3,
            status: crate::model::Status::Hairy,
            created: None,
            updated: None,
            parent: None,
            labels: vec![],
            depends_on: vec![],
            source: None,
            body: String::new(),
        }
    }

    #[test]
    fn snapshot_has_header_and_grid() {
        let out = drive(&["key j"], None);
        // Two frames (initial + after j); each framed with a state header.
        assert_eq!(out.matches("=== frame ").count(), 2);
        assert!(out.contains("focus=list"));
        assert!(out.contains("cursor=0")); // initial frame
        assert!(out.contains("cursor=1")); // after moving down
        assert!(out.contains("Root A"));
    }

    #[test]
    fn style_layer_emitted_and_aligned() {
        let out = drive(&[], Some(StyleEncoding::Parallel));
        assert!(out.contains("--- styles ---"));
        assert!(out.contains("legend:"));
        assert!(out.contains("default"));
    }

    #[test]
    fn spans_encoding_has_legend_and_no_style_grid() {
        let out = drive(&[], Some(StyleEncoding::Spans));
        assert!(out.contains("legend:"));
        assert!(!out.contains("--- styles ---"));
    }

    #[test]
    fn interleaved_encoding_has_legend_and_no_style_grid() {
        let out = drive(&[], Some(StyleEncoding::Interleaved));
        assert!(out.contains("legend:"));
        assert!(!out.contains("--- styles ---"));
    }

    #[test]
    fn parse_encoding_names() {
        assert_eq!(StyleEncoding::parse("spans"), Some(StyleEncoding::Spans));
        assert_eq!(
            StyleEncoding::parse("interleaved"),
            Some(StyleEncoding::Interleaved)
        );
        assert_eq!(
            StyleEncoding::parse("parallel"),
            Some(StyleEncoding::Parallel)
        );
        assert!(StyleEncoding::parse("nope").is_none());
    }

    #[test]
    fn parse_key_handles_names_and_ctrl() {
        assert_eq!(parse_key("Enter").code, KeyCode::Enter);
        assert_eq!(parse_key("j").code, KeyCode::Char('j'));
        let c = parse_key("C-c");
        assert_eq!(c.code, KeyCode::Char('c'));
        assert!(c.modifiers.contains(KeyModifiers::CONTROL));
    }

    #[test]
    fn quit_stops_the_loop() {
        let mut d = Driver {
            app: App::new(vec![]),
            w: 40,
            h: 8,
            style: None,
            frame: 0,
        };
        let mut out: Vec<u8> = Vec::new();
        assert!(!d.step("quit", &mut out).unwrap());
    }
}
