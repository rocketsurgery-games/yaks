---
id: yaks-594b
title: Surface needs + attribution across the TUI/CLI
type: task
priority: 2
created: '2026-09-03T17:58:45Z'
updated: '2026-09-04T17:07:48Z'
parent: yaks-3901
labels:
- agent
- ui
---

eb66 (actor attribution) and b517 (needs-human block) landed at the FILE + CLI-command level, but the read/query/render surfaces lag. Dogfooding found the model works on disk while the UI is blind to it (Joel: 'yaks ask yaks-b517 did the right thing at the file level, but there are no UI affordances to render/query it'). Umbrella for the affordances that make needs + attribution first-class in both surfaces. Children are the concrete gaps; each is a small, independently shippable slice.

---
▸ 2026-09-03T20:12:50Z [coordinator]
SECOND PARALLEL RUN — done, clean. Two-lane disjoint run (the point: workers coordinating while USING the attribution/needs work we just landed). CLI lane (wt/cli-needs, main.rs+json.rs): yaks-45c7 (row marker) + yaks-29a3 (show renders extra). TUI lane (wt/tui-needs, tui.rs+tui/detail.rs): yaks-548b (badge + 'i' inbox toggle + 'a' ask/answer keys) + yaks-eeba (detail renders extra).

RESULT: zero-conflict merge on BOTH lanes (2nd was a real 3-way). 219 tests green integrated (each worker added its own; +8 over baseline). Verified on merged binary: 'yaks show yaks-b517' now surfaces Joel's real 'wat: foo' under 'Other fields:'; a blocked yak shows '⚠ needs:human' in list. TUI parts covered by the worker's headless tests (badge/inbox/ask-answer/detail).

DISJOINT-SCOPE DISCIPLINE HELD: pre-caught the FilterSpec cross-lane hazard (exhaustive literal constructions in tui/) and descoped 45c7's --needs filter so neither lane touched a shared type — result was a trivially clean merge. This is the key lesson: shared-TYPE edits (not just shared files) are the real coupling; keep them in a coordinator prep commit or one lane.

OBSERVATIONS on the sub-agents: (1) file-tool gotcha did NOT bite either — explicit .worktrees/<name>/ paths + post-edit main-status check worked again; promote to SOP. (2) Sharp refinement from the CLI worker: a linked worktree's .yaks/ changes never show in 'git -C <main> status' (git skips the nested worktree), so the main-clean guard effectively reduces to 'no src/ there'. (3) Both correctly identified the 012b/b517 .yaks drift as pre-existing HUMAN notes and left them untouched — the disjoint-ownership + 'leave human drift' convention held under parallelism. (4) TUI worker completed ask/answer keybindings without deferral because Overlay::Edit/commit_edit already existed (open_comment pattern).

FRICTION (agent-reported, actionable): 'yaks shave'/'create' don't accept --as (attribution only on note-bearing verbs) — transition attribution still relies on git author (per eb66 scoping); 'create' rejects a bare positional title, requires --title (yaks-2120). Both hit by both workers.

594b cluster now: 45c7, 29a3, 548b, eeba, 1603, 1edb, 4e8a, bc68, ffa5 all shorn; only follow-ups remain (accent styling, --needs filter deferred).

---
▸ 2026-09-04T17:07:48Z [coordinator]
COMPLETE. needs + attribution are first-class across CLI+TUI now. Attribution: shared actor::resolve (1603), TUI comment [actor] parse+render (1edb), auto-attributed TUI comments (ffa5). needs visibility: detail field (4e8a), warning accent (685e), CLI row marker (45c7), inbox status-invariant (bc68). Preserved-unknown-frontmatter render: CLI show (29a3) + TUI detail (eeba). Inbox as filter/view (a3a6). All children shorn across runs 2-5; herd stayed integrity-clean throughout (yaks doctor).
