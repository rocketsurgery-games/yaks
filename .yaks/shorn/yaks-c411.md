---
id: yaks-c411
title: Refresh yak skill + README for the current CLI surface
type: task
priority: 3
created: '2026-09-05T03:18:30Z'
updated: '2026-09-05T03:22:41Z'
parent: yaks-a412
labels:
- skills
- docs
---

Many commands landed this herd; keep the shipped guidance current. Update skills/yak/SKILL.md + README.md to cover: positional-title create + create --json; ask/answer/inbox (needs); multi-id update/reparent + bulk transitions; scan-ids leak check; TUI multi-select (m) + Inbox view + needs badge/accent. Additive + accurate; keep tight.

---
▸ 2026-09-05T03:22:41Z [wt-docs]
Additively refreshed README.md + skills/yak/SKILL.md against the live CLI (verified via yaks --help and per-cmd --help, and tui.rs for TUI claims). README: positional-title create + --json; ask/answer/inbox rows; doctor + scan-ids rows; multi-id transition/update/reparent note; --as attribution; TUI m/a/Inbox/⏳ line. SKILL: create positional+--json, update multi-id+--as, ask/answer/inbox + doctor + scan-ids rows, multi-id note, new 'Asking a human' subsection, scan-ids leak-check in Keep-yaks-private, inbox in --json list. Did NOT document the parallel-lane 'bulk' command.
