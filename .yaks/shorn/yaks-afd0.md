---
id: yaks-afd0
title: schema/migration gate parity
type: task
priority: 3
created: '2026-08-20T03:26:09Z'
updated: '2026-08-20T19:45:35Z'
parent: yaks-6e21
labels:
- rust
---

---
▸ 2026-08-20T19:45:05Z
Scope (correct-enough, safe): a schema-version gate rather than a full migration engine. SCHEMA=3. schema_status(root): read .yaks/schema; newer than SCHEMA -> hard error + exit (avoid misparsing/corrupting a future format during interop); older -> warn but proceed best-effort; equal/absent/unparseable -> proceed. Full old->new migration parity is deferred (yaks-rs herds are born at v3; the Python tool owns legacy migration until it is retired).

---
▸ 2026-08-20T19:45:35Z
Done. store: SCHEMA=3 + schema_status(root) (Compatible/Older/Newer; missing/unparseable -> Compatible). main gates every command right after discover_root: Newer -> error + exit 1; Older -> stderr warning, proceed; Compatible -> silent. 1 unit test (35 total). Verified live: normal v3 herd is silent; a v9 herd is refused with a clear message + exit 1.
