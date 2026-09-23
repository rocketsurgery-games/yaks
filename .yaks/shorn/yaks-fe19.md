---
id: yaks-fe19
title: Rename attachment
type: feature
priority: 3
created: '2026-09-22T00:10:18Z'
updated: '2026-09-23T21:44:50Z'
labels:
- cli
- ui
---

Pasted PNGs tend to have ugly names. It would be nice to be able to rename them in the UI. And, for that matter, from the CLI as an option to `yaks --attach`.
Bonus points for an attach-time affordance in the "empty to paste PNG from clipboard" flow.

---
▸ 2026-09-23T21:35:37Z [coordinator]
Lightning-round lane 'attach' (worktree wt/attach, branch lane-attach). Scope: rename attachments: CLI + TUI (a09b is the TUI half). Evidence contract: gate = cargo test -p yaks green (re-accept + eyeball any snapshot ripple); PLUS drive the headless TUI (toque) to the relevant state and yaks attach the frame (text, or docshot SVG when color matters). Judge: coordinator at merge review; subjective forks -> yaks ask and hand back.

![rename-attachment-cli](artifacts/yaks-fe19/rename-attachment-cli.txt)

---
▸ 2026-09-23T21:44:40Z [lane-attach]
CLI evidence (scratch farm, cargo run): attach --name sanitizes 'login page (v2)' -> login-page-v2.png; rename-attachment moves file + rewrites link; collision refused exit 1.

---
▸ 2026-09-23T21:44:50Z [lane-attach]
Done. Core: Farm::rename_attachment(id, old, new) moves .yaks/artifacts/<id>/<old> and rewrites every artifacts/<id>/<old> path mention farm-wide (boundary-aware, so a.png.bak is untouched) plus the ![stem] alt text attach writes; refuses collision/missing/invalid; farm::attachment_name sanitizes (last path component, non [alnum._-+] -> '-', trims dots/dashes) and keeps the old extension when omitted. CLI: 'yaks attach --name <name>' + new 'yaks rename-attachment <id> <old> <new>' (mirrors rename / rename-prefix). TUI bonus: empty-path (clipboard PNG) attach now prompts 'Name pasted PNG (empty = paste-<ts>.png)'; refuses an existing name. Tests: farm unit x3, tests/cli.rs attach_name_and_rename_attachment, tui paste_attach_prompts_for_a_name. cargo test -p yaks --bins 284 ok, --test cli 26 ok. Docs: cli.md, tui.md, skills/yaks table, clap help.
