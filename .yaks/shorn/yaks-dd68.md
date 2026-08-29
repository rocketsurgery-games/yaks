---
id: yaks-dd68
title: 'Parity: list row format (id, type word, glyph, right-aligned labels) + blocked marker'
type: task
priority: 3
created: '2026-08-22T03:44:40Z'
updated: '2026-08-22T04:03:29Z'
parent: yaks-0a93
labels:
- rust
- tui
- parity
---

Match Python row: ' {id}   p{pri} {type-word}   {glyph} {title}   {[labels]right-aligned}'. Investigate + reproduce the magenta * blocked/tangled marker (fixture fix-0004 has an unresolved dep). docs/tui-parity.md #4,#6.

---
▸ 2026-08-22T04:03:29Z
Done. Row = {lead}{indent}{id ljust} p{pri} {type:8} {emoji} {title} … right-aligned [labels] + star slot + collapse badge ▶N. Blocked (hairy w/ unresolved dep) gets a magenta * lead. No left chevron; tree = indentation. Near-identical to Python at 100 wide.
