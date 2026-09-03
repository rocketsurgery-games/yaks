---
id: yaks-29a3
title: Render preserved unknown frontmatter fields in CLI show
type: feature
priority: 3
created: '2026-09-03T19:58:25Z'
updated: '2026-09-03T20:06:12Z'
parent: yaks-594b
labels:
- cli
---

yaks-031d preserves unmodeled frontmatter (Task.extra) on round-trip but nothing renders it, so a hand-added key (Joel added 'wat: foo' to yaks-b517) is invisible. Surface Task.extra read-only in 'yaks show' (a small section, e.g. 'Other fields:' listing the raw lines) and include it in json show_value. Read-only: do NOT let show become an editor. Scope: src/main.rs (render_show) + src/json.rs. Pairs with the TUI-detail counterpart on the other lane.

---
▸ 2026-09-03T20:06:12Z [wt-cli]
Done. render_show (src/main.rs) now prints an 'Other fields:' section listing task.extra verbatim after needs/source, before body; json::task_value (src/json.rs) emits an 'extra' array when non-empty (omitted when empty, so goldens unchanged). Read-only, not an editor. Verified on a throwaway with hand-added 'wat: foo': text show renders 'Other fields:\n  wat: foo'; show --json emits "extra": ["wat: foo"]. Unit tests task_value_surfaces_extra_when_present + task_value_omits_extra_when_empty pass; full suite 214 unit + 20 cli green.
