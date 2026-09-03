---
id: yaks-eb66
title: Optional attribution on updates (--as actor), ownership-free
type: feature
priority: 3
created: '2026-08-30T22:52:32Z'
updated: '2026-09-03T16:23:01Z'
parent: yaks-3901
labels:
- agent
---

Attribution (who wrote a note / made a transition), NOT ownership. Committed transitions already carry attribution via the git commit author, so this is mainly for in-file visibility and local-only mode. Add optional --as <actor> on update/shave/shorn, stamped into the note; default from an env (YAKS_ACTOR) or git user. Open design question: naming for humans vs agents, and agents acting on behalf of a human (agent@human?). Do NOT introduce yak ownership. From the experiment where [wtA]/[wtB] tags were used by hand.

---
▸ 2026-09-03T16:22:56Z
[coordinator] DECISION (design settled, ready to build):
- Store STABLE ACTOR IDENTITY, not ephemeral lane role. [wtA]/[coordinator] conflated the two; formalize identity only, do NOT globally-uniquify lane tags (transient by nature). Yak says which-work, actor says who; lane is usually redundant.
- Resolution order for the actor: --as <actor> flag  >  $YAKS_ACTOR env (harness/coordinator pins it once per worker)  >  default from git config user.name/email.
- Humans get attribution FREE via git identity (one identity space with commit authorship); agents self-declare via --as/env. Still NO ownership.
- Storage: stamp onto the note metadata line reusing the bracket syntax people already type: '▸ <ts> [actor]'. Additive + backward-compatible (old notes -> actor None; parse_notes takes leading token as ts, optional trailing [..] as actor). It IS an on-disk note-format touch -> keep the 'additive, no migration' guarantee (AGENTS.md).
- agent-on-behalf-of-human: an 'agent@human' handle (e.g. opus@joel) carries both, no second field.
- Query surface: 'yaks log --by <actor>'; show actor in log/show output.
