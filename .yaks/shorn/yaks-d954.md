---
id: yaks-d954
title: Document yaks verify + docs-freshness rule in AGENTS.md
type: task
priority: 2
created: '2026-09-08T02:34:09Z'
updated: '2026-09-08T02:35:26Z'
parent: yaks-6624
labels:
- docs,cli
verify: cargo test -p yaks skills
---

---
▸ 2026-09-08T02:34:15Z [coordinator]
[accept] Done = (1) docs/cli.md documents 'yaks verify <id>...' + the --verify flag on create/update; (2) the yaks skill's Commands table gains a verify row and mentions --verify on create/update; (3) AGENTS.md gains a concise docs-freshness rule (a user-facing CLI/TUI/behavior change updates docs/ + the bundled skills in the SAME change). Evidence: quote the key additions; 'yaks verify yaks-d954' (verify: = cargo test -p yaks skills) -> PASS. Judge: coordinator (scriptable + re-read).

---
▸ 2026-09-08T02:35:11Z [coordinator]
verify: `cargo test -p yaks skills` -> PASS (exit 0)

---
▸ 2026-09-08T02:35:26Z [coordinator]
Shorn. docs/cli.md: new '## Verification' section documenting 'verify <ids>' + --verify on create/update rows. yaks skill: added a 'yaks verify' Commands row + --verify on create/update. AGENTS.md: new '## Keeping docs current' section (user-facing change updates docs/ + bundled skills in the SAME change; cargo test -p yaks skills guards the skills, docs/ has no gate so parity is part of the evidence). Verified: 'yaks verify yaks-d954' (verify: cargo test -p yaks skills) -> PASS.
