---
id: yaks-114a
title: 'edtui PR: native gj/gk/g0/g$ + expose wrap'
type: feature
priority: 3
created: '2026-08-24T22:12:10Z'
updated: '2026-08-25T22:47:17Z'
parent: yaks-6099
labels:
- ui
- edtui
- upstream
---

Add display-line motions natively and/or expose the wrap segmentation (LineWrapper/ViewState screen area are pub(crate)). Landing this lets yaks DELETE its local wrap reimplementation (wrap_segments/display_line_nav), removing the drift risk. Reference impl: yaks display_line_nav(). Requires GitHub fork.

---
▸ 2026-08-25T04:00:02Z
Note: gj/gk/g0/g$ already ship via yaks-side shims (display_line_nav) and work today. This yak's real remaining value is removing the shim's reimplementation of edtui's PRIVATE hard-wrap algorithm (wrap_segments) - a drift risk if edtui changes wrapping. That still needs upstream (expose wrap, or native motions), deferred to issues-first.

---
▸ 2026-08-25T22:47:17Z
Done natively: added display-line motions gj/gk/g0/g$ to edtui (branch yaks/display-line-motions) - they read the view's render width + reproduce the wrap, count-aware. Merged to yaks-integration. yaks now consumes them and DELETED its entire wrap reimplementation (display_line_nav/wrap_segments/GMotion/row_chars/max_col_for) - the drift-risk shim is gone.
