---
id: yaks-ef11
title: 'De novo repo: rewrite README/docs as the canonical yaks, remove port framing + python/perf cruft'
type: task
priority: 1
created: '2026-08-23T14:50:34Z'
updated: '2026-08-23T14:56:01Z'
labels:
- docs
---

Make this read as the only yaks repo. Full README rewrite (drop yaks-rs name, port/spike framing, Python references, startup-perf). Reframe AGENTS.md and Cargo.toml description de novo. Scrub docs/tui-style-eval.md of Python/differential framing; remove docs/tui-parity.md (pure port-comparison). Remove bench/ and tools/ python cruft. Document install (npm + source) and usage, including install-as-a-skill (npx, no plugin).

---
▸ 2026-08-23T14:56:01Z
Done. README fully rewritten de novo (install via npm/@rocketsurgery/yaks + source, quick start with 'mkdir .yaks', concepts, command table, TUI, install-as-a-skill, config). AGENTS.md + Cargo.toml description reframed. Scrubbed all Python/port/yaks-rs/spike/parity references from docs + source comments + the herd.rs schema error message; renamed detail.rs mod parity_tests -> detail_tests. Removed bench/ (startup perf), tools/ (py_tui_capture + scratch generator), docs/tui-parity.md; scrubbed docs/tui-style-eval.md. Build + tests green, warning-free.
