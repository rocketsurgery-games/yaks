---
id: yaks-012b
title: Docs + skill-name normalization + TUI doc screenshots
type: task
priority: 2
created: '2026-08-30T19:34:28Z'
updated: '2026-09-06T23:47:17Z'
parent: yaks-8f81
labels:
- skills
---

First in-repo pstack-lite artifact. Encodes check-before-start, evidence-before-shorn, one-writer-per-yak, and optional attribution. Deliberately minimal.

---
▸ 2026-08-30T19:35:46Z
Seeded skills/dev/README.md and skills/dev/working-a-yak/SKILL.md. Encodes check-before-start (re-read + freshest note first), evidence-before-shorn, one-writer-per-yak, and optional [actor] attribution. Not shipped: BUNDLED in src/skills.rs unchanged. Unit complete pending human review + commit; left shaving until committed alongside, per team-mode rule.

---
▸ 2026-09-06T23:02:22Z [coordinator]
[coordinator] REPURPOSING (original skills-seed work is long done). Now the umbrella for: (1) normalize skill names to yaks[-*]; (2) write docs/; (3) TUI doc screenshots; (4) consistency/error pass over all skills; (5) final docs review. Sequence per Joel: docs first, then skill pass, then docs review. Skill rename decision: yak->yaks, yak-tracker->yaks-tracker, working-a-yak->yaks-working, coordinating-yaks->yaks-coordinating (all yaks[-*], consistent with project name + each other).

---
▸ 2026-09-06T23:11:52Z [coordinator]
PROGRESS (this session): DONE — 865d (skill rename to yaks[-*], symlinks repointed, 233+25 green; recovered a broken intermediate commit where a bad git-add pathspec dropped the sed content edits) and d6d3 (docs/: README/cli/tui/skills, accurate to --help + help_content). REMAINING children: 3677 (TUI SVG screenshots — feasible, plan recorded), 8c46 (skill consistency/error pass), f85f (final docs review). Sequence continues: skill pass, screenshots, docs review.

---
▸ 2026-09-06T23:42:07Z [coordinator]
3677 (color TUI screenshots) shorn + committed (fb92025): 4 SVGs embedded in docs/tui.md. Remaining children: 8c46 (skill consistency pass, now shaving), then f85f (docs review).

---
▸ 2026-09-06T23:47:17Z [coordinator]
Umbrella complete — all 5 children shorn. Delivered: (865d) skill rename to yaks[-*]; (d6d3) docs/ suite — README/cli/tui/skills; (3677) color TUI screenshots via headless buffer->SVG, embedded in docs/tui.md; (8c46) skill consistency pass — 4 fixes; (f85f) docs review — 3 fixes incl. a wrong 'doctor --strict' CLI help string. Docs public-facing + memory-refreshing; skills+docs+CLI now mutually consistent, all claims verified against --help. cargo test --workspace green (233+25+13+1); yaks doctor clean.
