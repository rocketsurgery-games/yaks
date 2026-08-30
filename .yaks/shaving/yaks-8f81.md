---
id: yaks-8f81
title: 'Dogfooding harness: in-repo pstack-lite skills and worktree isolation'
type: task
priority: 2
created: '2026-08-30T19:34:21Z'
updated: '2026-08-30T19:35:46Z'
parent: yaks-3901
labels:
- skills
---

Put simplified, repo-internal workflow skills under skills/dev/ (not shipped; BUNDLED stays yak + yak-tracker). Seed a minimal working-a-yak skill. Continue on the Zed harness; to test a repo-local skill live, symlink it into ~/.agents/skills for the session. Adopt git worktrees for isolation. Open design fork: worktrees check out their own committed .yaks/, so per-branch herds diverge and reconcile at merge; a live shared blackboard wants one out-of-tree herd symlinked into each worktree instead. Sandbox note: an agent only reaches paths under its project root, so a coordinator cannot see sibling worktrees; parallel agents open each worktree as its own project and coordinate through yaks.

---
▸ 2026-08-30T19:35:46Z
Worktree probe: git worktree add --detach target/wt-probe HEAD, inspected, removed cleanly. Finding: each worktree checks out its own committed .yaks/, so herds are per-branch and commit-synchronized, not a live shared blackboard. Evidence: the new hairy yaks created in the main tree were absent from the worktree (uncommitted), shorn counts differed (worktree 111 vs main 112). Also: agent file tools only reach paths under the project root, so a coordinator cannot read sibling worktrees. Design fork: (a) per-branch herds reconciled at merge (simple, matches team mode) vs (b) one shared out-of-tree .yaks/ symlinked into each worktree for live cross-agent coordination. Deferred.
