---
id: yaksrs-9dae
title: 'Meta-probe: can current LLMs read character-grid layout at all?'
type: task
priority: 1
created: '2026-08-22T13:29:05Z'
updated: '2026-08-22T14:23:09Z'
parent: yaksrs-9b8d
labels:
- tui
- eval
---

Thinnest end-to-end eval loop and biggest unknown. Before investing in style encoders, measure how well current models read a plain char grid: comprehension (what / how many), alignment (equal widths, left / center / right), grouping, and connectivity (is a box closed). Fixtures escalate difficulty (single box, side-by-side boxes, nested, wide list). Ground truth derived programmatically from the Buffer. Calibrates every later encoding: style channels only matter if spatial reading works. Hypothesis: modern models handle gestalt well but precise column-exact lookups remain shaky.

---
▸ 2026-08-22T13:41:45Z
Probe results (model: Opus 4.8 via spawn_agent). Simple 2-box fixture: parallel + runlist both 7/7. Discriminating 9x60 list (highlighted row, blocked marker, 2 label chips): parallel/interleaved/runlist/spans ALL 7/7. Accuracy SATURATES on the strong model even on deliberately hard cross-reference questions. Conclusion: accuracy does not discriminate encodings at this model tier; finding the accuracy cliff needs harder fixtures (exact-lookup, larger/denser grids) and/or weaker models. Token cost becomes the deciding axis.

---
▸ 2026-08-22T13:55:33Z
Cliff-hunt round (Opus 4.8): 3 real-world scenarios (align / contain / confound) x 3 representations (plain / ruler / spans) = 9/9 correct. Adversarial cases did NOT break it: subtle 1-col misalignment caught on plain grid; nested-vs-disjoint boxes correct; list-in-a-box vs same-format decoy list correct. Notably spans got containment + decoy discrimination right DESPITE mangling 2D layout. Conclusion: accuracy cliff is not reachable at this model tier with small hand-authored fixtures. To find it we need weaker models (Haiku etc.) or full-screen/dense real TUI frames. For strong models, encoding choice = token cost + robustness margin -> spans (cheapest) / interleaved (aligned-grid option).

---
▸ 2026-08-22T14:04:32Z
Vertical-alignment round (Opus 4.8): 4 fixtures (divider aligned/broken, box clean/jagged) x plain/ruler/spans = 12/12, both directions (no yes/no bias). spans caught a 1-col divider shift and a jagged box right-border. Mechanism: spans preserves inter-run whitespace LITERALLY, so the model recovers column offsets by counting spaces (arithmetic), not by seeing columns. Human-legibility != model-legibility. Still untested: deep target column after many variable-width runs across distant rows (cumulative arithmetic) - the likely spans breaking point, and where weaker models should crack first.

---
▸ 2026-08-22T14:14:38Z
Cumulative-offset sweep (Opus 4.8): wide tables 4/6/9/12 cols (to 100 wide), a 1-char-narrower pad in one row shifts the deep last column 'END' by one. spans 4/4 broken correct + aligned control correct (no false positive); plain & ruler correct at max width. HONEST CAVEAT: in fixed-width tables the shift shows in spans as a LOCAL single-space gap anomaly (1 space vs 2), catchable without full summation. So this proves spans robust for REALISTIC TUI misalignment (fixed columns always leave such a local gap), NOT that the model can sum arbitrary deep offsets with no local cue. Definitive pure-arithmetic test would use variable (1-3 space) separators so only the running total reveals the shift - more synthetic than real TUIs.

---
▸ 2026-08-22T14:23:09Z
CONCLUSION: could not break spans on a frontier model. Cue-free variable-gap tables (mixed 1-3 space gaps, no local anomaly; only the running total reveals a 1-col drift of the deep 'END' column) at 6/9/12/16 cols (~100 wide): spans 4/4 broken correct + aligned control correct (no bias); plain+ruler correct at 16. Caveat: broken row's total line length is also 1 shorter, but that's equivalent-difficulty to summing (not a local shortcut). Did not test beyond 16 cols. Practical verdict: on a frontier model spans is robust for vertical-alignment / cumulative-offset across realistic TUI widths. Recommend banking spans as default + pivot to wiring it into the Rust harness (a39e) + frame-diff (9f43).
