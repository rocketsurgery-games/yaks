---
id: yaks-8d58
title: Remove toque text style-encodings + yaks --style flags + headless style tests
type: task
priority: 2
created: '2026-09-09T04:12:12Z'
updated: '2026-09-09T04:54:55Z'
parent: yaks-89b9
labels:
- toque
---

Delete the dual-plane colored-text machinery now that SVG is the visual channel. Scope: crates/toque/src/lib.rs — remove enum StyleEncoding (Spans/Interleaved/Parallel), StyleEncoding::parse, encode_body's style branches, spans_layer/style_layer, StyleRegistry/base36/legend, and the DriverOpts.style field (keep DriverOpts.diff — it's style-independent); toque emits the plain grid + state header only. yaks src/main.rs — drop Tui{style, style_encoding} fields + the parse block (~420-426, ~987-1015). src/tui/headless.rs — drop the 3 style tests (parallel/spans/interleaved 'legend:' asserts, ~105-121) + module-doc mention of 'style encoders' (line 3). Also remove the toque lib.rs unit tests for encodings (~657-749). Verify: cargo test --workspace green; committed .snap fixtures already plain so unaffected. NOTE: coordinate with the docs child (yaks-<C>). Independent of the renderer child (yaks-dce6).

---
▸ 2026-09-09T04:54:55Z [agent]
Shorn. Removed toque's text style-encodings: enum StyleEncoding (+parse), DriverOpts.style, encode_body's style branches, spans_layer/style_layer, StyleRegistry/base36/legend/color_name/StyleKey. SnapshotEncoder is now plain-grid only (unit struct + Default). Dropped --style/--style-encoding from 'yaks tui' (src/main.rs). Removed 3 headless.rs style tests + 5 toque lib style tests. cargo test --workspace green (7+235+25+1); clippy clean; 'tui --help' no longer lists style flags.
