---
id: yaks-dd85
title: 'Harden skills: require real-artifact validation + UI evidence before shear'
type: task
priority: 3
created: '2026-09-22T00:40:20Z'
updated: '2026-09-22T00:42:47Z'
labels:
- skills
- docs
---

Motivated by yaks-71d1 shipping half-done (H key: no help entry, wrong edit surface) despite 'tests pass'. The rich evidence contract lives only in skills/dev/yaks-working (repo-internal, NOT shipped), so agents in any yaks project reading the bundled skills/yaks/SKILL.md get only a weak 'note + shorn' step. Bake evidence-before-shear into the shipped skill: verify against the real artifact (not 'it compiles'), record a verify PASS when scriptable, and for user-facing/UI changes attach a screenshot OR a text serialization of the affected state + check every surface the change touches (in-app help, --help, docs). Also sharpen yaks-working's UI clause and cross-check AGENTS.md. Guarded by cargo test -p yaks skills (embedded==on-disk).

---
▸ 2026-09-22T00:42:41Z [agent]
Shipped skill (skills/yaks/SKILL.md): new 'Evidence before you shear' section — scriptable check → verify: command; user-facing/UI change → attach a screenshot OR text serialization and update every exposed surface (in-app help, --help, docs); no lever → ask. Workflow step 4 now points to it. yaks-working (repo-internal): added an explicit UI-evidence bullet citing the 71d1 H miss. AGENTS.md 'Verifying changes': added the attach-the-frame habit + prefer a docshots scene for durable states. Guarded by cargo test -p yaks skills (embedded==on-disk); full suite green.

---
▸ 2026-09-22T00:42:42Z [agent]
verify: `cargo test -p yaks skills` -> PASS (exit 0)
