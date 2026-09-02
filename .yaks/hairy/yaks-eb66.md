---
id: yaks-eb66
title: Optional attribution on updates (--as actor), ownership-free
type: feature
priority: 3
created: '2026-08-30T22:52:32Z'
updated: '2026-08-30T22:52:32Z'
parent: yaks-3901
labels:
- agent
---

Attribution (who wrote a note / made a transition), NOT ownership. Committed transitions already carry attribution via the git commit author, so this is mainly for in-file visibility and local-only mode. Add optional --as <actor> on update/shave/shorn, stamped into the note; default from an env (YAKS_ACTOR) or git user. Open design question: naming for humans vs agents, and agents acting on behalf of a human (agent@human?). Do NOT introduce yak ownership. From the experiment where [wtA]/[wtB] tags were used by hand.
