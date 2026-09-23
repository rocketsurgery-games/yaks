---
id: yaks-63f3
title: Open a yak by id (jump to its detail)
type: feature
priority: 2
created: '2026-08-23T02:49:48Z'
updated: '2026-09-23T21:39:45Z'
labels:
- ui
---

We already have a type-ahead search facility. We should make it easier to use this to find a random yak by id.

---
▸ 2026-09-23T21:35:37Z [coordinator]
Lightning-round lane 'openid' (worktree wt/openid, branch lane-openid). Scope: jump to a yak by id from the TUI. Evidence contract: gate = cargo test -p yaks green (re-accept + eyeball any snapshot ripple); PLUS drive the headless TUI (toque) to the relevant state and yaks attach the frame (text, or docshot SVG when color matters). Judge: coordinator at merge review; subjective forks -> yaks ask and hand back.

![openid-goto_picker_with_id_typed](artifacts/yaks-63f3/openid-goto_picker_with_id_typed.txt)

---
▸ 2026-09-23T21:39:36Z [lane-openid]
Frame: '#' opened the Go-to-yak picker (Hairy view), 'b0' typed -> 1 match (the Shaving yak).

![openid-goto_jumped_to_detail](artifacts/yaks-63f3/openid-goto_jumped_to_detail.txt)

---
▸ 2026-09-23T21:39:36Z [lane-openid]
Frame: after Enter -> switched to the Shaving view, detail pane on b0, notification '→ b0'; 'o' returns to a0.

---
▸ 2026-09-23T21:39:45Z [lane-openid]
Done. Key: '#' (list + detail panes; free in both, mnemonic for 'yak #id'; '/' left as list search since it filters rows rather than jumping). Opens the existing fuzzy picker (FuzzyAction::Goto) over ALL yaks — id-prefix > id-substring > title match. Enter -> new App::goto_task (factored out of follow_link): switches to the target's status view, expands ancestors, opens detail, pushes o/i history. Docs: help_content ('Search & filter'), docs/tui.md. help_hint left alone (bar already full; avoids snapshot ripple). Tests: 3 new (picker->Enter->detail + o back w/ 2 insta snapshots; title match from detail; ad-hoc search doesn't block). cargo test: unit 281 ok + toque ok; tests/cli.rs slaughter_guard_and_family fails only because the shared CARGO_TARGET_DIR binary/test is being rebuilt by the 'slaughter' lane (panic cites tests/cli.rs:602; this worktree's file is 556 lines).
