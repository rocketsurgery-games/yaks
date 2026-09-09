---
id: yaks-2ed9
title: 'Docs/skills sweep: retire dual-plane style-snapshot references'
type: task
priority: 3
created: '2026-09-09T04:12:26Z'
updated: '2026-09-09T04:54:55Z'
parent: yaks-89b9
depends_on:
- yaks-dce6
- yaks-8d58
labels:
- docs
---

Purge/repoint everything that still describes toque's text style-encodings. Known surface (mapped): crates/toque/README.md — rewrite: drop the whole 'Style encodings' section + eval-summary table + the StyleEncoding quick-start line; reframe toque as two projections (plain text = cheap/diffable; SVG = visual). AGENTS.md lines 30/32 — update the toque one-liner (no 'per-cell style' encoders) and FIX the stale pointer 'docs/tui-style-eval.md' -> 'docs/research/tui-style-eval.md'. crates/toque/README.md line 89 has the same stale path. docs/research/tui-style-eval.md — keep as an ARCHIVED finding (it justified both the original default AND this reversal); add a header note that the encodings were retired + why, so it reads as history not current guidance. docs/tui.md — verify the docshots/'generated headlessly' blurb still matches after svg_of moves into toque (yaks-dce6); update the toque link/《wording if needed. Bundled skills (skills/yaks, skills/yaks-tracker, skills/dev/*) already carry NO style-encoding refs — confirm still true (guarded by cargo test -p yaks skills). Depends on the renderer + removal children so docs describe the end state.

---
▸ 2026-09-09T04:54:55Z [agent]
Shorn. toque/README.md: 'Style encodings' section replaced with 'Visual output (SVG)' (render_to_svg/buffer_to_svg) + a History note; quick-start + frame example de-styled. AGENTS.md: toque one-liner de-styled + stale pointer fixed (docs/tui-style-eval.md -> docs/research/tui-style-eval.md). docs/research/tui-style-eval.md: added Archived(historical) header. docs/tui.md: renderer attribution updated (lives in toque now). Bundled skills confirmed clean (no style refs; skills guard passes). Final repo grep: only intentional historical refs remain.
