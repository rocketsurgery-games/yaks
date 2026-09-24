---
id: yaks-b2ed
title: Autocompletion UI infrastructure
type: task
priority: 3
created: '2026-08-26T13:27:01Z'
updated: '2026-09-23T21:38:18Z'
labels:
- ui
needs: human
---

We could use this when typing in both single-line fields, and text blocks.
We'll need to work out the visual and triggering affordances in both contexts. Including space-constrained single-line completions, which could get a little tricky.
Two immediately obvious use-cases:
- Label completion (once we have predefined labels)
- Yak-id completion, in desc/comment blocks

---
▸ 2026-09-23T20:10:53Z [Joel Webber]
I *think* this one might already be done, with the inline yak tab-completion stuff?

---
▸ 2026-09-23T21:38:18Z [coordinator]
Partly done, not fully: yaks-5656 shipped Tab-triggered yak-id completion inside editor blocks (desc/comment), reusing the fuzzy picker as the popup. Not yet built: completion in single-line fields (e.g. labels), which is blocked on predefined labels (yaks-f04d). No inline/ghost-text popup infra exists; completion is modal via the picker. Options: (a) shear b2ed as covered by yaks-5656 and file 'label completion' under yaks-f04d [my lean]; (b) keep b2ed open for single-line completion infra.
