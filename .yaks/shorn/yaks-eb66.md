---
id: yaks-eb66
title: Optional attribution on updates (--as actor), ownership-free
type: feature
priority: 3
created: '2026-08-30T22:52:32Z'
updated: '2026-09-03T16:28:56Z'
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

---
▸ 2026-09-03T16:28:52Z [coordinator]
BUILT + verified. Actor attribution on notes.
- store::append_note_as(body, ts, actor, note) stamps '▸ <ts> [actor]'; append_note is now a None wrapper (no churn at ~10 existing call sites). NoteEntry gains actor: Option<String>; parse_notes splits an optional trailing [actor] (bare notes -> None, byte-compatible, no migration).
- TaskEdit.actor threads through herd.update. CLI: 'yaks update --as <actor>' + resolve_actor: --as > $YAKS_ACTOR > git user.name (best-effort, never fails). 'yaks log --by <actor>' filters; log + json show the actor.
- Verified all three resolution paths on a throwaway: --as opus@joel -> [opus@joel]; YAKS_ACTOR=worker-b -> [worker-b]; default -> [Joel Webber] (git user.name). log --by filters correctly. 204 yaks + 20 CLI + 13 toque tests green. THIS NOTE is itself attributed via --as coordinator (dogfooded).
FOLLOW-UP (out of scope here): TUI comment path (tui.rs EditAction::Comment) still writes actor=None -- wire it to resolve_actor/git identity so human TUI comments are attributed too. Transition attribution stays deferred (git author covers committed moves).
