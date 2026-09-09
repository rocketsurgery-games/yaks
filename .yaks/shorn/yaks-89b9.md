---
id: yaks-89b9
title: Make plain-text + SVG the two TUI-evidence channels; retire toque's text style-encodings
type: task
priority: 2
created: '2026-09-09T04:11:36Z'
updated: '2026-09-09T04:55:01Z'
labels:
- ui
---

Decision (origin: yaks-1a96; prototype: yaks-dce6). Keep DUAL TUI-evidence output but split the jobs cleanly: (1) PLAIN toque text snapshot = cheap, greppable, diffable, CI/every-step channel — and the semantic style that matters (selection/focus/blocked/overlay) is ALREADY in the state header as facts, so plain text loses nothing semantic; (2) SVG (Option C, lean row-per-line; -66..85%) = the visual channel, rendered to PNG on demand (resolution = a fidelity/token lever). This RETIRES toque's dual-plane colored-text style-encodings (spans/interleaved/parallel + registry/legend): they re-encoded literal appearance that SVG renders natively + better, nothing currently gates real style regressions through them (every committed .snap is plain; only 3 tests assert 'legend:' exists), and maintaining 3 encodings + the fragile whitespace-arithmetic property isn't worth it. Children: promote svg_of into toque (render_to_svg); drop the encodings + yaks --style flags; docs/skills sweep. Caveats to honor in docs: PNG needs a rasterizer (not a free CI gate — style becomes reviewed, not auto-asserted); 'resolution control' is visual fidelity, not cheaper tokens.

---
▸ 2026-09-09T04:55:01Z [agent]
Shorn — all three children landed. Outcome: TUI evidence is now two clean channels (plain toque text snapshot + toque SVG->PNG); the dual-plane colored-text encodings are gone. svg_of promoted into toque (render_to_svg/buffer_to_svg); StyleEncoding machinery + --style flags removed; docs/skills swept. cargo test --workspace green; clippy clean.
