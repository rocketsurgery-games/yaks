---
id: yaks-1d54
title: Reconstruct skills for yaks-rs (yak + yak-tracker)
type: task
priority: 2
created: '2026-08-23T02:04:36Z'
updated: '2026-08-23T02:09:26Z'
labels:
- skills
- docs
---

Move skills from the Python repo into yaks-rs, reconstructing (keep/toss/change) rather than copying. Adapt to the Rust/npm reality (no uvx; npx yakherder / installed yaks binary). Fold in guidance from the old repo's open [agent] yaks: terminology clarity (yak-0433), stronger anti-leak guidance (yak-5836/5d00), label-use guidance (yak-8660, yak-efb9 tracker-derived), human+agent collaboration / herd-drift-is-normal (yak-86ad). Surface missing tools as new yaks.

---
▸ 2026-08-23T02:09:26Z
Both skills reconstructed + shorn (a894 yak, 79c3 yak-tracker). Missing tools surfaced as yaks: 7511 (init command — real gap), 7cd1 (per-repo agent context, from 4930+011e). Folded old [agent] guidance yaks into the skills (0433 terminology, 5836/5d00 anti-leak, 8660 labels, efb9 tracker labels, 86ad human+agent collab) — those are effectively addressed for the Rust skill. Broader carry-over triage handed to the user for confirmation.
