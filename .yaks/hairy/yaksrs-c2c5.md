---
id: yaksrs-c2c5
title: Fix up inter-yak reference rendering/linking
type: feature
priority: 2
created: '2026-08-25T03:35:02Z'
updated: '2026-08-29T18:37:46Z'
parent: yaksrs-688d
depends_on:
- yaks-0187
labels:
- ui
---

Ensure that all references of the form `yaktype-1234` are treated as followable links, and propertly highlighted.

Also include skill guidance nudging agents to use the full form, rather than just the shorthand `1234`, so that they'll tend to work out of the box.

---
▸ 2026-08-29T18:37:46Z
Scope tightened after code read. Detection ALREADY EXISTS (detail.rs::scan_body_links: bare prefix-1234 + [[wiki]], validated against known ids, applied to desc AND comments). Remaining work: (1) consume the shared resolver from yaks-0187 instead of the local copy; (2) highlight styling for detected refs; (3) follow + (editing-time) autocomplete — the parent residual, may split out later. Skill nudge (use full prefix-1234 form) MOVED to yaks-3563. INTEGRITY GAP to decide: refs to slaughtered (dead/) or cross-herd yaks are not in the known set, so they render as inert plain text with no cue — link-as-dead vs leave-plain?
