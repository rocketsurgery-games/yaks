---
id: yaks-766d
title: 'working-a-yak: add needs/ask-answer guidance (hand back, don''t self-clear)'
type: task
priority: 3
created: '2026-09-04T04:23:24Z'
updated: '2026-09-04T04:25:22Z'
parent: yaks-a412
labels:
- skills
---

The skill already has 'Before you start' (re-read notes/feedback first) and 'follow or ask' on redirect. ADD a short paragraph on the needs/ask/answer mechanism now that it's built: on a human decision, raise it with 'yaks ask <id> --note ...' (drops the yak from next) and HAND BACK rather than blocking; never clear your own needs block ('yaks answer' is human-reserved); the human's queue is 'yaks inbox'. Keep it tight and consistent with the existing voice. Scope: skills/dev/working-a-yak/SKILL.md only.

---
▸ 2026-09-04T04:25:19Z [wt-skill]
Added a tight paragraph after the 'Before you start' list in skills/dev/working-a-yak/SKILL.md covering the needs/ask/answer mechanism: 'yaks ask <id> --note' sets the needs block + drops from next -> hand back (don't block-and-wait); pending questions surface in 'yaks inbox'; 'yaks answer' is human-reserved (never self-clear). Verified via git diff (6 added lines, no restructuring). Edit-tool accepted the worktree path; no terminal fallback needed.
