---
id: yaks-a894
title: Reconstruct the yak SKILL (Rust/npm invocation + strengthened guidance)
type: task
priority: 2
created: '2026-08-23T02:04:42Z'
updated: '2026-08-23T02:06:23Z'
parent: yaks-1d54
labels:
- skills
---

Rewrite Running-yaks for npm/binary; update command table to the Rust surface; keep workflow/hard-rules/local-team/task-format (flat IDs + parent field already correct); strengthen external-leak; ADD label-use + terminology + human/agent-collaboration guidance.

---
▸ 2026-08-23T02:06:23Z
Wrote skills/yak/SKILL.md. Changes vs Python: Running-yaks rewritten for npm/binary (yaks / npx yakherder / ./target/release/yaks), dropped uvx+Python. Command table updated to Rust surface incl aliases + create/update flag names (--labels, --add-label/--remove-label, --all). ADDED: Terminology section (0433), 'You are working alongside a human' herd-drift section (86ad), Labels guidance section (8660/efb9). STRENGTHENED external-leak into hard rule 3 + firmer 'Keep yaks private' (5836/5d00). Kept: workflow, parent/child rules, flat-ID task format, source linking, filtering (verified flags against list --help; status/type/priority/label are repeatable Vecs).
