---
id: yaksrs-b0b4
title: Edit source frontmatter in the create/edit form
type: bug
priority: 2
created: '2026-08-23T02:49:48Z'
updated: '2026-08-23T02:49:48Z'
labels:
- ui
---

Carried over from Python-repo yak-3f26. Confirmed gap in yaks-rs: CreateForm has only title/labels/description/type/priority, no source field. Only the CLI update --source can set it; the TUI form cannot. Add a source field to the form.
