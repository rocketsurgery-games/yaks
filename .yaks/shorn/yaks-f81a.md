---
id: yaks-f81a
title: 'a3a6 CLI: --needs filter flag; inbox = list --needs sugar'
type: feature
priority: 3
created: '2026-09-03T22:28:44Z'
updated: '2026-09-03T22:35:08Z'
parent: yaks-a3a6
labels:
- cli
---

After the prep lands: add --needs to FilterFlags + wire into build_spec (main.rs). Lane scope: main.rs only.

---
▸ 2026-09-03T22:35:08Z [wt-cli]
Wired --needs into CLI: added bool flag to FilterFlags and set needs_only: f.needs in build_spec (src/main.rs); noted inbox == list --needs across statuses. Added tests/cli.rs::list_needs_filters_to_blocked (temp-herd, passes). Verified on worktree release binary: 'list' shows both yaks, 'list --needs' shows only the ask'd (needs:human) yak; --needs also composes with search. cargo test: 21 passed. filter.rs untouched.
