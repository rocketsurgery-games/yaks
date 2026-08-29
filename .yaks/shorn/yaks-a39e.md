---
id: yaks-a39e
title: StyleEncoder abstraction + --style-encoding flag
type: task
priority: 2
created: '2026-08-22T13:29:05Z'
updated: '2026-08-22T14:38:10Z'
parent: yaks-9b8d
labels:
- tui
- eval
---

Pluggable trait so all encodings share the headless harness, fixtures, and Q&A. Flag: --style-encoding {parallel|interleaved|ruler|runlist|spans|doublewidth|diff}. Encoder takes the rendered Buffer (+ layout Rects for optional region scoping) and returns text. Foundational; the encoding yaks depend on it.

---
▸ 2026-08-22T14:38:10Z
Done: StyleEncoding enum (parallel/interleaved/spans) + --style-encoding flag (implies --style; parallel default). Encoders in src/tui/headless.rs operate on the ratatui Buffer; spans preserves literal whitespace (load-bearing) and keys runs by visual StyleKey with a legend. 93 unit + 19 CLI tests green, warning-free; verified end-to-end on the real herd incl. bad-encoding error. frame-diff (9f43) + portable bundle (d487) remain separate.
