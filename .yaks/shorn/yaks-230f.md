---
id: yaks-230f
title: 'TUI slice 1: skeleton (yaks tui, backend-agnostic render, two-pane nav)'
type: task
priority: 2
created: '2026-08-20T22:58:15Z'
updated: '2026-08-21T01:24:30Z'
parent: yaks-86a3
labels:
- rust
- ui
---

Phase 2 slice 1. yaks tui subcommand; ratatui + crossterm; terminal init/restore + panic hook; kitty keyboard flags when supported. Pure render(&App,&mut Frame) so the same painter targets crossterm or TestBackend. Two-pane (list+detail) + status tabs (Hairy/Shaving/Shorn) + help line; j/k/g/G nav, Tab/[ ] tab switch, l/Enter focus detail, h/Esc back, q quit. Read-only over herd.list. TestBackend + insta snapshot from day one.

---
▸ 2026-08-20T23:00:46Z
Done. yaks tui subcommand + tui module (ratatui 0.30 + crossterm). Terminal init/restore + panic hook + kitty DISAMBIGUATE flags when supported. Pure render(&App,&mut Frame): two-pane list+detail, status tabs (Hairy/Shaving/Shorn) with counts, focus-aware help; j/k/g/G nav, Tab/[ ] tab switch, l/Enter focus detail, h/Esc back, q/Ctrl-C quit. Read-only over herd.list. Backend-agnostic render -> 2 TestBackend+insta snapshot tests (37 total), warning-free. Live interactive run not automatable here (needs a real tty); snapshots cover render + focus. Next: tree+collapse, then mutations via Herd.
