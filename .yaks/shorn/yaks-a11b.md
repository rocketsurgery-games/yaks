---
id: yaks-a11b
title: 'yaks bulk: filter-driven field mutation (dry-run-default)'
type: feature
priority: 2
created: '2026-09-05T03:18:30Z'
updated: '2026-09-05T03:23:42Z'
parent: yaks-2ebe
labels:
- cli
---

Implements the filter half of bulk mutation per the yaks-7cc8 decision (DRY-RUN BY DEFAULT). New 'yaks bulk' subcommand: FilterFlags select the set, distinct mutation flags apply. Dedicated command (not --flags on update) because FilterFlags --priority/--type collide with update's set-priority/set-type. Field mutations only (labels/priority/type/reparent) — NO state transitions (filter-slaughter deferred as the scariest footgun).

---
▸ 2026-09-05T03:23:42Z [wt-cli]
Implemented 'yaks bulk' (src/main.rs) per yaks-7cc8 dry-run-default safety model. New Bulk command: #[command(flatten)] FilterFlags selector + distinct mutation flags --add-label/--remove-label/--set-priority/--set-type/--reparent/--unparent/--commit. Matched set via herd.list(build_spec(filter), false) (build_spec + store::load + filter::apply; filter.rs untouched). Hard safety: (1) FilterFlags::any_set() refuses unfiltered (exit 1); (2) refuse no mutation flag (exit 1); (3) DRY-RUN default prints 'would update N yaks:' + id/title list + mutation, writes nothing, exit 0; (4) only --commit applies, reusing update_many/reparent_many (per-id result, exit non-zero on any failure); (5) NO state transitions. Field edits + reparent can combine. describe_bulk_mutation() renders the preview/commit line. tests/cli.rs::bulk_dry_run_commit_and_refusals covers (a) filtered dry-run lists+unchanged, (b) --commit applies to matched set only (control yak untouched), (c) no-filter refuse, (d) no-mutation refuse. cargo test: 230 unit + 25 cli pass. Release-binary verified on throwaway herds: dry-run (no write), commit (labels+priority applied; control untouched), reparent dry-run+commit, refuse-unfiltered, refuse-no-mutation, --reparent/--unparent mutual-exclusion all correct. Scope: src/main.rs, tests/cli.rs only.
