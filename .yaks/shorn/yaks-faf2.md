---
id: yaks-faf2
title: Ease visual review of attached SVGs (SVG->PNG render helper)
type: idea
priority: 3
created: '2026-09-10T23:45:35Z'
updated: '2026-09-11T01:05:15Z'
labels:
- meta
---

Coordinating the UI batch, the coordinator cannot cheaply eyeball an attached SVG: agent read_file is in-project only and cannot rasterize SVG, so visual review needs an out-of-band headless-Chrome SVG->PNG dance into a gitignored dir. A helper -- yaks attach --render, a standalone yaks render <svg>, or at least a documented skill snippet -- that drops a PNG beside the artifact would smooth the --ask visual-review loop that TUI yaks now depend on.

---
▸ 2026-09-11T01:05:15Z [coordinator]
Shorn: resolved as a documented RECIPE, not a 'yaks render' subcommand. Rationale: (1) yaks's hard invariant is a single self-contained binary -- shelling to Chrome breaks it; (2) the pure-Rust alternative (resvg) renders our color emoji as tofu, so a browser engine is needed for fidelity regardless -- rasterization isn't yaks's job. Added a 'Rasterizing an SVG to view it' recipe to docs/tui.md (headless Chrome -> PNG in a gitignored dir; window-size from the SVG header; magick compare for pixel-diffs) + a pointer from AGENTS.md 'Verifying changes'. Bonus: closed a docs->reality gap (AGENTS.md already pointed at a docs/tui.md recipe that did not exist). Deferred (not pursued): a pure-Rust 'yaks render' if emoji-capable rasterization ever lands.
