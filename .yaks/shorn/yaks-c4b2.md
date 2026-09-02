---
id: yaks-c4b2
title: Public/private herd patterns + skill/docs/CLI support
type: task
priority: 2
created: '2026-09-02T22:57:10Z'
updated: '2026-09-02T23:28:50Z'
parent: yaks-3901
labels:
- docs
---

Consensus on how yaks supports PUBLIC (team) vs PRIVATE (local-only) herds, and the updates to make the patterns first-class.

Modes:
- Public/team: .yaks/ committed with the code. Visible on the platform (GitHub, PRs, git log). Yak surgery merges with code; provenance via file+commit (yaks commits / git log --follow). Yak ids OK in commit messages, NEVER in PR titles/descriptions or external trackers.
- Private/local-only: .yaks/ not committed to the code repo. Hiding options: root .gitignore; .yaks/.gitignore='*' (self-contained, non-nested ONLY); .git/info/exclude (per-repo, untracked, no root edit); global core.excludesFile (across many repos).
- Private + multi-machine: nested repo. Give .yaks/ its own .git on a private remote; hide from the outer repo via .git/info/exclude (NOT the '*' trick). Discovery just works, no yaks change. cd .yaks for herd git ops; pull-before / push-after.

Verified findings:
- The '*'-inside trick hides a local-only herd but BREAKS with a nested .git: the outer repo shows .yaks/ as an embedded repo, and '*' also blinds the herd's own repo (cannot track its files). Use .git/info/exclude for the nested case.
- Nested private herd needs zero yaks changes.
- Footgun: git clean -fdx in the OUTER repo deletes the gitignored/excluded .yaks/ and its nested repo. Push often.
- Provenance-to-code is cross-repo in private mode (already moot: nothing commits with code).
- A gitignored/excluded .yaks/ is NOT auto-shared across outer git worktrees (absent in fresh checkouts).
- Herd-location override (YAKS_DIR/--herd, ex yaks-d4e7, slaughtered) is NOT needed for private-multi-machine; keep only as a deferred idea for out-of-tree or non-.yaks-named herds.

Updates needed:
- Skills: extend the yak skill's local-vs-team section to cover the private-multi-machine nested-repo pattern, the correct hiding method per case ('*' vs .git/info/exclude vs global), and the clean -x footgun. Relates to yaks-7cd1 (per-repo agent context).
- Docs: document public/private patterns + multi-machine sync in README/docs.
- CLI: optional yaks sync wrapper (pull/commit/push the herd repo); tie into the init command (yaks-7511) to offer local/team/private setup; yaks path (yaks-aa49) to locate the herd. Override stays deferred.

---
▸ 2026-09-02T23:28:43Z
Docs: extended skills/yak/SKILL.md local-vs-team section to add the three private-hiding methods (root .gitignore; .yaks/.gitignore='*' non-nested only; .git/info/exclude; global core.excludesFile), the local-only-across-machines nested-repo pattern (own .git on a private remote, hidden via .git/info/exclude, pull-before/push-after), and the 'git clean -fdx' footgun. README.md: added a 'Public and private herds' section covering public/team, private/local-only, and multi-machine sync, plus the clean -x heads-up.

---
▸ 2026-09-02T23:28:43Z
Follow-up (CLI, not done here — src/ owned by a parallel worker): add an optional 'yaks sync' wrapper (pull/commit/push the herd repo), tie into 'yaks init' (yaks-7511) to offer local/team/private setup, and 'yaks path' (yaks-aa49) to locate the herd. Herd-location override (YAKS_DIR/--herd, ex yaks-d4e7) stays deferred — not needed for private-multi-machine.

---
▸ 2026-09-02T23:28:50Z
Shorn summary: docs portion of the consensus is rendered. Files changed: skills/yak/SKILL.md (modes/hiding/footgun) and README.md (new 'Public and private herds' section). All three modes and the per-case hiding method are documented; the clean -x footgun and the 'not shared across worktrees/clones' caveat are called out in both. No new design decisions — prose translation of the yak body only. CLI work left as the follow-up note above. src/ untouched (parallel worker owns it).
