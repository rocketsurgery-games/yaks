---
id: yaks-865d
title: Normalize skill names to yaks[-*]
type: task
priority: 2
created: '2026-09-06T23:02:22Z'
updated: '2026-09-06T23:05:16Z'
parent: yaks-012b
labels:
- skills
---

yak->yaks, yak-tracker->yaks-tracker, working-a-yak->yaks-working, coordinating-yaks->yaks-coordinating. Touch: dirs, SKILL.md name: frontmatter, BUNDLED + test in src/skills.rs, the 'skills' command help text, ~/.agents/skills symlinks, cross-references between skills, AGENTS.md, and any docs. Rebuild + cargo test after.

---
▸ 2026-09-06T23:05:16Z [coordinator]
DONE. Renamed all skills to yaks[-*]: yak->yaks, yak-tracker->yaks-tracker, working-a-yak->yaks-working, coordinating-yaks->yaks-coordinating. Updated: dirs (git mv), SKILL.md name: frontmatter + cross-refs, BUNDLED + test in src/skills.rs, the skills command help + doctor doc-comments (main.rs/herd.rs), README, dev/README. ~/.agents/skills symlinks repointed (added yaks-coordinating for consistency). id-PREFIX 'yak' left as-is (separate concept). 233+25 tests green.
