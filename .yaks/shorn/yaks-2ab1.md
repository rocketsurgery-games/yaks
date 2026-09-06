---
id: yaks-2ab1
title: Encode the coordinator-claim run shape in coordinating-yaks
type: task
priority: 2
created: '2026-09-06T01:54:12Z'
updated: '2026-09-06T01:54:49Z'
parent: yaks-a412
labels:
- skills
---

New run shape (agreed with Joel): (1) ONE coordinator commit on main creates/shaves the batch -> shaving + per-yak assignment notes ('lick the cookie'); (2) workers branch from it and SKIP the shave step — just do the work + move shaving->shorn (ideally one commit); (3) squash-merge each lane back (id in the message, per 213b); (4) coordinator reconciles any yak left shaving (regrow or re-run). Wins: fewer commits, explicit + OBSERVABLE claim (main's 'shaving' now reflects in-flight work), a durable home for assignment context, squash-friendliness. Cost: shaving-on-main must be reconciled on abort (doctor-flaggable). Updates the Merge/integration + worktree sections.

---
▸ 2026-09-06T01:54:49Z [coordinator]
DONE. Added a 'Parallel run shape (claim -> fan out -> merge -> reconcile)' section to coordinating-yaks and updated Merge/integration: squash-merge is now the DEFAULT for single-commit lanes (the claim commit documents the batch, so topology needn't live in the graph; provenance survives via id-in-message + --follow). --no-ff reserved for multi-commit lanes. About to trial this shape on a live run.
