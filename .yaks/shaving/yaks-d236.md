---
id: yaks-d236
title: 'yaks doctor --strict: evidence-before-shear predicate'
type: feature
priority: 2
created: '2026-09-06T01:55:43Z'
updated: '2026-09-06T01:55:43Z'
parent: yaks-a412
labels:
- cli
---

Mechanize a skill-adherence predicate as a doctor check: 'no shear without recorded evidence'. Add --strict to yaks doctor that additionally flags shorn/dead yaks with NO note (the working-a-yak evidence-before-shorn rule). Read-only; exits non-zero when strict issues found.

---
▸ 2026-09-06T01:55:43Z [coordinator]
[assignment] owner: wt-doctor | scope: src/herd.rs + src/main.rs | task: add --strict to 'yaks doctor' flagging shorn/dead yaks lacking any note (evidence-before-shear). one commit; move shaving->shorn at the end.
