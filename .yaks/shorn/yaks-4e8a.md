---
id: yaks-4e8a
title: TUI detail view renders the needs field
type: feature
priority: 2
created: '2026-09-03T17:59:04Z'
updated: '2026-09-03T18:28:07Z'
parent: yaks-594b
labels:
- ui
---

detail.rs build() renders Title/Status/Type/Priority/Created/Updated/Labels/Depends/Blocks/Parent/Children/Source but NOT needs. Add a needs line near Status (it is block state), styled as a warning/accent when set. Smallest possible slice; mirrors the CLI show 'needs:' line already added under b517.

---
▸ 2026-09-03T18:28:07Z [coordinator]
BUILT. detail.rs build() now inserts a 'Needs:' field immediately after Status when task.needs is set (mirrors the CLI show line). Test needs_block_renders_after_status_when_set (present+positioned when set, absent when unset); 210 tests green. Accent/warning styling deferred to yaks-548b (needs a new DLine Kind).
