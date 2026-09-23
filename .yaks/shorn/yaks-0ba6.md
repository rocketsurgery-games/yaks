---
id: yaks-0ba6
title: Release v0.0.10
type: chore
priority: 3
created: '2026-09-23T04:46:39Z'
updated: '2026-09-23T04:47:24Z'
---

Cut v0.0.10 to deliver the yaks-1d51 skill-management family. The version bump is functionally required, not ceremonial: auto-upgrade keys on version, so the e233 spec fix and the new stamped skills only reach users at a release. Highlights since v0.0.9: merge now declares incoming herds (yaks-095a); skills frontmatter is spec-compliant and enforced by a test (yaks-e233); installed skills carry a provenance stamp yielding seven states with yaks skills status + a doctor advisory (yaks-bd9a); ordinary commands auto-sync the user-level skills dir, never downgrading and never clobbering local edits; and the symlink clobber that reverted this repo's own skills source is structurally prevented (yaks-d8e9).

---
▸ 2026-09-23T04:47:20Z [agent]
verify: `cargo test --workspace` -> PASS (exit 0)
