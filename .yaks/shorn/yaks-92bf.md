---
id: yaks-92bf
title: 'Terminology migration: herd->farm, HerdScope->family; herd now names a prefix-group (code+docs+skills)'
type: task
priority: 2
created: '2026-09-21T17:32:32Z'
updated: '2026-09-21T17:57:04Z'
parent: yaks-15f7
labels:
- docs
- skills
---

User-facing terminology and usage sweep for consolidation. Proposed vocabulary: farm=the whole .yaks repo, herd=a prefix-group within it, family=a yak-tree (parent plus descendants). NOTE the collision: herd currently means the whole .yaks across docs, skills, and CLI help. Decide whether to redefine herd or pick a fresh word, and whether to sweep now or incrementally. Then document the consolidation model, external-farm discovery recipe, create --prefix, merge, and per-herd slicing in docs AND the bundled skills (same-change parity, guarded by cargo test -p yaks skills). Blocked on the terminology ask.

---
▸ 2026-09-21T17:32:51Z [agent]
Terminology gate before names get stamped into surfaces. Proposed: farm=whole .yaks, herd=prefix-group, family=yak-tree. Heads-up: herd currently means the whole .yaks everywhere in docs/skills/CLI help, so this redefines it and adds farm for the whole thing. Two questions: (1) redefine herd, or pick a fresh word to avoid churn and confusion? (2) full rename sweep across docs+skills+help now, or adopt incrementally as each surface is touched? Meanwhile I am landing create --prefix (accurate regardless; can gain a --herd alias) so mechanics are not blocked.

---
▸ 2026-09-21T17:39:57Z [Joel Webber]
Let's just bite the bullet and rename everything, throughout the code, docs, and skills. No other users yet anyway.

---
▸ 2026-09-21T17:49:35Z [agent]
Joel answered: bite the bullet, rename everything across code, docs, and skills (no other users yet). MAPPING decided during impl. farm = the whole .yaks: store facade Herd->Farm, NoHerd->NoFarm, temp_herd/with_herd->_farm, App.herd handle->farm, prose the-herd->the-farm. family = a yak-tree / descendants: the view scope type HerdScope->FamilyScope (its own doc already said it governs an anchor yak's family), App.herd_scope->family_scope, the tab-row label 'herd: <scope>'->'family: <scope>', help h:herd->h:family, cache UiState.herd field + json key->family. herd (now freed) names a prefix-group within a farm, used by create --prefix and the new concept docs. Kept the h keybinding (mnemonic now stale; revisit separately). Executing as a scripted perl rename over code with the compiler as backstop; hand-editing cache.rs (family field vs farm handle both read as bare 'herd'); regenerating insta snapshots + docshots SVGs (tab-indicator text changes); hand-editing shipped docs/skills prose and adding a Farms/herds/families section.

---
▸ 2026-09-21T17:57:04Z [agent]
SHORN. Terminology migration landed across code, docs, skills. Whole-.yaks facade Herd->Farm (herd.rs->farm.rs, NoHerd->NoFarm, temp_/with_herd->_farm, App.farm handle, OpenError, doctor message, all prose the-herd->the-farm, fixtures/herd->fixtures/farm). Tree/descendant view scope HerdScope->FamilyScope (App.family_scope, cache UiState.family + json key, tab label herd:<s>->family:<s>, help h:herd->h:family, all _scope compounds). Freed herd to mean a prefix-group within a farm; used it in the create --prefix surfaces + a new farm/herd/family concept blurb (README + skill Terminology). Method: scripted perl rename with the compiler as backstop; hand-edited cache.rs (family field vs farm handle both read as bare herd); regenerated 6 insta snapshots + 6 docshot SVGs, removed the orphaned herd_indicator snapshot. Verified: cargo test --workspace green (toque 7 + lib 245 + cli 25 + doctest 1), doctor clean, census shows herd only as the prefix-group meaning + the yak-herding meme. Kept the h keybinding (mnemonic now stale; revisit later).
