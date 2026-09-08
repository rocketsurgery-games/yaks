---
id: yaks-fe5a
title: yaks as substrate for blinded skill/prompt-change eval
type: idea
priority: 3
created: '2026-09-08T00:50:52Z'
updated: '2026-09-08T00:50:52Z'
parent: yaks-a2ca
labels:
- eval
---

From pstack's 'eval' playbook: 'test how a skill or prompt change affects agent behavior, blinded.' Directly answers Joel's original question — how do we know the skill system does what's intended? Insight: yaks already has the MEASUREMENT half — log --since/--by (each actor's activity trail), commits (what landed), --as attribution (separate actors), diff <refA> <refB> (yaks-70e5) (herd-state delta between two runs). So 70e5 is EVAL INSTRUMENTATION, not a diff toy. Key property: this instrumentation is OBJECTIVE behavioral evidence, independent of the agent's self-report (= pstack 'prove-it-works: not a proxy or self-report', applied to evaluating the agent itself). Missing piece is a LIGHT methodology skill (fix a task set; run variant A/B, ideally BLINDED so the agent can't game it; measure the delta via the primitives above; record the verdict on an 'eval'-labeled yak), not much new tooling. Candidate light affordances only if pain shows: an eval label/type convention; a run-scoped log/diff window.
