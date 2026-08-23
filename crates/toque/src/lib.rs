//! # toque
//!
//! Drive a [`ratatui`] app **headlessly** — from under the hat, so to speak. Instead of a real
//! terminal, the app renders into an in-memory [`ratatui::backend::TestBackend`] buffer; you inject
//! keys over a tiny line protocol and get back a plain-text snapshot after each step.
//!
//! Two jobs, both from a hidden position:
//! - **Drive** — feed keystrokes (`key j`, `key C-c`, `type hello`, `resize`).
//! - **Observe** — emit a deterministic text snapshot: a state header (internal
//!   app facts you choose to expose), the character grid (layout), and,
//!   optionally, per-cell **style** encoded so a language model can actually use
//!   it (selection, focus, links, borders, dimming).
//!
//! Because the frame is a pure function of the app plus the terminal size, the
//! output is deterministic — good for agent-driven exploration *and* for
//! `insta`-style snapshot tests of any ratatui UI.
//!
//! ## Using it
//!
//! Implement [`HeadlessApp`] for your app, then either drive it interactively
//! with [`run`] (reads the protocol from stdin, writes frames to stdout) or
//! step it programmatically with [`Session`] for tests. To snapshot a single
//! frame without the protocol at all, use [`render_to_buffer`] + [`SnapshotEncoder`].
//!
//! ```no_run
//! use toque::{HeadlessApp, DriverOpts, StyleEncoding, run};
//! use ratatui::Frame;
//! use ratatui::crossterm::event::{KeyCode, KeyEvent};
//!
//! struct MyApp { /* … */ }
//! impl HeadlessApp for MyApp {
//!     fn render(&self, f: &mut Frame) { /* draw widgets */ }
//!     fn handle_key(&mut self, key: KeyEvent) { /* mutate state */ }
//! }
//!
//! run(MyApp { /* … */ }, DriverOpts { width: 80, height: 24,
//!     style: Some(StyleEncoding::Spans), diff: false }).unwrap();
//! ```
//!
//! ## Protocol (one action per stdin line; a framed snapshot follows each)
//!
//! ```text
//!   key <name>     press one key: a single char, or a name (Enter, Esc, Tab,
//!                  BackTab, Space, Backspace, Up/Down/Left/Right, Home, End,
//!                  PageUp, PageDown, Delete). Prefix `C-` for Ctrl (e.g. C-c).
//!   type <text>    type each character of the rest of the line verbatim.
//!   snapshot       re-emit the current frame.
//!   resize <w> <h> change the terminal size.
//!   quit           exit.
//! ```
//!
//! ## Style encodings
//!
//! See [`StyleEncoding`]. On a frontier model, `spans` (the recommended default)
//! is the cheapest style-bearing encoding and survives vertical-alignment
//! probes because inter-run whitespace is emitted *literally* — the model
//! recovers columns by summing spaces. See the crate README for the evaluation
//! behind that choice.

use std::collections::BTreeSet;
use std::io::{self, BufRead, Write};

use ratatui::Terminal;
use ratatui::backend::TestBackend;
use ratatui::buffer::{Buffer, Cell};
use ratatui::crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use ratatui::style::{Color, Modifier};

/// The seam a headless-drivable app implements.
///
/// [`render`](HeadlessApp::render) and [`handle_key`](HeadlessApp::handle_key)
/// are the same pure functions your live terminal loop already calls, so what a
/// headless consumer sees is faithful to the real UI. The rest are optional
/// hooks with sensible defaults.
pub trait HeadlessApp {
    /// Paint the current state into the frame. Called for every snapshot.
    fn render(&self, frame: &mut ratatui::Frame);

    /// Apply one key event, mutating state.
    fn handle_key(&mut self, key: KeyEvent);

    /// Told the current viewport size on startup and on every `resize`. Use it
    /// to derive anything key handling needs before the next render (e.g. page
    /// size for PageUp/PageDown). Default: no-op.
    fn on_resize(&mut self, _width: u16, _height: u16) {}

    /// A one-line, human-readable digest of internal state, appended to each
    /// frame header — invaluable for debugging what the UI *thinks* is true.
    /// Default: empty (the header omits it entirely).
    fn state_header(&self) -> String {
        String::new()
    }

    /// Whether the driver loop should stop after the last key. Default: never.
    fn should_quit(&self) -> bool {
        false
    }
}

/// Which style representation a snapshot emits for each frame.
///
/// All three keep the layout recoverable; they trade token cost for form.
/// `Spans` is the recommended default (cheapest style-bearing, word-preserving);
/// `Interleaved` is the aligned-grid fallback and dominates `Parallel`.
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
    /// Parse a CLI-friendly name (`parallel` | `interleaved` | `spans`).
    pub fn parse(s: &str) -> Option<StyleEncoding> {
        match s {
            "parallel" => Some(StyleEncoding::Parallel),
            "interleaved" => Some(StyleEncoding::Interleaved),
            "spans" => Some(StyleEncoding::Spans),
            _ => None,
        }
    }
}

/// Configuration for a [`Session`] / [`run`].
pub struct DriverOpts {
    pub width: u16,
    pub height: u16,
    /// `None` emits just the char grid; `Some(enc)` appends style information.
    pub style: Option<StyleEncoding>,
    /// When true, after the first (full) frame emit only changed body lines as
    /// `L<i>: <line>` — large token savings across a multi-step session.
    pub diff: bool,
}

/// Render a [`HeadlessApp`] into an in-memory buffer at the given size.
///
/// Pure and deterministic — handy on its own for snapshot tests.
pub fn render_to_buffer<A: HeadlessApp>(app: &A, w: u16, h: u16) -> Buffer {
    let mut term = Terminal::new(TestBackend::new(w.max(1), h.max(1))).expect("test backend");
    term.draw(|f| app.render(f)).expect("render");
    term.backend().buffer().clone()
}

/// Encodes buffers into snapshot body lines, keeping a persistent style-id map
/// so ids stay stable across frames (which keeps frame diffs compact when only
/// the selection moves). Reusable independently of the driver.
pub struct SnapshotEncoder {
    enc: Option<StyleEncoding>,
    reg: StyleRegistry,
}

impl SnapshotEncoder {
    /// `None` encodes just the plain grid; `Some(enc)` adds style information.
    pub fn new(enc: Option<StyleEncoding>) -> Self {
        SnapshotEncoder {
            enc,
            reg: StyleRegistry::new(),
        }
    }

    /// Encode one buffer into body lines (the legend, when present, is the last
    /// line). Trailing whitespace on each grid row is trimmed, but interior
    /// whitespace is preserved verbatim — it is load-bearing for column
    /// arithmetic, so callers must not collapse it.
    pub fn encode(&mut self, buf: &Buffer) -> Vec<String> {
        encode_body(buf, self.enc, &mut self.reg)
    }
}

/// A stepwise headless session: hold an app, feed it protocol lines, get frames.
///
/// [`run`] is a thin stdin/stdout loop over this; tests can drive it directly by
/// passing any [`Write`] sink to [`emit`](Session::emit) / [`step`](Session::step).
pub struct Session<A: HeadlessApp> {
    app: A,
    w: u16,
    h: u16,
    diff: bool,
    /// Serialized body of the previous frame, for line-level diffing.
    prev_body: Option<Vec<String>>,
    enc: SnapshotEncoder,
    frame: usize,
}

impl<A: HeadlessApp> Session<A> {
    /// Create a session and inform the app of its initial size. Does not emit a
    /// frame — call [`emit`](Session::emit) for the first frame.
    pub fn new(mut app: A, opts: DriverOpts) -> Self {
        let w = opts.width.max(1);
        let h = opts.height.max(1);
        app.on_resize(w, h);
        Session {
            app,
            w,
            h,
            diff: opts.diff,
            prev_body: None,
            enc: SnapshotEncoder::new(opts.style),
            frame: 0,
        }
    }

    /// Borrow the underlying app (e.g. to assert on state in a test).
    pub fn app(&self) -> &A {
        &self.app
    }

    /// Render the current state and write one framed snapshot to `out`.
    pub fn emit(&mut self, out: &mut impl Write) -> io::Result<()> {
        let buf = render_to_buffer(&self.app, self.w, self.h);
        let body = self.enc.encode(&buf);
        // In diff mode, emit only changed lines against the previous frame once
        // we have one to compare with the same shape; otherwise emit the whole
        // body (first frame, or geometry/line-count changed).
        let (tag, lines) = if self.diff {
            match self.prev_body.take() {
                Some(prev) if prev.len() == body.len() => {
                    let changed = prev
                        .iter()
                        .zip(body.iter())
                        .enumerate()
                        .filter(|(_, (a, b))| a != b)
                        .map(|(i, (_, b))| format!("L{i}: {b}"))
                        .collect::<Vec<_>>();
                    (" · diff", changed)
                }
                _ => (" · full", body.clone()),
            }
        } else {
            ("", body.clone())
        };
        let mut head = format!("=== frame {} · {}x{}{}", self.frame, self.w, self.h, tag);
        let hdr = self.app.state_header();
        if !hdr.is_empty() {
            head.push_str(" · ");
            head.push_str(&hdr);
        }
        head.push_str(" ===");
        writeln!(out, "{head}")?;
        for line in &lines {
            writeln!(out, "{line}")?;
        }
        writeln!(out, "=== end ===")?;
        out.flush()?;
        if self.diff {
            self.prev_body = Some(body);
        }
        self.frame += 1;
        Ok(())
    }

    /// Apply one protocol line, emitting the resulting frame. Returns `false`
    /// when the loop should stop (`quit`, or the app reports `should_quit`).
    pub fn step(&mut self, line: &str, out: &mut impl Write) -> io::Result<bool> {
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
                    self.app.on_resize(self.w, self.h);
                }
            }
            self.emit(out)?;
            return Ok(true);
        }
        if let Some(rest) = line.strip_prefix("type ") {
            for c in rest.chars() {
                self.app
                    .handle_key(KeyEvent::new(KeyCode::Char(c), KeyModifiers::NONE));
            }
            self.emit(out)?;
            return Ok(!self.app.should_quit());
        }
        if let Some(rest) = line.strip_prefix("key ") {
            self.app.handle_key(parse_key(rest.trim()));
            self.emit(out)?;
            return Ok(!self.app.should_quit());
        }
        writeln!(out, "! unknown action: {line}")?;
        out.flush()?;
        Ok(true)
    }
}

/// Drive an app from stdin, emitting a framed snapshot to stdout after each
/// action. Blocks until `quit`, EOF, or the app reports `should_quit`.
pub fn run<A: HeadlessApp>(app: A, opts: DriverOpts) -> io::Result<()> {
    let out = io::stdout();
    let mut out = out.lock();
    let mut session = Session::new(app, opts);
    session.emit(&mut out)?; // initial frame
    let stdin = io::stdin();
    for line in stdin.lock().lines() {
        let line = line?;
        if !session.step(&line, &mut out)? {
            break;
        }
    }
    Ok(())
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

/// Append-only registry mapping a cell style to a stable base36 id, persisted
/// across frames so ids don't renumber between snapshots — which keeps frame
/// diffs compact when the selection moves.
struct StyleRegistry {
    keys: Vec<StyleKey>,
}

impl StyleRegistry {
    fn new() -> Self {
        StyleRegistry { keys: Vec::new() }
    }

    fn id_of(&mut self, key: StyleKey) -> usize {
        match self.keys.iter().position(|k| *k == key) {
            Some(i) => i,
            None => {
                self.keys.push(key);
                self.keys.len() - 1
            }
        }
    }

    /// Legend entries for exactly the ids used in the current frame, in id order.
    fn legend(&self, used: &BTreeSet<usize>) -> Vec<String> {
        used.iter()
            .map(|&i| format!("{}={}", base36(i), self.keys[i].describe()))
            .collect()
    }
}

/// Assemble the snapshot body (between the frame header and `=== end ===`) for
/// the chosen style encoding. `None` emits just the plain grid; the others add
/// or interleave style information.
fn encode_body(buf: &Buffer, enc: Option<StyleEncoding>, reg: &mut StyleRegistry) -> Vec<String> {
    let plain = plain_grid(buf);
    match enc {
        None => plain,
        Some(StyleEncoding::Parallel) => {
            let (rows, legend) = style_layer(buf, reg);
            let mut out = plain;
            out.push("--- styles ---".into());
            out.extend(rows);
            out.push(format!("legend: {}", legend.join("  ")));
            out
        }
        Some(StyleEncoding::Interleaved) => {
            let (rows, legend) = style_layer(buf, reg);
            let mut out = Vec::with_capacity(plain.len() * 2 + 1);
            for (text, ids) in plain.iter().zip(rows.iter()) {
                out.push(text.clone());
                out.push(ids.clone());
            }
            out.push(format!("legend: {}", legend.join("  ")));
            out
        }
        Some(StyleEncoding::Spans) => {
            let (rows, legend) = spans_layer(buf, reg);
            let mut out = rows;
            out.push(format!("legend: {}", legend.join("  ")));
            out
        }
    }
}

/// Inline spans: default-styled cells are emitted literally (whitespace
/// preserved — load-bearing for column arithmetic); each run of one non-default
/// style becomes `id[run text]`, keyed via the persistent registry (stable ids).
fn spans_layer(buf: &Buffer, reg: &mut StyleRegistry) -> (Vec<String>, Vec<String>) {
    let mut used = BTreeSet::new();
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
            let id = reg.id_of(key);
            used.insert(id);
            let text: String = (start..x).map(|c| buf[(c, y)].symbol()).collect();
            line.push_str(&format!("{}[{text}]", base36(id)));
        }
        rows.push(line.trim_end().to_string());
    }
    (rows, reg.legend(&used))
}

/// Aligned style-id rows + legend, keyed via the persistent registry so ids are
/// stable across frames. The legend lists only the ids present this frame.
fn style_layer(buf: &Buffer, reg: &mut StyleRegistry) -> (Vec<String>, Vec<String>) {
    let mut used = BTreeSet::new();
    let mut rows = Vec::new();
    for y in 0..buf.area.height {
        let w = row_width(buf, y);
        let mut row = String::new();
        for x in 0..w {
            let id = reg.id_of(StyleKey::of(&buf[(x, y)]));
            used.insert(id);
            row.push(base36(id));
        }
        rows.push(row);
    }
    (rows, reg.legend(&used))
}

#[derive(PartialEq)]
struct StyleKey {
    fg: Color,
    bg: Color,
    mods: Modifier,
}

impl StyleKey {
    fn of(cell: &Cell) -> StyleKey {
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

/// Parse a key spec: a single char, a named key, optionally `C-`-prefixed for
/// Ctrl. Unknown names map to [`KeyCode::Null`].
pub fn parse_key(spec: &str) -> KeyEvent {
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
    use ratatui::Frame;
    use ratatui::style::Style;
    use ratatui::text::{Line, Span};
    use ratatui::widgets::Paragraph;

    /// A minimal, dependency-free app: one line with a cyan run, a space, and a
    /// red counter that increments on any key (`q` quits).
    struct Dummy {
        n: u32,
        quit: bool,
    }

    impl Dummy {
        fn new() -> Self {
            Dummy { n: 0, quit: false }
        }
    }

    impl HeadlessApp for Dummy {
        fn render(&self, f: &mut Frame) {
            let line = Line::from(vec![
                Span::styled("hi", Style::new().fg(Color::Cyan)),
                Span::raw(" "),
                Span::styled(format!("{}", self.n), Style::new().fg(Color::Red)),
            ]);
            f.render_widget(Paragraph::new(line), f.area());
        }
        fn handle_key(&mut self, key: KeyEvent) {
            match key.code {
                KeyCode::Char('q') => self.quit = true,
                _ => self.n += 1,
            }
        }
        fn state_header(&self) -> String {
            format!("n={}", self.n)
        }
        fn should_quit(&self) -> bool {
            self.quit
        }
    }

    fn session(style: Option<StyleEncoding>, diff: bool) -> Session<Dummy> {
        Session::new(
            Dummy::new(),
            DriverOpts {
                width: 20,
                height: 3,
                style,
                diff,
            },
        )
    }

    fn drive(script: &[&str], style: Option<StyleEncoding>) -> String {
        let mut s = session(style, false);
        let mut out: Vec<u8> = Vec::new();
        s.emit(&mut out).unwrap();
        for line in script {
            s.step(line, &mut out).unwrap();
        }
        String::from_utf8(out).unwrap()
    }

    #[test]
    fn snapshot_has_header_and_grid() {
        let out = drive(&["key j"], None);
        assert_eq!(out.matches("=== frame ").count(), 2);
        assert!(out.contains("n=0")); // initial state header
        assert!(out.contains("n=1")); // after a key
        assert!(out.contains("hi"));
    }

    #[test]
    fn empty_state_header_omitted() {
        struct Bare;
        impl HeadlessApp for Bare {
            fn render(&self, _f: &mut Frame) {}
            fn handle_key(&mut self, _k: KeyEvent) {}
        }
        let mut s = Session::new(
            Bare,
            DriverOpts {
                width: 10,
                height: 2,
                style: None,
                diff: false,
            },
        );
        let mut out: Vec<u8> = Vec::new();
        s.emit(&mut out).unwrap();
        let out = String::from_utf8(out).unwrap();
        // No trailing " · <hdr>" section when state_header is empty.
        assert!(out.contains("=== frame 0 · 10x2 ==="));
    }

    #[test]
    fn parallel_has_style_grid_and_legend() {
        let out = drive(&[], Some(StyleEncoding::Parallel));
        assert!(out.contains("--- styles ---"));
        assert!(out.contains("legend:"));
        assert!(out.contains("fg=cyan"));
        assert!(out.contains("fg=red"));
    }

    #[test]
    fn spans_has_legend_and_no_style_grid() {
        let out = drive(&[], Some(StyleEncoding::Spans));
        assert!(out.contains("legend:"));
        assert!(!out.contains("--- styles ---"));
        // cyan run emitted inline as `id[hi]`, whitespace preserved literally
        assert!(out.contains("[hi]"));
    }

    #[test]
    fn interleaved_has_legend_and_no_style_grid() {
        let out = drive(&[], Some(StyleEncoding::Interleaved));
        assert!(out.contains("legend:"));
        assert!(!out.contains("--- styles ---"));
    }

    #[test]
    fn diff_mode_full_then_delta() {
        let mut s = session(None, true);
        let mut out: Vec<u8> = Vec::new();
        s.emit(&mut out).unwrap(); // frame 0: full
        s.step("key j", &mut out).unwrap(); // counter changes -> a changed line
        let out = String::from_utf8(out).unwrap();
        assert!(out.contains(" · full · "));
        assert!(out.contains(" · diff · "));
        assert!(out.contains("\nL")); // at least one L<i>: changed-line label
    }

    #[test]
    fn spans_diff_keeps_legend_stable() {
        let mut s = session(Some(StyleEncoding::Spans), true);
        let mut out: Vec<u8> = Vec::new();
        s.emit(&mut out).unwrap(); // full
        s.step("key j", &mut out).unwrap(); // only the counter text changes
        let out = String::from_utf8(out).unwrap();
        let diff_frame = out.split("· diff ·").nth(1).unwrap();
        // stable ids keep the legend identical, so it must not reappear in a diff
        assert!(!diff_frame.contains("legend:"));
    }

    #[test]
    fn style_registry_ids_are_stable() {
        let mut reg = StyleRegistry::new();
        let a = StyleKey {
            fg: Color::Cyan,
            bg: Color::Reset,
            mods: Modifier::empty(),
        };
        let b = StyleKey {
            fg: Color::Red,
            bg: Color::Reset,
            mods: Modifier::empty(),
        };
        let ia = reg.id_of(a);
        let ib = reg.id_of(b);
        assert_ne!(ia, ib);
        let a2 = StyleKey {
            fg: Color::Cyan,
            bg: Color::Reset,
            mods: Modifier::empty(),
        };
        assert_eq!(reg.id_of(a2), ia); // same style -> same id across calls/frames
    }

    #[test]
    fn resize_updates_geometry() {
        let mut s = session(None, false);
        let mut out: Vec<u8> = Vec::new();
        s.step("resize 40 5", &mut out).unwrap();
        let out = String::from_utf8(out).unwrap();
        assert!(out.contains("40x5"));
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
    fn quit_action_stops_the_loop() {
        let mut s = session(None, false);
        let mut out: Vec<u8> = Vec::new();
        assert!(!s.step("quit", &mut out).unwrap());
    }

    #[test]
    fn should_quit_stops_the_loop() {
        let mut s = session(None, false);
        let mut out: Vec<u8> = Vec::new();
        // 'q' sets the app's quit flag; step returns false via should_quit()
        assert!(!s.step("key q", &mut out).unwrap());
    }
}
