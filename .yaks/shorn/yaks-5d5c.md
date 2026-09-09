---
id: yaks-5d5c
title: 'Human-driven interactive lane: kickoff/handoff guidance'
type: task
priority: 2
created: '2026-09-09T00:11:52Z'
updated: '2026-09-09T00:24:40Z'
parent: yaks-10fb
labels:
- skills
---

A human-driven interactive lane running in parallel with other work. FRAMING (approved): it is a PEER lane, not a new role — same git/yak mechanics as a worker lane; it lands through the coordinator, or the human IS the coordinator if none is running. Novelty is only: human drives it, it's long-lived/interactive, and it often starts yak-less (pure design talk) and emits code or a fresh herd. Key inversion: because the human opens wt/<name> as the harness root, the agent's file tools root at the WORKTREE correctly — this lane dodges our worst spawned-sub-agent pitfall by construction. Mode behavior: private = one shared live herd (emitted yaks visible to coordinator instantly; only code lands; claim by shaving/tagging so 'yaks next' skips held work); team = new yaks live on the branch until merge, so land the herd early via a .yaks/-only commit, land code when ready. Invariant intact: talking needs no yak; open a thin 'design lane' shaving yak the moment there's an artifact. Handoff 'we're done, merge up': no coordinator -> human merges/stamps/shears/removes worktree; coordinator active -> produce a committed branch + signal readiness (message cheapest; note/label in private; ask if it carries a decision), coordinator owns the land. Deliberately NO worktree-awareness in the CLI and NO live cross-worktree HITL (the human is physically at this worktree). Deliverable: a section in yaks-coordinating + a docs/skills.md touch. No tool change yet — a 'ready-to-land' marker (needs: land / label) must earn its place first. Judge: human read-over (no script lever).

---
▸ 2026-09-09T00:12:36Z [coordinator]
Drafted for review (judge = human read-over; no script lever). Added section 'Human-driven interactive lane (parallel, without a fan-out)' to skills/dev/yaks-coordinating/SKILL.md (between Merge/integration and PR-driven integration); touched docs/skills.md (coordinating bullet + workflows-at-a-glance line). Left shaving pending human judgment; uncommitted.

---
▸ 2026-09-09T00:24:40Z [coordinator]
Shorn: added 'Human-driven interactive lane' section to yaks-coordinating + two docs/skills.md touches; Kickoff sharpened so re-rooting reads as the human's non-delegable step. Evidence: prose reviewed and approved by human (no script lever — skills/dev/ isn't embedded, so 'cargo test skills' doesn't guard it). Landing alongside this commit.
