---
id: yaksrs-28af
title: 'update: fields + --note append; body edits'
type: task
priority: 3
created: '2026-08-20T03:26:09Z'
updated: '2026-08-20T03:58:57Z'
parent: yaksrs-6e21
depends_on:
- yaksrs-6b8c
labels:
- rust
---

---
▸ 2026-08-20T03:57:49Z
Starting. Porting cmd_update: --title/--type/--priority/--description, --add-label/--remove-label (append-if-absent / remove-if-present), --source (ignored if empty), --note (append '---\u{25b8} ts note' block); bump updated only if something changed; messages 'Updated X' / 'No changes specified.'. Factoring append_note into store for a unit test + reuse.

---
▸ 2026-08-20T03:58:57Z
Done. update ported: field sets (title/type/priority/description), add/remove-label, source (empty ignored), and --note append via store::append_note; updated bumped only on change; messages match Python ('Updated X' / 'No changes specified.'). 2 append_note unit tests (8 total). Dogfooded on THIS yak entirely with the Rust binary: shave -> update --note -> shorn.
