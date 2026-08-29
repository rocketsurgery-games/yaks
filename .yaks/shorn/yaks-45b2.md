---
id: yaks-45b2
title: Skill installation path(s)
type: task
priority: 3
created: '2026-08-29T22:27:42Z'
updated: '2026-08-29T22:32:50Z'
---

The README just tells you to clone the yaks repo and manually copy the skills. Two paths:
- We could at least tell users to use `openskills` or whatever's most popular with the cool kids these days.
- It would also be useful to have a `yaks skills install` or similar affordance.

Feel free to research the most up-to-date norms before committing to particular patterns and tools.

---
▸ 2026-08-29T22:32:50Z
SHORN. Two install paths, per the yak:
1) Built-in `yaks skills install [--dir <PATH>] [--force]` — the yak+yak-tracker SKILL.md are embedded in the binary (include_str!, src/skills.rs) and written to ~/.agents/skills by default, or --dir for another agent (e.g. ~/.claude/skills); skips existing unless --force. Handled BEFORE Herd::open so it runs with no herd (installing the skill usually precedes any .yaks). Fits the self-contained-binary model: no clone needed.
2) README: replaced the git clone + cp -r block with `yaks skills install`, and added openskills as the cross-agent option (npx openskills install rocketsurgery-games/yaks) — researched: openskills is the popular universal Anthropic-format SKILL.md loader (~13k weekly dl).
1 unit test (install writes both / skips / forces). 192 lib tests, warning-free.
