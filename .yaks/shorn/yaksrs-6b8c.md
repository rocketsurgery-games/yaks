---
id: yaksrs-6b8c
title: 'Task model: full fields + frontmatter serialize (round-trip save)'
type: task
priority: 2
created: '2026-08-20T03:26:09Z'
updated: '2026-08-20T03:34:47Z'
parent: yaksrs-6e21
labels:
- rust
---

---
▸ 2026-08-20T03:30:58Z
Starting. Matching the Python serializer (yaklib.model.dump_yaml/save_task): frontmatter field order id,title,type,priority,created,updated,parent,depends_on,labels,source; priority as plain int; timestamp-like + YAML-ambiguous strings single-quoted (with '' escaping); block lists at column 0; body after '---' with a leading blank line; atomic temp+rename. Adding created/updated to Task, a serializer, save(), and round-trip tests.

---
▸ 2026-08-20T03:34:47Z
Done. Added created/updated to Task (opaque ISO strings, preserved on round-trip). store::write: render() + atomic save() (temp+rename) emitting the Python dump_yaml subset — field order id,title,type,priority,created,updated,parent,depends_on,labels,source; plain-int priority; single-quoted timestamps + YAML-ambiguous scalars with '' escaping; column-0 block lists. Parser now captures created/updated; unquote handles '' unescaping. 4 unit tests pass (round-trip, quoting+escaping, plain-when-safe); release build warning-free. Cross-tool interop (Python reading Rust-written files) will be proven when create lands (yaksrs-a2a4). write module carries allow(dead_code) until then.
