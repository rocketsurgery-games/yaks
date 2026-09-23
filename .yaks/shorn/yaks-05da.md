---
id: yaks-05da
title: Slaughter family with confirmation
type: feature
priority: 3
created: '2026-09-07T23:42:25Z'
updated: '2026-09-23T21:43:13Z'
labels:
- ui
---

There's a guard against slaughtering a parent if any of its children are un-slaughtered. We should allow this to be forced, with a confirmation.

---
▸ 2026-09-23T21:35:37Z [coordinator]
Lightning-round lane 'slaughter' (worktree wt/slaughter, branch lane-slaughter). Scope: force-slaughter a family with confirmation (farm + CLI flag + TUI confirm overlay). Evidence contract: gate = cargo test -p yaks green (re-accept + eyeball any snapshot ripple); PLUS drive the headless TUI (toque) to the relevant state and yaks attach the frame (text, or docshot SVG when color matters). Judge: coordinator at merge review; subjective forks -> yaks ask and hand back.

![slaughter-confirm-frame](artifacts/yaks-05da/slaughter-confirm-frame.txt)

---
▸ 2026-09-23T21:43:04Z [lane-slaughter]
Headless TUI frame: X on a parent with 2 live descendants raises the family confirm (overlay=confirm), naming the count.

![slaughter-cli-transcript](artifacts/yaks-05da/slaughter-cli-transcript.txt)

---
▸ 2026-09-23T21:43:04Z [lane-slaughter]
CLI transcript: plain slaughter refuses (exit 1, names descendants, points at --family); --family slaughters grandchild, child, parent.

---
▸ 2026-09-23T21:43:13Z [lane-slaughter]
Shorn. Core: Farm::slaughter(id, family) -> SlaughterOutcome + farm::live_descendants (transitive, deepest-first, cycle-safe) in src/farm.rs, shared by CLI and TUI. CLI: new `yaks slaughter <id> --family`; plain slaughter now REFUSES a yak with live descendants (previously the CLI had no guard -- only the TUI did), exit 1 with an error naming them and pointing at --family. TUI: X on a yak with live descendants now raises a y/N confirm 'Slaughter <id> (<title>) AND its N live descendants?' (ConfirmAction::SlaughterFamily) instead of refusing; bulk S→x still skips parents (unchanged). Docs: cli.md, tui.md, README, skills/yaks SKILL.md, clap help, help_content. Tests: farm unit, CLI test, TUI flow + snapshot (replaced slaughter_refused_with_children snapshot). cargo test -p yaks green (282 unit + 26 cli).
