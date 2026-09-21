---
id: yaks-98d5
title: 'Farm config: herds map + one-level global->herd cascade'
type: feature
priority: 3
created: '2026-09-21T17:32:32Z'
updated: '2026-09-21T18:57:49Z'
parent: yaks-15f7
labels:
- cli
---

Deferrable. Config is single-valued today (one default_type and default_priority, one verify map resolved by label). A farm spanning projects with different levers eventually wants per-prefix config: default type and priority per herd and verify resolution keyed by herd as well as label. Incremental; label-based verify plus per-yak source already cover most of the gap, so this lands after the MVP proves out.

---
▸ 2026-09-21T18:37:07Z [agent]
DESIGN (session with Joel). Add a herds: map to .yaks/config.yaml declaring the predefined prefix set; each entry is a PARTIAL config overriding globals for that herd. Top-level prefix: stays the DEFAULT herd (one of the herds; if herds: is absent, a single implicit herd = prefix, so existing configs are unchanged). Resolution cascades ONE level, global -> herd, evaluated per lookup: value(setting, herd) = herds[herd].setting else global.setting else builtin. For map-valued settings (verify by label; label defs for 640a/f04d) the same cascade applies at the lookup key, so a herd can add/override one lever or label without redeclaring the rest. Per-herd-capable fields: default_type, default_priority, verify, and label definitions (enables 640a per project); a per-herd source/tracker base is a natural future field (the mixed Jira/GitHub motivation) so rollup/seed know each herd's tracker. The herds: keys are the KNOWN-HERD SET that powers the UI herd picker (no re-typing prefixes) and the TUI current-herd (yaks-3290); CLI --prefix stays free-form but can warn on an unknown herd. Parser: hand-rolled small-YAML; verify already nests one level, so a herds: map-of-maps is a modest extension. Relates: 640a (label defs) and f04d (known labels) become per-herd via this cascade; 3290's UI picker/current-herd consumes the known set.

---
▸ 2026-09-21T18:57:49Z [agent]
SHORN. Farm config models herds with a one-level global->herd cascade. store::Config gains herds: map<prefix, HerdConfig{default_type?, default_priority?, verify}> plus known_herds()/default_type_for()/default_priority_for()/resolve_verify(labels, herd). read_config extended with a small indent-aware pass over a herds: map of per-herd blocks (2/4/6-space nesting: name/fields/verify labels); back-compat preserved (no herds: => one implicit herd = prefix). create routes default_type/default_priority through the chosen herd; the verify command resolves via the yak's herd (id prefix) then global. known_herds() is the picker known-set (consumed by yaks-3290; allow(dead_code) until then). Tests: store herds parse+cascade, farm create-applies-herd-default-type; suite green (252 lib + 25 cli), warning-free. Verified end-to-end on a scratch farm: web yak got feature+p3, core yak got p1+task; yaks verify resolved WEB-LEVER (herd) vs GLOBAL-LEVER (global). Docs+skill updated with the herds: block + cascade. Sequenced before yaks-3290 so its picker/current-herd consume known_herds() with no stopgap. Future entries in this same cascade: label defs (yaks-640a) and a per-herd source/tracker.
