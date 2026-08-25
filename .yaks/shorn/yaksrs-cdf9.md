---
id: yaksrs-cdf9
title: 'edtui PR: x deletes into the clipboard'
type: feature
priority: 3
created: '2026-08-24T22:12:10Z'
updated: '2026-08-25T22:47:29Z'
parent: yaksrs-6099
labels:
- ui
- edtui
- upstream
---

RemoveChar (x) currently doesn't populate state.clip, so x then p doesn't work like vim. Make RemoveChar yank the removed char(s) via delete_range/clip (dd/dw already do). Small, matches vim. Requires GitHub fork.

---
▸ 2026-08-25T04:02:32Z
Shipped as a yaks-side shim (not upstream): x and X now yank the deleted char to the system clipboard before edtui deletes it, so x then p works (consistent with dd/dw which already yank). In route_multiline_key normal mode. Test x_and_shift_x_delete_chars covers the delete; the clipboard yank is best-effort. The upstream edtui version (make RemoveChar populate clip) is deferred to issues-first.

---
▸ 2026-08-25T22:47:29Z
Now native too: added x-yank + X (RemoveCharBefore) to edtui (branch yaks/x-yanks), merged to yaks-integration. yaks-side x/X shims DELETED; edtui's x/X yank natively.
