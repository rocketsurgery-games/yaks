---
id: yaksrs-0a93
title: TUI parity with the Python original (differential pass)
type: task
priority: 2
created: '2026-08-22T03:44:28Z'
updated: '2026-08-22T03:44:28Z'
labels:
- rust
- tui
- parity
---

Match the Rust TUI closely to the Python curses TUI before intentionally evolving. Findings + method captured in docs/tui-parity.md, produced via yaks tui --headless (Rust) vs tools/py_tui_capture.py (Python/pyte). Children are the per-area fixes; close them against re-captured diffs.
