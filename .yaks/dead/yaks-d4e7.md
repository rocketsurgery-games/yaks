---
id: yaks-d4e7
title: Herd-location override (YAKS_DIR / --herd) to decouple herd from cwd/repo
type: feature
priority: 2
created: '2026-09-01T23:31:59Z'
updated: '2026-09-02T22:57:15Z'
parent: yaks-3901
labels:
- cli
---

A YAKS_DIR env and/or --herd <path> flag so the CLI/TUI can point at a herd anywhere, instead of only discovering .yaks/ upward from cwd. Enabling primitive for: (a) out-of-tree / shared herds (the git-worktree coordination models), and (b) a PRIVATE herd kept in a separately-synced repo OUTSIDE the code repo, for working across machines without committing yaks to the shared remote. Independently useful for multi-herd workflows.

---
▸ 2026-09-02T01:16:53Z
Correction: the private + two-machines case needs NO code change and NOT this override. A nested repo works today: keep .yaks/ in place, gitignore it in the outer repo, and give .yaks/ its own .git pointed at a private remote. yaks' normal upward .yaks discovery finds it; the outer repo ignores it cleanly (verified: binary lists the nested herd, outer status clean, two independent .git dirs). This override (YAKS_DIR/--herd) remains useful for OTHER cases: a herd not named .yaks, or one shared out-of-tree across worktrees. Key caveat of the nested repo: git clean -fdx in the OUTER repo would delete the gitignored .yaks and its nested repo, so push often / avoid clean -x.

---
▸ 2026-09-02T22:52:11Z
Hiding a NESTED-repo herd: the .yaks/.gitignore='*' self-hide trick does NOT carry over (verified). Two reasons: (T1) once .yaks/ has its own .git, the outer repo treats it as an embedded repo and shows '?? .yaks/' regardless of the inner .gitignore; (T2) '*' also blinds the herd's OWN repo, so it can't track its yak files (same file, opposite needs). Closest equivalent that still avoids editing the committed root .gitignore: add '/.yaks/' to the outer repo's .git/info/exclude (untracked, per-repo, per-machine) and drop the '*'. Verified: outer status clean + check-ignore matches, inner repo tracks files. Alt: a global core.excludesFile if you do yaks-private across many repos. Fully-self-contained (external gitdir + git add -f) is possible but fiddly; better handled by a future yaks-native sync.

---
▸ 2026-09-02T22:57:15Z
Obviated. Private + multi-machine is solved by the nested-repo pattern with zero yaks changes, so the YAKS_DIR/--herd override is not needed for it. Residual value (out-of-tree or non-.yaks-named herds) is captured as a deferred idea in yaks-c4b2. Slaughtering.
