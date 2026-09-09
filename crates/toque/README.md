# toque

Drive a [ratatui](https://ratatui.rs) app **headlessly** — from under the hat, so to speak. Instead
of a real terminal, your app renders into an in-memory `TestBackend` buffer; you inject keys over a
tiny line protocol and get back a deterministic, plain-text snapshot after each step.

It does two things, both from a hidden position:

- **Drive** — feed keystrokes (`key j`, `key C-c`, `type hello`, `resize 80 24`).
- **Observe** — emit a text snapshot: a state header (internal facts you choose to expose) and the
  character grid (layout). The text snapshot is the cheap, greppable, diffable channel and carries
  no colour; when you need to *see* a frame, render the same buffer to **SVG**.

Because a frame is a pure function of the app plus the terminal size, the output is deterministic —
good for **agent-driven exploration** of a TUI *and* for `insta`-style **snapshot tests** of any
ratatui UI.

## Quick start

Implement `HeadlessApp`, then drive it:

```rust
use toque::{HeadlessApp, DriverOpts, run};
use ratatui::Frame;
use ratatui::crossterm::event::KeyEvent;

struct MyApp { /* … */ }

impl HeadlessApp for MyApp {
    fn render(&self, f: &mut Frame) { /* draw your widgets */ }
    fn handle_key(&mut self, key: KeyEvent) { /* mutate state */ }
    // optional: on_resize, state_header, should_quit
}

run(MyApp { /* … */ }, DriverOpts { width: 80, height: 24, diff: false }).unwrap();
```

`run` reads the protocol from stdin and writes frames to stdout. For tests, drive a `Session`
directly against any `Write` sink, or skip the protocol entirely with `render_to_buffer` +
`SnapshotEncoder`.

## Protocol

One action per stdin line; a framed snapshot follows each:

```text
key <name>     press one key: a char, or a name (Enter, Esc, Tab, BackTab,
               Space, Backspace, Up/Down/Left/Right, Home, End, PageUp,
               PageDown, Delete). Prefix `C-` for Ctrl (e.g. C-c).
type <text>    type each character of the rest of the line verbatim.
snapshot       re-emit the current frame.
resize <w> <h> change the terminal size.
quit           exit.
```

A frame looks like:

```text
=== frame 3 · 80x24 · focus=list cursor=1 … ===
<body: the character grid, one line per row>
=== end ===
```

With `diff: true`, after the first (full) frame only changed body lines are emitted as `L<i>:
<line>` — a large token saving across a multi-step session.

## Visual output (SVG)

The text snapshot deliberately carries no colour — layout plus the state header are what a model
needs for cheap, diffable verification. When you need to *see* a frame (colour, bold/dim, borders),
render the same buffer to a self-contained SVG:

- `render_to_svg(&app, w, h) -> String` — drive and render in one call.
- `buffer_to_svg(&buffer) -> String` — render a buffer you already have.

The SVG is lean and row-ordered: one `<style>` class block for the palette, coalesced background
rects, and one `<text>` per row whose runs are `<tspan>`s pinned by an absolute `x` (a run breaks
after a wide glyph so an emoji can't shove the rest of the row out of column). It embeds directly in
Markdown/HTML and rasterises cleanly to PNG (e.g. via a headless browser) when a pixel image is
wanted — which also lets you control the resolution handed to a vision model.

> **History.** toque previously offered three *text* style-encodings (`spans`/`interleaved`/
> `parallel`) that packed per-cell colour into the snapshot. They were retired once SVG became the
> visual channel: SVG renders style natively and better, and the semantic style that matters
> (selection, focus, blocked) already rides in the state header as plain facts. The original
> evaluation that picked `spans` is kept for the record at
> [`docs/research/tui-style-eval.md`](https://github.com/rocketsurgery-games/yaks/blob/main/docs/research/tui-style-eval.md).

## Status

Extracted from the [yaks](https://github.com/rocketsurgery-games/yaks) TUI, which is its first consumer.
Pre-1.0: the API may shift as more apps adopt it.

## License

Apache-2.0.
