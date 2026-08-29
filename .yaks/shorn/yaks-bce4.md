---
id: yaks-bce4
title: 'Style/color parity pass: priority palette, subtle cursor highlight, id/type/label colors'
type: task
priority: 2
created: '2026-08-22T20:06:04Z'
updated: '2026-08-22T20:14:06Z'
parent: yaks-0a93
labels:
- rust
- tui
- parity
---

Match the Python palette more closely (colors.py): priority P1 red/P2 magenta/P3 yellow/P4 green/P5 blue; id=blue, type=cyan, label=magenta; cursor highlight should be a subtle dark-gray bg (256-color 237, default fg) rather than the obtrusive black-on-cyan reverse; ghost badges hairy=yellow+bold/shorn=green/shaving=gray+dim; link=blue, match=black-on-yellow. Compare Rust vs Python via --style snapshots.

---
▸ 2026-08-22T20:14:06Z
Matched the Python palette (colors.py), verified via --style snapshots. Added priority_style: P1 red+bold/P2 magenta/P3 yellow/P4 green/P5 blue+dim (was flat gray). Type cyan (was olive Rgb); labels magenta+dim (was gray). Selection highlight is now a subtle Indexed(237) bg with fg reset to default (Python C_SELECTED) instead of black-on-cyan reverse; unfocused list uses 236. Applied the same subtle bg to fuzzy/view pickers and the chip cursor (keeps green fg). Detail current-link = blue on 237 bold+underline (C_LINK_SEL). Colours live only in --style snapshots so no insta *.snap changed. Added priority_palette_matches_python unit test. 110 unit + 19 CLI green, no new clippy. docs/tui-parity.md #14.
