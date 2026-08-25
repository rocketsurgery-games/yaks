---
id: yaksrs-114a
title: 'edtui PR: native gj/gk/g0/g$ + expose wrap'
type: feature
priority: 3
created: '2026-08-24T22:12:10Z'
updated: '2026-08-24T22:12:10Z'
parent: yaksrs-6099
labels:
- ui
- edtui
- upstream
---

Add display-line motions natively and/or expose the wrap segmentation (LineWrapper/ViewState screen area are pub(crate)). Landing this lets yaks DELETE its local wrap reimplementation (wrap_segments/display_line_nav), removing the drift risk. Reference impl: yaks display_line_nav(). Requires GitHub fork.
