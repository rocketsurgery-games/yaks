---
id: yaksrs-d168
title: 'edtui PR: count prefixes (3j, 2dd, 5x)'
type: feature
priority: 2
created: '2026-08-24T22:12:10Z'
updated: '2026-08-24T22:12:10Z'
parent: yaksrs-6099
labels:
- ui
- edtui
- upstream
---

Marquee contribution. KeyEventHandler has no numeric-count concept, but nearly every Action already carries an unused usize count (MoveDown/DeleteLine/RemoveChar/DeleteWordForward...). Route a numeric register accumulated in the key handler into those counts. High value; codebase is pre-shaped for it. Requires GitHub fork.
