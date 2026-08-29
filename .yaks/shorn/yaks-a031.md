---
id: yaks-a031
title: Rapid double-esc == Ctrl-C
type: feature
priority: 3
created: '2026-08-26T11:58:43Z'
updated: '2026-08-26T13:42:21Z'
parent: yaks-fc85
labels:
- ui
---

---
▸ 2026-08-26T13:02:04Z
Debounce hint: treat two Escs within roughly 300ms as a Ctrl-C-equivalent cancel. A lone Esc keeps its normal meaning (to Normal mode / peel back).

---
▸ 2026-08-26T13:42:21Z
Implemented. App.register_double_esc() records each editor-overlay Esc and reports a rapid second Esc (<=300ms, DOUBLE_ESC_MS) as a Ctrl-C-equivalent cancel; computed once in handle_overlay_key and threaded into the Edit branch + handle_create_key. Purely additive: single Esc keeps its meaning (drop to Normal in editors), a fast double cancels. Immediately useful in the multiline comment editor (M) and the edit-form content rows, where a lone Esc only ever drops to Normal and Ctrl-C was previously the only way out. Tests: lone-esc-stays, double-esc-cancels (comment editor + form content row). 148 bin tests green, 0 warnings.
