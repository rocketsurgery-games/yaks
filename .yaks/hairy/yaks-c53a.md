---
id: yaks-c53a
title: 'Body content input beyond inline args: --description-file, stdin, and yaks edit'
type: feature
priority: 2
created: '2026-08-23T03:43:29Z'
updated: '2026-08-23T03:43:29Z'
parent: yaks-8d53
labels:
- cli
---

create and update accept --description-file PATH and --description - (read body from stdin), plus a yaks edit ID command that opens the yak (frontmatter + body) in EDITOR and validates on save. Motivation: rich markdown bodies with backticks, quotes and newlines are painful and unsafe as inline --description args, especially from an agent shell; this drove hand-edits of .yaks/*.md during the skills port and the description-restore pass.
