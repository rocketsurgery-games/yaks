# toque

Drive a [ratatui](https://ratatui.rs) app **headlessly** — from under the hat, so to speak. Instead
of a real terminal, your app renders into an in-memory `TestBackend` buffer; you inject keys over a
tiny line protocol and get back a deterministic, plain-text snapshot after each step.

It does two things, both from a hidden position:

- **Drive** — feed keystrokes (`key j`, `key C-c`, `type hello`, `resize 80 24`).
- **Observe** — emit a text snapshot: a state header (internal facts you choose to expose), the
  character grid (layout), and, optionally, per-cell **style** encoded so a language model can
  actually use it (selection, focus, links, borders, dimming).

Because a frame is a pure function of the app plus the terminal size, the output is deterministic —
good for **agent-driven exploration** of a TUI *and* for `insta`-style **snapshot tests** of any
ratatui UI.

## Quick start

Implement `HeadlessApp`, then drive it:

```rust
use toque::{HeadlessApp, DriverOpts, StyleEncoding, run};
use ratatui::Frame;
use ratatui::crossterm::event::KeyEvent;

struct MyApp { /* … */ }

impl HeadlessApp for MyApp {
    fn render(&self, f: &mut Frame) { /* draw your widgets */ }
    fn handle_key(&mut self, key: KeyEvent) { /* mutate state */ }
    // optional: on_resize, state_header, should_quit
}

run(MyApp { /* … */ }, DriverOpts {
    width: 80, height: 24,
    style: Some(StyleEncoding::Spans),
    diff: false,
}).unwrap();
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
<body: char grid, then style information if requested>
=== end ===
```

With `diff: true`, after the first (full) frame only changed body lines are emitted as `L<i>:
<line>` — a large token saving across a multi-step session.

## Style encodings

The hard question isn't the character grid (layout serializes trivially) — it's how to encode
per-cell **style** so a model can use it, and at what token cost. `StyleEncoding` offers three, all
keyed by a persistent registry so style-ids stay stable across frames (keeping diffs compact):

| encoding | form |
|----------|------|
| `Spans` (default) | each row inline as `id[run text]`; default-styled cells left literal |
| `Interleaved` | each text row followed by an aligned row of style-ids |
| `Parallel` | the whole char grid, then a second aligned grid of style-ids |

Each frame ends with a `legend:` mapping ids to concrete styles (`fg=cyan`, `bg=idx237`, `bold`,
`reversed`, …).

### Why `spans` is the default (evaluation summary)

We evaluated six encodings against a battery of layout/alignment questions, scored by a frontier
model, with real token counts. The full write-up lives in the yaks repo (`docs/tui-style-eval.md`);
the short version:

- **Accuracy saturated.** On simple *and* deliberately adversarial layouts (subtle 1-column
  misalignment, nested-vs-disjoint boxes, a list inside a box next to a same-format decoy), every
  style-bearing encoding scored ~perfectly. Accuracy did **not** discriminate the encodings at this
  model tier — so the deciding axis is **token cost**.
- **Cost (dense-list fixture, vs plain-only = 132 tokens):**

  | encoding | tokens | × plain |
  |----------|-------:|--------:|
  | **spans** | **259** | **1.96×** |
  | interleaved | 338 | 2.56× |
  | parallel | 364 | 2.76× |
  | runlist *(relational; not shipped)* | 451 | 3.42× |
  | ruler *(coordinate anchors; not shipped)* | 508 | 3.85× |
  | doublewidth *(id+char interleave; dead)* | 836 | 6.33× |

- **`spans` survives vertical alignment** — the surprising result. It preserves inter-run whitespace
  **literally**, so a model recovers a column by *summing whitespace (arithmetic)*, not by seeing
  it. It stayed correct on cue-free cumulative-offset stress out to ~16 columns / ~100 wide, with no
  false positives. Human-legibility and model-legibility diverge here.

**Load-bearing constraint:** because `spans` leans on literal whitespace, `SnapshotEncoder` never
collapses runs of spaces. Don't normalize them.

**Caveat:** all probes ran on a single frontier model family (Claude Opus 4.8). Cross-model-family
behavior at this specific capability is untested, and weaker models would likely crack the
whitespace arithmetic first — it's reasonable to require a frontier model for UI work. `interleaved`
is the explicit aligned-grid fallback if you want a column grid handed to you rather than
reconstructed; it dominates `parallel`.

## Status

Extracted from the [yaks](https://github.com/rocketsurgery-games/yaks) TUI, which is its first consumer.
Pre-1.0: the API may shift as more apps adopt it.

## License

Apache-2.0.
