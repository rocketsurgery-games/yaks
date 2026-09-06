---
id: yaks-f85f
title: Final docs review pass
type: task
priority: 3
created: '2026-09-06T23:02:22Z'
updated: '2026-09-06T23:47:04Z'
parent: yaks-012b
labels:
- docs
---

After the skill pass: re-read docs for any remaining issues/inconsistencies (with skills + reality).

---
▸ 2026-09-06T23:47:04Z [coordinator]
Docs review pass: read all four docs (README, cli, tui, skills) against the now-consistent skills and the CLI. Found + fixed 3: (1) CLI help for 'doctor --strict' claimed 'shorn/dead yaks' but the impl (herd.rs) flags SHORN only and explicitly exempts dead -> fixed the clap help in main.rs to match docs+skill (this was the reverse of a doc bug: code help was wrong); (2) docs/skills.md 'install via `yaks skills`' -> '`yaks skills install`' (skills requires the install subcommand); (3) docs/cli.md command cell 'skills' -> 'skills install'. Verified accurate as-is: filter-flag list, --json coverage, log --since/--by, create positional title, dep add|remove <id> <dep>, rename/rename-prefix, bulk safety rails, HITL trio, attribution chain, solo/team hiding options. Embedded SVG paths (assets/tui-*.svg) resolve from docs/tui.md. cargo test --workspace green (233+25+13+1).
