---
id: yaks-548b
title: 'TUI needs affordances: list badge, inbox filter/view, ask/answer keybindings'
type: feature
priority: 3
created: '2026-09-03T17:59:19Z'
updated: '2026-09-03T20:10:42Z'
parent: yaks-594b
depends_on:
- yaks-4e8a
- yaks-1edb
labels:
- ui
---

Make needs actionable in the TUI: (1) a list-row badge/indicator for needs-blocked yaks (parallels the CLI row marker); (2) a filter/saved-view for the inbox (yaks awaiting a human); (3) keybindings to ask (block + prompt for a question) and answer (clear + prompt for a reply) from the detail/list, reusing the CLI verbs. Builds on detail rendering (yaks-4e8a) and the comment-actor work (yaks-1edb).

---
▸ 2026-09-03T20:10:42Z [wt-tui]
Done — all three parts landed in src/tui.rs.
(a) List badge: list_item() gains a needs_blocked param; a yellow hourglass badge renders on the right for any yak with needs set (disp_width teaches U+23F3 width 2). Distinct axis from the dep-blocked '*' lead, so they co-occur. Test: needs_badge_renders_on_a_blocked_row.
(b) Inbox: new App.inbox_only flag toggled by 'i' (list pane); rows() returns a flat, id-sorted list of every yak with needs.is_some() across all statuses (mirrors herd.inbox), overriding the active view. No FilterSpec change. Test: inbox_toggle_shows_only_needs_blocked_across_statuses (incl. a shorn-but-blocked yak surfacing).
(c) Ask/answer: EditAction::Ask/Answer reuse the Overlay::Edit single-line editor plumbing (like open_comment); context-sensitive 'a' answers if needs set else asks; set_needs_edit() wraps herd.set_needs (needs=human on ask, cleared on answer) + records the typed text as an attributed note via actor::resolve(None). Bound in both list & detail panes; help + entries added. Tests: ask_raises_needs_and_records_the_question, answer_clears_needs_and_records_the_reply.
Evidence: cargo build --release clean; cargo test --release = 215 passed + 20 integration passed (was 211; +4 new).
