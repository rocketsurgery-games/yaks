---
id: yaks-7cb3
title: Parse label edits with/without comments
type: bug
priority: 3
created: '2026-08-26T13:23:02Z'
updated: '2026-08-26T13:23:02Z'
parent: yaks-f04d
labels:
- ui
- labels
---

At present, it's possible to create labels with spaces in them. We should disallow this.
Then we can enforce label structure at parse time (we should probably normalize them in the CLI tools as well).
If the user types:
- `foo, bar` -> `[foo bar]`
- `foo bar`  -> `[foo bar]`
- `foo,bar`  -> `[foo bar]`

And so forth. Ie, we should disallow spaces and commas in labels.
