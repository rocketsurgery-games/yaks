# Archaeology gaps: the 2026-09-23 UI lightning round

Part of yaks-77a5, under yaks-64c4. Question: after a large parallel run (11
worktree lanes, 3 design scouts, 1 coordinator, ~10.5 min wall clock),
how much of *what happened* can be rebuilt from durable sources?

## What made reconstruction possible (keep doing this)

| Source | What it gave | Query |
|---|---|---|
| Claim commit (`b196a1e`) + a claim note per yak | run start, the full yak set, lane/worktree/branch/scope for each yak, evidence contract | `git show b196a1e`; `yaks log --since` |
| Attributed notes (`--as lane-<name>`, `design-scout`, `coordinator`) | per-lane progress, evidence, and shorn times to the second; who said what | `yaks log --since <t> --json` |
| `yaks attach` evidence | what each lane actually saw (frames, transcripts, SVG/PNG) | `.yaks/artifacts/<id>/` |
| Yak ids in squash messages | the merge time and merged diff for each yak | `yaks commits <id>`, `git log --grep` |
| Asks | open decisions, with their context | `yaks inbox` |

Rebuilding the run this way was possible **because the coordinator followed the
yaks-coordinating run shape**. A run without the claim notes and lane actors
would only have the squash commits left.

## Gaps

Ranked by how much was lost, and whether it can be recovered at all.

1. **Lane commits and branch topology: lost by squash + branch delete.**
   Squash merges keep the yak → `main` link and drop the lane's own commit
   (its SHA, its time, and its diff before conflict resolution). Deleting the
   branch also deletes its reflog. The commits survive only as unreachable
   objects until `git gc` prunes them (about 2 weeks by default). They can be
   found with `git fsck --unreachable` plus a grep of commit messages for yak
   ids, but that only works while they last and with that trick in hand.
   *Captured this time:* `refs/archive/lightning-2026-09-23/lane-*` (local, not pushed).
   *Candidate fixes:* a `Lane: <sha>` trailer in the squash message; a
   coordinator note at merge (`merged lane <sha> as <squash>`); an archive-ref
   convention in yaks-coordinating.

2. **Agent lifecycle: exists only in the harness.** Spawn time, end time,
   duration, tool uses and tokens for each agent came from task notifications
   in the coordinator's context. Nothing durable records them. Spawn time here
   was *derived* (lane commit − duration).
   *Candidate:* the coordinator writes a start note ("spawned") and an end note
   with the metrics (a harness-agnostic convention, per the yaks-coordinating
   design test: orchestration belongs in skills, not the tool).

3. **Integration work: mostly invisible.** The semantic merge fixes (the
   `goto_task`×`NavEntry` reconciliation, the missing `Event::Mouse` arm, the
   Tab count broken by the form's new row) are the most instructive part of
   the run. They show up only as a line or two in squash messages, mixed into
   the squashed diff. The textual resolutions (append/append, interleaved
   hunks, re-applied patches) aren't recorded at all.
   *Candidate:* an `Integration:` section in each squash message, and/or a
   coordinator merge-log note on each yak.

4. **Coordinator decisions and near-misses: recorded nowhere.** Recorded
   nowhere: the triage (why these 11 lanes, why these groupings, why scouts),
   the shared-target decision driven by disk space, the claim hiccup (lanes
   briefly cut from the pre-claim commit), and the SendMessage broadcast to
   running lanes.
   *Candidate:* a run-log note on the run's umbrella or claim, written as it
   happens, not afterwards.

5. **No durable handle for the run.** Nothing ties the 14 yaks together as
   "the 2026-09-23 lightning round" except the claim commit, which you have to
   find by grepping. The input set (the human's **stars**) is per-user TUI
   state outside the repo (`~/.config/yaks/<slug>/working_set.json`), and it
   still held 7 stale `yaksrs-*` ids from before a herd rename.
   *Candidates:* a `run` umbrella yak that the claim note references;
   `refs/archive/<run>/…`; recording the input set in the claim commit.

6. **State-transition times are coarse.** The farm doesn't timestamp
   hairy→shaving→shorn moves (only `updated:`). In team mode git keeps them,
   but a squash folds a lane's shave and shear into one merge commit, so the
   shear time comes only from the lane's shorn-summary note, if it wrote one.
   Several claim notes share one second (21:35:37Z), so ordering inside a batch
   is lost.

7. **Worktree lifecycle: inferred only.** `wt/<lane>` creation and removal
   are unrecorded. The claim notes name the worktree but not when it existed.

8. **Human drift is only visible as working-tree state.** The human's
   concurrent edits (answers, shaves, new yaks) are uncommitted, so they have
   no git time. Notes give answer times; shaves and slaughters are visible
   only as file moves, and only as last-touched.

9. **Mixed clocks.** Notes are in UTC (`Z`), git in local time (−0400). This
   is harmless but a trap when merging timelines by hand; a stitching tool
   should normalize.

10. **Build-environment side effects: not in any artifact.** The shared
    `CARGO_TARGET_DIR` caused cross-lane test-binary clobbering, and lanes
    worked around it in different ways. Future runs would benefit from
    knowing this, but only the lanes' final reports (in the harness) and one
    or two notes mention it.

## What a stitching tool would join

Every event in the timeline came from joining four streams on yak id + time:
(a) `yaks log --json` (notes: actor, ts, id), (b) `git log` on main (claim +
squashes, id-grep), (c) lane commits (archive refs), (d) harness metrics
(lost unless captured). (a)–(c) are joinable today with a script. (d) needs a
capture convention. That makes a `yaks timeline`-style view (or a skill
recipe) feasible without any new on-disk format, as long as runs leave (1),
(2) and (5) behind.
