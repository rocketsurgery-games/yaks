# UI lightning round, 2026-09-23 — coordinator context capture

Captured post facto (yaks-77a5) from the coordinator's conversation context:
facts that existed **only** in the session transcript and would otherwise be
lost. Times are local (-0400) unless marked Z. Everything that *is* in git or
the farm is referenced, not duplicated.

## Run input

- Trigger: human asked to "churn through" their **starred** UI yaks in parallel
  worktrees + sub-agents, asking (`yaks ask`) on judgement calls.
- The starred set is **per-user TUI state, not in the repo**:
  `~/.config/yaks/84e3c4b50ca0/working_set.json` (slug of the farm path). At run
  time it held 26 ids; 7 were stale `yaksrs-*` ids (pre-rename herd, no longer
  resolvable), leaving 19 hairy yaks:
  9009 4da6 28b4 158d 63f3 f2aa b0b4 027a 05da 7cb3 1bbd 2d17 683f 97a2 953f
  9e59 a09b b2ed fe19 (all `yaks-`).

## Triage (coordinator decisions, not recorded elsewhere)

| Bucket | Yaks | Why |
|---|---|---|
| 11 implementation lanes | see lane table | concrete, scoped |
| 3 read-only design scouts | 158d; 953f+1bbd; 683f | "break into research + design" / cross-cutting; scout writes proposal + `yaks ask` |
| coordinator ask, no lane | b2ed | human's own note suspected it was done; partly covered by yaks-5656 |

Groupings: 4da6+b0b4 (both the create/edit form), 027a+2d17 (render polish),
fe19+a09b (a09b is the TUI half of fe19). 28b4 split off from 158d as a local
precursor. 7cb3's parent f04d was shaved per the parent/child rule but is not
itself done.

## Pre-flight facts

- **Disk: 13 GB free (99% full)**; `target/` was 5.5 GB. 11 per-worktree
  targets would not fit → decision: **one shared `CARGO_TARGET_DIR`** (main's
  `target/`) for all lanes. This decision caused the run's main friction (below).
- 14 CPUs.
- Human drift present at start (left untouched throughout): uncommitted edits to
  158d, 182f, b2ed; 2344 + 9aac slaughtered; 27c6 + b47b newly created.

## Claim-commit hiccup (invisible in history)

The first claim attempt failed: a zsh loop `for i in $ids` does not word-split,
so `git add` got one bogus path and the commit aborted — but the 11
`git worktree add` calls in the same command still ran, cutting every lane from
the **pre-claim** commit `f38acd8`. Caught from the `worktree list` output;
worktrees + branches removed and re-cut from the real claim commit `b196a1e`
(17:35:54). No trace of this remains in git or the farm.

## Spawn

All 14 agents were launched in a single coordinator message at ≈17:36:00–17:36:10
(derived: lane commit time − agent duration). Every lane's brief carried: explicit
`wt/<lane>` paths only, shared `CARGO_TARGET_DIR`, never run `target/debug/yaks`,
evidence contract (tests green + `yaks attach` a headless frame), ask-and-hand-back
on judgement calls, one commit on the lane branch, no merge/push.

## Lane table

Duration/tool-uses/tokens come from the harness's task notifications (not
persisted anywhere in the repo). Lane SHAs are now pinned at
`refs/archive/lightning-2026-09-23/lane-<name>` (local refs only — not pushed).

| Lane | Yaks | Lane SHA | Lane commit | Agent dur | Tools | Tokens | Squash on main | Merged | Merge conflicts |
|---|---|---|---|---|---|---|---|---|---|
| scroll | 9009 | 58467a5 | 17:37:35 | 88s | 14 | 40.8k | 7112fba | 17:38:18 | none |
| openid | 63f3 | 7a69fac | 17:39:46 | 168s | 22 | 67.7k | 53b8a6d | 17:41:06 | tests.rs (append/append) |
| width | 9e59 | 0ae2789 | 17:40:38 | 208s | 30 | 48.7k | 656e78b | 17:42:01 | tests.rs (append/append) |
| navpos | 28b4 | f767815 | 17:41:43 | 246s | 31 | 51.1k | 2f7299a | 17:43:01 | detail_nav.rs (**semantic**) |
| form | 4da6 b0b4 | 8f25dde | 17:43:03 | 412s | 48 | 95.8k | e147369 | 17:43:28 | none |
| polish | 027a 2d17 | c5908a3 | 17:43:12 | 407s | 56 | 77.1k | 5898905 | 17:43:45 | tests.rs (append/append) |
| slaughter | 05da | e58c77a | 17:43:20 | 400s | 47 | 79.6k | b2788be | 17:44:03 | none |
| mouse | 97a2 | 47f046e | 17:43:34 | 366s | 39 | 96.9k | 6490372 | 17:44:29 | render.rs (both-keep) |
| paste | f2aa | 4829d28 | 17:44:00 | 447s | 67 | 102.5k | ec3106e | 17:45:18 | docs/tui.md, runtime.rs, tests.rs (interleaved) + **2 semantic** |
| attach | fe19 a09b | 2d4324a | 17:44:56 | 454s | 60 | 97.8k | 8b7aa68 | 17:45:49 | docs/tui.md, farm.rs, main.rs, render.rs, tests/cli.rs |
| labels | 7cb3 | 387391c | 17:45:18 | 505s | 55 | 102.2k | 6f4e4ec | 17:46:23 | docs/cli.md, docs/tui.md, create.rs, handlers.rs, tests/cli.rs |

Design scouts (no worktree, no code; wrote to main's farm directly):

| Scout | Yaks | Dur | Tools | Tokens | Output |
|---|---|---|---|---|---|
| drafts | 683f | 74s | 9 | 44.1k | 683f-design.md + ask |
| global nav | 158d | 122s | 12 | 67.3k | 158d-design.md + ask; found the follow_link dead-yak bug → yaks-cc52 |
| global search | 953f, 1bbd | 135s | 21 | 54.5k | search-design.md + ask; root cause: sticky toast, not the filter header |

Totals: 14 agents, ≈1.03M sub-agent tokens, 511 tool uses; longest agent 505s.
Wall clock claim→last merge: 10m29s. Sum of agent durations: ≈68 min → ≈6.5×
effective parallelism.

## Merge log (coordinator; mostly not in commit messages)

Merge gate for every lane: `cargo test --workspace` in a **separate**
`target-main/` dir (to dodge the shared-target race), squash-merge, delete
worktree + branch. Merge order = completion order.

- **openid, width, polish** — both sides appended tests at the end of
  `src/tui/tests.rs`; resolved by concatenation (scripted: `/tmp/append_resolve.py`).
  Two different marker shapes occurred (closing `}` inside vs outside the block).
- **navpos** — *semantic* conflict: openid had factored `follow_link` into
  `goto_task` (pushing bare ids to history) while navpos changed history entries
  to `NavEntry {id,line,scroll}`. Resolved by making `goto_task` snapshot
  `current_nav_entry()` before moving. Coordinator also updated the `?` help
  text ("restores position") — a lane had deliberately skipped it to avoid
  render.rs contention.
- **mouse** — render.rs: `list_offset` (scroll lane) vs hit-rect recording
  (mouse lane) on the same lines; kept both. Checked that the width lane's new
  modal mode (list not rendered) can't leave a stale clickable list rect: hits
  are reset per frame.
- **paste** — `tests.rs` conflict was interleaved (git paired unrelated hunks);
  resolved by taking main's file and re-appending the lane's single additive
  hunk. Two **semantic** integration breaks that textual merge alone would not
  catch:
  1. paste's new `pump_events`/`handle_event` replaced the loop's direct
     `match`, which the mouse lane had extended — mouse events would have been
     silently dropped. Coordinator added the `Event::Mouse` arm.
  2. paste's tests Tab 4× to reach the description; form lane's new `source`
     row made it 5× — caught by the merge-gate test run.
- **attach, labels** — pure-additive files (`farm.rs`, `tests/cli.rs`)
  re-applied as patches onto main; docs/help rows hand-merged to keep both lanes'
  text; labels × form in `handlers.rs` (source edit + `label_edit`) needed one
  stray line removed after resolution. `cargo fmt` run after attach.
- Final: clippy 15 warnings vs 16 pre-run baseline (no new ones); docshots
  regenerated with no diff.

## Cross-lane friction: the shared target dir

- `tests/cli.rs` runs `target/debug/yaks`, which every lane overwrote → lanes
  saw CLI failures from *other lanes'* code (first reported by openid, citing a
  line number beyond its own file's length).
- Worse, cargo sometimes considered a lane's test binary fresh when it was built
  from another worktree → new tests silently not run. Lanes coped differently:
  `touch` sources to force rebuild (form, slaughter, attach, width), release
  profile (paste), a lane-private profile `--profile lanelabels` (labels,
  ≈400 MB), or skipping `tests/cli.rs` (navpos, mouse, openid).
- Coordinator broadcast a warning to the 9 still-running lanes via SendMessage
  after openid's report (~17:40).
- Mouse lane once built into its own worktree `target/` (620 MB) when the env
  var didn't take; it cleaned up.
- Leftovers: `target-main/` and `target/lanelabels/` (removal was
  permission-denied for the coordinator).

## Outputs recorded elsewhere

- Asks raised: b2ed, 97a2, 7cb3 (coordinator); 158d, 683f, 953f (scouts).
- Follow-up bugs filed: yaks-cc52 (follow_link to an unviewed yak), yaks-a211
  (detail_page counts the sticky title row).
- Behavior changes flagged to the human: CLI slaughter guard (`--family`),
  `update --source ""` clears, `doctor` malformed-label check.
- Human answers arrived later (2026-09-24T01:04–01:06Z) on 7cb3 and 97a2.
