---
id: yaks-b517
title: User feedback comments the agent can find reliably
type: idea
priority: 3
created: '2026-08-23T02:49:59Z'
updated: '2026-09-03T16:39:10Z'
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

---
▸ 2026-09-03T16:39:05Z [coordinator]
BUILT + verified. needs-human as a soft block on the readiness gate.
- New Task.needs: Option<String> (frontmatter 'needs: <who>'; parse + canonical write; back-compatible — absent field = unblocked, no migration). filter: ready_only now also excludes needs.is_some(), so a blocked yak drops out of 'next' exactly like an unmet dep. No new status.
- Commands: 'yaks ask <id> --note ... [--needs human]' sets the block + records an attributed question (uses eb66 --as). 'yaks answer <id> --note ...' clears it + records the reply (human-reserved by convention). 'yaks inbox' lists needs-blocked yaks. needs shows in 'show' + json.
- e2e verified on a throwaway: create->next(shown)->ask->next(HIDDEN)->inbox(shown)->show(needs:human + [coordinator] question)->answer->next(RESTORED)->inbox(empty). 205 yaks + 20 CLI + 13 toque tests green; added filter test ready_only_excludes_a_needs_block.
FOLLOW-UPS: (1) TUI inbox filter + badge (CLI is the durable part; done here). (2) inbox rows show only the yak, not the ask inline — enrich to show the latest question, or point to show/log. (3) --needs is generic (defaults human) so other external blockers (e.g. review) are expressible now. (4) clearing is convention-guarded, not enforced; the skill reserves answer for humans.
