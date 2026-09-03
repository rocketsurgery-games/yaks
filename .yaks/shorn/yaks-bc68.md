---
id: yaks-bc68
title: inbox must show every needs-blocked yak (status-independent); ask should guard finished yaks
type: bug
priority: 2
created: '2026-09-03T17:59:04Z'
updated: '2026-09-03T18:22:18Z'
parent: yaks-594b
labels:
- cli
---

FOUND BY DOGFOODING: 'yaks ask yaks-b517' set needs=human on a SHORN yak, but 'yaks inbox' (currently hairy+shaving only) never shows it -> a silent, invisible block. Invariant to enforce: if needs is set, the yak MUST appear in inbox regardless of status. Fix: inbox filters on needs.is_some() across all statuses (or at least not-dead). Separately, 'ask' on a shorn/dead yak is almost always a mistake -> warn (or refuse) when blocking finished work. Decide: is a needs block on a done yak ever meaningful? Likely no.

---
▸ 2026-09-03T18:22:17Z [coordinator]
BUILT + verified. inbox invariant: herd.inbox now includes all statuses (was hairy+shaving), so a needs block is never invisible — the exact gap Joel hit (ask on shorn yaks-b517 -> silent). ask guard: set_needs returns the yak's status; ask warns to stderr when blocking a Shorn/Dead yak ('did you mean a hairy yak?') but still proceeds (non-blocking, and now visible in inbox). answer unchanged. Test inbox_shows_blocked_yaks_regardless_of_status (shorn block included, unblocked excluded); 207 tests green. e2e: ask on a shorn throwaway warned AND appeared in inbox.
