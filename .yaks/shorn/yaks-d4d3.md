---
id: yaks-d4d3
title: No-yak-ids validation tool
type: feature
priority: 1
created: '2026-09-04T12:10:00Z'
updated: '2026-09-05T03:04:46Z'
labels:
- eval
---

CLI tool for scanning arbitrary text for valid yak-ids, using the same validation mechanism used by the rendering code to highlight yak-links.

To be used by a pre-commit check in private-mode herds, to ensure that yak-ids aren't leaking into the repo. Similar affordance for upstream PRs and issues.

---
▸ 2026-09-05T03:04:46Z [wt-cli]
Shipped scan-ids leak check. refs::scan_text (+FoundRef) is the non-TUI reusable primitive: normalizes wikilinks per line like the TUI, layers line:col on refs::scan, validated against store::all_ids (prefix-agnostic, same membership test the renderer highlights with). CLI subcommand scan-ids reads a FILE and/or piped stdin, prints 'line:col  id', --json array, exits non-zero on any hit (pre-commit gate). Verified on worktree binary: 'yaks-d4d3' -> '1:14 yaks-d4d3' exit=1; [[yaks-d4d3]] alias -> col 3 exit=1; fix-0004 via file -> exit=1; fake yaks-9999 -> exit=0. Tests: tests/cli.rs scan_ids_flags_real_ids_and_is_clean_otherwise + 2 refs unit tests. cargo test: 228 lib + 24 cli, 0 failed. Native edit_file worked on the wt/ worktree path. Scope: src/refs.rs, src/main.rs, tests/cli.rs only.
