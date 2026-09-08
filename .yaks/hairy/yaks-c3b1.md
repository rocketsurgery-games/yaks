---
id: yaks-c3b1
title: Per-repo context names the project's verification lever(s)
type: feature
priority: 2
created: '2026-09-08T01:32:42Z'
updated: '2026-09-08T01:32:42Z'
parent: yaks-6624
labels:
- agent,config,eval
---

Ties yaks-7cd1 (per-repo agent context). The repo declares what 'verify' concretely MEANS for it — its levers and how to invoke them (e.g. 'TUI: drive toque and look + cargo test docshots'; 'web: sightmap/CDP'; 'stateful: this seed script'). Gives the agent the real-artifact definition so 'it was too hard' is not an excuse, and gives doctor/ask something concrete to point at. This is yaks' lightweight analog of pstack's Feature Map / verification-skill, but yaks supplies the escalation+tracking substrate rather than shipping a create-verification-skill.
