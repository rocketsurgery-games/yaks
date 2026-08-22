---
id: yaksrs-5283
title: 'Encoding: ruler / coordinate anchoring (add-on)'
type: task
priority: 3
created: '2026-08-22T13:29:05Z'
updated: '2026-08-22T14:43:39Z'
parent: yaksrs-9b8d
labels:
- tui
- eval
---

Prefix a column ruler and row indices so the model reads coordinates instead of counting whitespace runs (which merge unpredictably under BPE). Combinable with any grid encoding. Hypothesis: high leverage for precise spatial / style-lookup questions.

---
▸ 2026-08-22T13:55:33Z
Found + fixed an off-by-one bug in the ruler encoder (header prefix 6 vs row prefix 7 -> ruler was shifted one col from the grid). On the strong model the ruler added no measurable accuracy (everything already saturates) while costing the most tokens (3.85x plain). Verdict: only worth its cost if a weaker model needs coordinate anchoring; re-test there.

---
▸ 2026-08-22T14:43:38Z
Slaughter: evaluated, not worth building into Rust. ruler cost 3.85x plain tokens with no accuracy gain on a frontier model (docs/tui-style-eval.md). Revisit only if a cross-family study needs coordinate anchoring.
