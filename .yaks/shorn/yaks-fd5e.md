---
id: yaks-fd5e
title: Pinned yak detail header
type: feature
priority: 3
created: '2026-09-08T22:16:28Z'
updated: '2026-09-10T22:37:20Z'
labels:
- ui
---

When scrolled down in a yak detail, it can be difficult to see exactly which yak you're looking at. It's visible in the list selection, but when the detail view's open, it's usually clipped.

It would be really helpful if the detail view maintained a pinned header whenever the top of the frontmatter is off-screen. I'm thinking of something roughly equivalent to the list view -- id, description, state, type, starred, etc.

![tui-detail-pinned](artifacts/yaks-fd5e/tui-detail-pinned.svg)

---
▸ 2026-09-10T22:37:08Z [agent]
TUI validation output (both channels). VISUAL: this docshot renders the HITL yak's detail scrolled to the end in a short pane — the sticky header '🪒 yaks-b517  feature  p2  HITL…' pins to the top while the frontmatter is off-screen (regenerate: cargo test -p yaks docshots -- --ignored). TEXT: headless snapshot test tui::headless::tests::detail_pins_sticky_header_once_scrolled asserts the state header flips pinned=no -> pinned=yes after scrolling (G); the insta snapshot detail_find_overlay now shows the pin row. Suite green (7+236+25+1), clippy clean (no new warnings).

---
▸ 2026-09-10T22:37:20Z [agent]
Shorn. Implemented the sticky detail header: when detail_scroll>0, render_detail reserves the top row (content shifts down one; nothing hidden) and draws pinned_header_line — a faint background band with status emoji, id, type, priority, and bold title. state_header now exposes 'detail_scroll=N · pinned=yes/no' so headless frames self-document. Validated both channels (text: new headless test; visual: docshot attached). docs/tui.md updated + screenshot embedded. Files: src/tui.rs, src/tui/docshots.rs, src/tui/headless.rs, src/snapshots (detail_find_overlay accepted — it scrolls, so the pin now shows), docs/tui.md, docs/assets/tui-detail-pinned.svg.
