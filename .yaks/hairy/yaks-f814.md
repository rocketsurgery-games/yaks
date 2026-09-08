---
id: yaks-f814
title: 'Evidence contract per yak: acceptance criteria set at claim'
type: feature
priority: 2
created: '2026-09-08T01:44:55Z'
updated: '2026-09-08T01:44:55Z'
parent: yaks-6624
labels:
- eval,skills
---

The unifying mechanism for yaks-6624. A yak carries an evidence CONTRACT (definition-of-done) authored BEFORE work starts — by the coordinator at claim time (the licks-the-cookie prep commit), or by the working agent itself in solo mode. The worker satisfies it with evidence (note carrying command+output/artifact-path, or a literal attachment) before shear; doctor --strict (yaks-79a9) checks evidence exists AND post-dates the contract. Forcing function: inability to state acceptable evidence == the lever is missing == the yaks-ask walkabout trigger (ties 139b). Layering: per-repo config (c3b1) gives the baseline contract per change-type; the coordinator specializes per-yak (same defaults+override shape as views/config). Generalize 'correctness' to 'outcome achieved+seen, appropriate to the yak kind' (dead exempt). Shape: start as an 'accept:' note-convention, graduate to a frontmatter field once earned. Sub-gap: add a 'yaks attach' CLI so CLI-first agents can attach a seen artifact as first-class evidence (today only herd.attach + TUI A/O exist).
