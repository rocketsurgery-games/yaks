---
id: yaksrs-1c5c
title: Eval fixtures + programmatic gold Q&A battery
type: task
priority: 1
created: '2026-08-22T13:29:05Z'
updated: '2026-08-22T15:05:48Z'
parent: yaksrs-9b8d
labels:
- tui
- eval
---

Fixture set spanning difficulty + a question battery split by category: comprehension, alignment, style-lookup, connectivity / grouping. Gold answers derived programmatically from the Buffer (no hand labeling, no drift). Includes a plain-grid-only control (no style) and a B / state-header control to establish the ceiling per-cell encodings must beat.

---
▸ 2026-08-22T13:41:45Z
Built throwaway generator tools/scratch/style_eval.py (uncommitted): one source-of-truth fixture -> plain grid + 6 encodings + programmatic gold; 'sizes' mode reports real tiktoken o200k_base tokens. Discriminating list fixture + 7-question battery. Real tokens vs plain-only (132): spans 259 (1.96x), interleaved 338 (2.56x), parallel 364 (2.76x), runlist 451 (3.42x), ruler 508 (3.85x), doublewidth 836 (6.33x).

---
▸ 2026-08-22T13:55:33Z
Generator extended: box + list + 3 cliff-hunt scenario fixtures (align/contain/confound) with programmatic gold; modes all/sizes/doc/scenario. Reference dump at tools/scratch/encodings-sample.md. Next: emit a portable JSON eval bundle (frames+questions+gold) for out-of-band weaker-model runs.

---
▸ 2026-08-22T15:05:48Z
Delivered: throwaway generator tools/scratch/style_eval.py (uncommitted) emits every encoding + programmatic gold for the fixture/scenario/valign/vartable batteries; findings in docs/tui-style-eval.md. Further extension (portable cross-family bundle) tracked in d487.
