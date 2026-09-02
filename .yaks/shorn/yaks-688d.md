---
id: yaks-688d
title: Informal yak-linking affordances
type: feature
priority: 3
created: '2026-08-26T13:32:33Z'
updated: '2026-08-30T01:40:43Z'
labels:
- ui
- links
---

While we have formal parentage and dependencies among yaks, sometimes it's useful just to be able to mention a yak in a desc/comment and link to it.
We could require formal `[yak-123]` link structures to make it more markdown-friendly, or just detect the `yak-123` pattern with the configured prefix.
Then we can add the ability to follow them like any other link, and autocompletion during editing, which would save a lot of copying/pasting/memory.

---
▸ 2026-08-29T17:51:24Z
Remember to address these tools as a combination of CLI tools and TUI affordances layered atop them, when appropriate.

---
▸ 2026-08-29T17:51:57Z
And required skill updates as well, so agents know how to use the tools.

---
▸ 2026-08-29T18:37:42Z
Decomposition (planning; pre-review). Reframed the herd around a shared yak-REFERENCE model rather than just informal linking, after pulling in 15f7 (multi-herd). Added two children: yaks-0187 (reference resolution core — the shared resolver everything sits on) and yaks-3563 (agent skill updates). Build order via deps: 0187 -> {c2c5 render, 0af1 rename} ; 0af1 -> 7a92 (single-yak rename generalizes to bulk prefix rename) ; 0187 -> 15f7 ; {c2c5, 7a92} -> 3563 skills.

Key finding: informal linking already half-exists (detail.rs::scan_body_links, wired across desc+comments) and already made the right call — validate candidates against the real id set, prefix-AGNOSTIC. So the "formal markup vs prefix-regex" question in this desc is effectively settled; codify the validation approach in 0187.

OPEN DECISION for review: config prefix is now "yaks" but disk is 160 yaksrs- / 3 yaks-, and generate_id keeps widening the split (these two new yaks are yaks-* too). Do we migrate yaksrs->yaks or revert config to yaksrs? 7a92 needs this call.

RESIDUAL not yet split out: the parent asks for follow + autocomplete (editing-time). Folded into c2c5 scope note for now; split into its own child once 0187 lands if it grows.

---
▸ 2026-08-29T18:42:14Z
DECISION (user): migrate yaksrs- -> yaks-. The yaksrs prefix was an accidental side-effect of the python->rust rewrite. We will DOGFOOD the rename tool (yaks-7a92) to perform the 160-file migration — that migration is its acceptance test. Autocomplete/follow stays folded into yaks-c2c5 for now (tracked in its scope note). Starting pen-to-paper on yaks-0187 (reference resolution core).
