---
id: yaks-865d
title: Normalize skill names to yaks[-*]
type: task
priority: 2
created: '2026-09-06T23:02:22Z'
updated: '2026-09-06T23:02:22Z'
parent: yaks-012b
labels:
- skills
---

yak->yaks, yak-tracker->yaks-tracker, working-a-yak->yaks-working, coordinating-yaks->yaks-coordinating. Touch: dirs, SKILL.md name: frontmatter, BUNDLED + test in src/skills.rs, the 'skills' command help text, ~/.agents/skills symlinks, cross-references between skills, AGENTS.md, and any docs. Rebuild + cargo test after.
