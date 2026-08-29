---
id: yaks-0a93
title: TUI parity with the Python original (differential pass)
type: task
priority: 2
created: '2026-08-22T03:44:28Z'
updated: '2026-08-22T21:26:34Z'
labels:
- rust
- tui
- parity
---

Match the Rust TUI closely to the Python curses TUI before intentionally evolving. Findings + method captured in docs/tui-parity.md, produced via yaks tui --headless (Rust) vs tools/py_tui_capture.py (Python/pyte). Children are the per-area fixes; close them against re-captured diffs.

---
▸ 2026-08-22T21:26:34Z
All 22 parity children shorn; Rust TUI at genuine parity with the Python original. docs/tui-parity.md documents the method and per-area findings across §1-§15.
