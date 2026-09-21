---
id: yaks-209a
title: Add herd-filtering UI affordance
type: task
priority: 3
created: '2026-09-21T19:58:48Z'
updated: '2026-09-21T20:44:11Z'
labels:
- herds
- tui
---

This should be just another line in the filtering options (as we did with the creation UI).

---
▸ 2026-09-21T20:44:11Z [agent]
SHORN. Herd-filtering affordance in the TUI filter drawer (fanned out to a subagent, then integrated + reviewed). Drawer gains a multi-select 'herd' chip row modelled on the status row, populated from App::herd_choices(), gated to multi-herd farms (no row/layout change when single-herd). from_filter takes the herd choices; chip_count/toggle_chip/row_count/render_drawer handle the gated row; build_spec emits FilterSpec.herds; open_drawer threads herd_choices and nav wraps on row_count. Test drawer_herd_chip_scopes_the_filter_in_a_multi_herd_farm. Integrator fix: herd_choices counts only real dashed id prefixes. docs/tui.md updated. Verified headless: drawer shows 'herd  test yaks' on this repo. Suite green (258 lib + 25 cli), warning-free.
