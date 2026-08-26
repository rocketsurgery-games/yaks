---
id: yaksrs-2e80
title: In vi mode, make single-line fields modal
type: task
priority: 3
created: '2026-08-26T12:51:14Z'
updated: '2026-08-26T13:02:04Z'
parent: yaksrs-fc85
depends_on:
- yaksrs-a031
labels:
- ui
---

Esc is canceling the entire yak edit by default, so if we *do* already support modal editing in single-line fields, it's getting lost in the process.
It would be nice if this were true by default, across the UI. This would require Ctrl-C / Double-Esc to cancel (vi only, not emacs), but I think that's ok.

---
▸ 2026-08-26T13:02:04Z
Fix site: in the edit form, handle_create_key treats Esc on any non-content row as 'cancel the whole form' (the Esc && !is_content branch). That overrides the first-Esc->Normal behavior single-line fields otherwise have. Needs the non-Esc cancel path (yaksrs-a031) first.
