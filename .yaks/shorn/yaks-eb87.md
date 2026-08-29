---
id: yaks-eb87
title: 'Differential-testing tool: capture Python curses TUI via PTY+pyte'
type: task
priority: 3
created: '2026-08-22T03:33:06Z'
updated: '2026-08-22T03:33:17Z'
parent: yaks-2892
labels:
- rust
- phase3
---

tools/py_tui_capture.py drives the Python (curses) yaks TUI under a real pseudo-terminal and scrapes the emulated screen with pyte into the SAME framed grid format as yaks tui --headless, so one stdin script (key/type/snapshot/resize/quit) drives both and we can diff. PEP723 (pyte); --launch defaults to uv run (resolves yak.py PEP723 pyyaml) or point at a python with pyyaml for speed. Enables cataloguing + ironing out TUI differences from the original.

---
▸ 2026-08-22T03:33:17Z
Done. tools/py_tui_capture.py working end-to-end against tests/fixtures/herd: PTY + pyte, readiness wait for first curses paint, key translation (names + C- ctrl + arrows/page/etc), child stderr captured separately. Same frame markers as the Rust harness (minus the state header, which is internal to Rust). Verified keys drive it (l -> Python detail pane). Immediately surfaces differences: emoji tab bar with (N) counts + wrapping, id-first list rows with emoji glyphs + labels shown, detail header "Task: id" + "Blocks:" + humanized dates + Title:/Status:/... field labels, different help lines. Known pyte artifact: the detail vertical divider renders as x (DEC line-drawing charset not mapped) — cosmetic.
