---
id: yaksrs-f0c2
title: 'Phase 3 slice: headless agent-driver harness (stdin actions -> text+style+state snapshots)'
type: task
priority: 3
created: '2026-08-22T03:18:46Z'
updated: '2026-08-22T03:22:49Z'
parent: yaksrs-2892
labels:
- rust
- phase3
---

yaks tui --headless: a second driver over the pure render()+handle_key(), rendering into a TestBackend Buffer (no PTY/VT). Line protocol on stdin: key <name> (incl C-<char>), type <str>, snapshot, resize W H, quit. Auto-emit a framed snapshot after each action: state header (focus/view/cursor/sel/overlay) + plain char grid; --style adds an aligned base36 style grid + legend (semantic categories: selection/focus/link/dim). Plain text only. Doubles as the demo-cast substrate + enables differential testing vs the Python TUI. Snapshot tests over scripted sessions.

---
▸ 2026-08-22T03:22:49Z
Done. yaks tui --headless [--size WxH] [--style]: a second driver (tui/headless.rs) over the same pure render()+handle_key(), rendering into a TestBackend Buffer (no PTY). Line protocol on stdin: key <name> (C- prefix for Ctrl), type <text>, snapshot, resize W H, quit; a framed snapshot auto-emits after each action. Frame = state header (focus/view/cursor/sel/overlay via App::state_header) + trailing-trimmed char grid; --style adds an aligned base36 style grid + legend describing semantic categories (selection/focus/link/dim). main.rs gained Tui{headless,size,style} + parse_size. 109 tests (was 103): headless unit tests (header/grid, style layer, key parse, quit) + 2 assert_cmd end-to-end tests with isolated XDG for determinism. Warning-free. Doubles as the demo-cast substrate and enables differential testing vs the Python TUI.
