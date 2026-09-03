---
id: yaks-b517
title: User feedback comments the agent can find reliably
type: idea
priority: 3
created: '2026-08-23T02:49:59Z'
updated: '2026-09-03T16:22:56Z'
labels:
- agent
---

Use user feedback to drive agent behavior. E.g., "look for any new comments I've added and follow up
on them". This requires some kind of formal structure for comments, so that the agent can find them
reliably. Probably best to support this directly through skills and python code, so the agent's work
is limited.

---
▸ 2026-09-03T16:22:56Z
[coordinator] DECISION (design settled, ready to build): model needs-human as a SOFT BLOCK on the existing readiness gate, via a fresh FRONTMATTER FIELD (not a label — a label is too freeform for something load-bearing).
- Field: 'needs: human'. Makes 'yaks next' treat the yak as NOT ready, exactly like an unmet depends_on (needs-human == a dependency on an external human event). The guard falls out for free: agents that trust 'yaks next' never pick it up. No new status (keep the tiny state machine tiny).
- Raise (attributed, uses eb66 actor): 'yaks ask <id> --note "which auth approach?"' sets the field + records the question in one step.
- Human queue: 'yaks inbox' (alias for list filtered to needs=human); TUI filter + badge.
- Resolve (human-reserved by convention): 'yaks answer <id> --note "..."' clears the field + records the reply. next skips blocked yaks; un-blocking is a separate named verb, so agents don't clear their own block.
- Works BOTH directions: working-a-yak's notes-first re-read means a needs-human a human raises on an in-flight yak stops the agent at its next check.
- Notification: CLI is pull ('yaks inbox') now; TUI badge; true push (desktop/hook/'yaks watch') deferred, not core.
Depends on eb66 (the 'ask' note should be attributed). Build eb66 first.
