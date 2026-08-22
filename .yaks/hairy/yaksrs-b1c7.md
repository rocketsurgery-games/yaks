---
id: yaksrs-b1c7
title: 'Encoding: delimited spans (word-preserving inline)'
type: task
priority: 3
created: '2026-08-22T13:29:05Z'
updated: '2026-08-22T14:23:09Z'
parent: yaksrs-9b8d
labels:
- tui
- eval
---

Inline style attached to runs as name[run text], e.g. 1[test widget]. Keeps whole words intact (does not shred them into per-char tokens), so text comprehension survives; cost is loss of alignment. Best-of-inline hypothesis.

---
▸ 2026-08-22T13:41:45Z
Current value winner in the strong-model regime: cheapest style-bearing encoding (259 tok, 1.96x plain) AND 7/7. Preserves whole words so BPE still merges them; only bracket/name overhead added. Caveat: loses column alignment, so may lose exact-spatial questions on harder fixtures - untested.

---
▸ 2026-08-22T13:55:33Z
Held up under pressure: full accuracy on containment, alignment, and confounder scenarios even though the inline form discards 2D column alignment. Combined with lowest token cost (1.96x plain) this is the current default recommendation. Open risk still only at weaker model tiers / full-screen frames.

---
▸ 2026-08-22T14:04:32Z
CAVEAT (load-bearing): spans only survives vertical-alignment questions because inter-run whitespace is emitted literally. Do NOT collapse/normalize space runs as a token optimization - that would destroy column recoverability. Keep spaces literal if shipped.

---
▸ 2026-08-22T14:14:38Z
Held at 100-wide/12-col tables incl. aligned control (no bias). Robust for realistic (fixed-width) TUI misalignment because a bug there leaves a locally-detectable single-space gap in the spans form. Not yet proven for cue-free deep summation. Net: strong default for frontier-model UI dev on real layouts.

---
▸ 2026-08-22T14:23:09Z
Survived the cue-free cumulative test to 16 cols / ~100 wide incl. aligned control (no bias). Confirmed as default snapshot encoding for frontier-model UI dev.
