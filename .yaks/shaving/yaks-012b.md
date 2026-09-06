---
id: yaks-012b
title: Docs + skill-name normalization + TUI doc screenshots
type: task
priority: 2
created: '2026-08-30T19:34:28Z'
updated: '2026-09-06T23:02:22Z'
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
