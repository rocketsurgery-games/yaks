---
id: yaksrs-9a8f
title: 'edtui: word-aware (soft) line wrapping in the editor'
type: feature
priority: 3
created: '2026-08-25T02:25:19Z'
updated: '2026-08-25T02:25:19Z'
parent: yaksrs-6099
labels:
- ui
- edtui
- upstream
---

In edit mode edtui's LineWrapper hard-wraps at character boundaries (splits words mid-word). The read-only detail pane now word-wraps (yaks detail::wrap), but the editor still char-wraps. Options: PR edtui to add word-aware wrapping (or a wrap-mode option) to LineWrapper (it's pub(crate)); reference impl = yaks detail::wrap_ranges. Lower priority than the other edtui PRs; nice-to-have for editing long prose in narrow viewports.
