---
id: yaks-c2c5
title: Fix up inter-yak reference rendering/linking
type: feature
priority: 2
created: '2026-08-25T03:35:02Z'
updated: '2026-08-29T22:13:56Z'
parent: yaks-688d
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

---
▸ 2026-08-29T22:13:56Z
SHORN. Rendering/linking is complete, largely delivered by the shared resolver (yaks-0187) + existing detail rendering:
- Detection: detail.rs scan_body_links now goes through refs::token_at + refs::resolve — validation-based, prefix-agnostic, trailing-dot-safe (yaks-9531). Formal parent/dep/child refs link via ref_line.
- Highlight: render_dline styles task links blue + underlined, and the current link blue-on-237 bold+underlined.
- Follow: Tab/[/] cycle (jump_link), Enter follows (follow_link), with forward/back nav.
Covered by existing tests: detail.rs links_parent_deps_children_and_body_refs, body_link_span_points_at_the_id, wrap_remaps_link_columns, url_in_body_is_a_link; tui.rs detail_shows_children_and_links, follow_link_reveals_collapsed_target, detail_tab_cycles_link_lines.

DECISION (dead/unresolved refs): a mention of a non-existent / not-loaded id renders as PLAIN text (no cue), by design — the validation-based model cannot distinguish an intended-but-dangling ref from ordinary prose without false positives, and dangling formal refs are surfaced by `yaks refs` instead.

Autocomplete (the folded-in parent residual) SPLIT OUT to yaks-5656 — it needs overlay nesting, so it is a feature in its own right, not a fold-in. The skill nudge (full-form ids) lives in yaks-3563.
