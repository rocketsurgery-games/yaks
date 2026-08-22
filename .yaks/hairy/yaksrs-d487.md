---
id: yaksrs-d487
title: Eval runner + scoring + portable bundle + write-up
type: task
priority: 2
created: '2026-08-22T13:29:05Z'
updated: '2026-08-22T14:35:43Z'
parent: yaksrs-9b8d
labels:
- tui
- eval
---

Eval runner + scoring + write-up. RESEARCH REPORT lives at docs/tui-style-eval.md (committed, README-seed for a future shared crate). Summary: on a frontier model (Opus 4.8) accuracy saturates across all encodings; token cost decides. Winner: spans (1.96x plain tokens) - robust even on cue-free cumulative vertical-alignment stress to 16 cols/100 wide; interleaved is the aligned-grid fallback (dominates parallel); ruler/runlist not worth the premium; doublewidth dead. Whitespace is load-bearing (never collapse). Open: cross-model-family study via a portable bundle (deferred). Throwaway generator: tools/scratch/style_eval.py (uncommitted).
