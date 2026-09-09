//! # toque
//!
//! Drive a [`ratatui`] app **headlessly** — from under the hat, so to speak. Instead of a real
//! terminal, the app renders into an in-memory [`ratatui::backend::TestBackend`] buffer; you inject
//! keys over a tiny line protocol and get back a plain-text snapshot after each step.
//!
//! Two jobs, both from a hidden position:
//! - **Drive** — feed keystrokes (`key j`, `key C-c`, `type hello`, `resize`).
//! - **Observe** — emit a deterministic text snapshot: a state header (internal
//!   app facts you choose to expose) and the character grid (layout). The text
//!   snapshot is the cheap, greppable, diffable channel; for *visual* inspection
//!   (colour, emphasis, borders) render the same buffer to SVG with
//!   [`render_to_svg`].
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
//! use toque::{HeadlessApp, DriverOpts, run};
//! use ratatui::Frame;
//! use ratatui::crossterm::event::{KeyCode, KeyEvent};
//!
//! struct MyApp { /* … */ }
//! impl HeadlessApp for MyApp {
//!     fn render(&self, f: &mut Frame) { /* draw widgets */ }
//!     fn handle_key(&mut self, key: KeyEvent) { /* mutate state */ }
//! }
//!
//! run(MyApp { /* … */ }, DriverOpts { width: 80, height: 24, diff: false }).unwrap();
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
//! ## Visual output
//!
//! The text snapshot deliberately carries no colour. When you need to *see* the
//! frame — colour, bold/dim, borders — render the buffer to a self-contained SVG
//! with [`render_to_svg`] (or [`buffer_to_svg`]); rasterise that to PNG if a
//! pixel image is required.

use std::io::{self, BufRead, Write};

use ratatui::Terminal;
use ratatui::backend::TestBackend;
use ratatui::buffer::Buffer;
use ratatui::crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

mod svg;
pub use svg::{buffer_to_svg, render_to_svg};

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

/// Configuration for a [`Session`] / [`run`].
pub struct DriverOpts {
    pub width: u16,
    pub height: u16,
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

/// Encodes buffers into snapshot body lines: the plain character grid, one
/// `String` per row. Reusable independently of the driver.
#[derive(Default)]
pub struct SnapshotEncoder;

impl SnapshotEncoder {
    pub fn new() -> Self {
        SnapshotEncoder
    }

    /// Encode one buffer into body lines (one per grid row). Trailing whitespace
    /// on each row is trimmed, but interior whitespace is preserved verbatim.
    pub fn encode(&mut self, buf: &Buffer) -> Vec<String> {
        plain_grid(buf)
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
            enc: SnapshotEncoder::new(),
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
    use ratatui::style::{Color, Style};
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

    fn session(diff: bool) -> Session<Dummy> {
        Session::new(
            Dummy::new(),
            DriverOpts {
                width: 20,
                height: 3,
                diff,
            },
        )
    }

    fn drive(script: &[&str]) -> String {
        let mut s = session(false);
        let mut out: Vec<u8> = Vec::new();
        s.emit(&mut out).unwrap();
        for line in script {
            s.step(line, &mut out).unwrap();
        }
        String::from_utf8(out).unwrap()
    }

    #[test]
    fn snapshot_has_header_and_grid() {
        let out = drive(&["key j"]);
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
    fn diff_mode_full_then_delta() {
        let mut s = session(true);
        let mut out: Vec<u8> = Vec::new();
        s.emit(&mut out).unwrap(); // frame 0: full
        s.step("key j", &mut out).unwrap(); // counter changes -> a changed line
        let out = String::from_utf8(out).unwrap();
        assert!(out.contains(" · full · "));
        assert!(out.contains(" · diff · "));
        assert!(out.contains("\nL")); // at least one L<i>: changed-line label
    }

    #[test]
    fn resize_updates_geometry() {
        let mut s = session(false);
        let mut out: Vec<u8> = Vec::new();
        s.step("resize 40 5", &mut out).unwrap();
        let out = String::from_utf8(out).unwrap();
        assert!(out.contains("40x5"));
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
        let mut s = session(false);
        let mut out: Vec<u8> = Vec::new();
        assert!(!s.step("quit", &mut out).unwrap());
    }

    #[test]
    fn should_quit_stops_the_loop() {
        let mut s = session(false);
        let mut out: Vec<u8> = Vec::new();
        // 'q' sets the app's quit flag; step returns false via should_quit()
        assert!(!s.step("key q", &mut out).unwrap());
    }
}
