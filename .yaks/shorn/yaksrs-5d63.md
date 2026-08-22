---
id: yaksrs-5d63
title: The i/o (in/out) keys don't do anything from within the detail view
type: bug
priority: 3
created: '2026-08-22T19:06:44Z'
updated: '2026-08-22T19:19:13Z'
parent: yaksrs-0a93
---

---
▸ 2026-08-22T19:19:13Z
Implemented o/i as browser-style task navigation history (o=back, i=forward), matching Python's _nav_back/_nav_forward. Added App.nav_back/nav_fwd stacks; following a detail link pushes the current task and clears forward (standard browser semantics). Extracted open_task_in_detail (shared by follow_link + nav). Bound o/i in the detail key handler (help hint already advertised i/o:fwd/back). Empty stacks notify 'no earlier/later yak'. Tests: nav_history_back_and_forward, nav_back_on_empty_history_is_noop. 104 unit + 19 CLI green.
