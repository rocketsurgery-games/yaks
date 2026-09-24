---
id: yaks-137a
title: Release v0.0.11
type: chore
priority: 3
created: '2026-09-24T02:30:23Z'
updated: '2026-09-24T02:30:42Z'
verify: cargo test --workspace
---

Cut v0.0.11 to ship the 2026-09-23 UI lightning round. Highlights since v0.0.10: list scrolling moves only as far as needed (yaks-9009); # go-to-yak picker (yaks-63f3); detail pane clamped in width, covering the list on narrow terminals (yaks-9e59); o/i restore detail position (yaks-28b4); single-line inputs scroll horizontally + Source field in the form (yaks-4da6, yaks-b0b4); ask/answer spacing + wrapped detail colouring fixes (yaks-027a, yaks-2d17); instant paste via bracketed paste (yaks-f2aa); slaughter --family + TUI family confirm (yaks-05da); label normalization + doctor malformed-label check (yaks-7cb3); rename attachments from the CLI and TUI (yaks-fe19, yaks-a09b); mouse support (yaks-97a2). Behavior changes: plain CLI slaughter refuses a parent with live descendants; update --source "" clears the source; doctor flags comma/space labels.

---
▸ 2026-09-24T02:30:35Z [Joel Webber]
verify: `cargo test --workspace` -> PASS (exit 0)
