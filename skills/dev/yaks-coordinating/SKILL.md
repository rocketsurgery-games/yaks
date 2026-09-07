---
name: yaks-coordinating
description: How agents and humans coordinate over a shared herd across different agent harnesses, without a heavyweight process. Experimental and repo-internal (yaks dogfooding); not shipped.
---

# Coordinating yaks (experimental)

Repo-internal conventions for coordinating work over a shared herd — across
harnesses, parallel agents, and humans. Deliberately minimal. The habits below
scale down to one agent and up to many; reach for the smallest one that keeps
the herd honest, and let yaks-working carry the per-yak trail.

## The design test (what belongs where)

Keep yaks core unopinionated. If a thing is expressible as a CLI query or
mutation over the task files, it belongs in the tool. If it assumes subagents,
worktrees, or a particular orchestration, it belongs in a skill or adapter, not
the tool. The files and the CLI are the contract; coordination is prose on top.

## Harness degradation

Design so the CLI is the whole coordination surface for the minimal case and the
durable spine for the rich one.

- **Minimal harness** (one agent loop, e.g. Pi): yaks is durable memory and a
  log. Notes are how the agent remembers across turns; the herd is its state.
- **Rich harness** (subagents + worktrees, e.g. Claude Code): yaks is a shared
  blackboard. Subagents claim disjoint yaks and read each other's notes.

The rich case adds nothing the minimal case needs; it just has more writers.

## Worktrees are per-branch herds

Each git worktree checks out its own committed `.yaks/`, so herds are
**per-branch** and reconcile at **merge**, not live. Two parallel agents in two
worktrees do not see each other's shave/update until those commits merge. So
coordinate by disjoint scopes plus merge, not by watching each other in real
time.

**File-tool SOP (validated across runs).** Some agent harnesses (e.g. Zed)
exclude git-ignored paths from the file-editing tools' project view AND root
those tools at the main checkout, not the terminal's cwd. The SOP:

1. **Put worktrees in a NON-gitignored in-project dir** (`wt/<name>`, *not*
   `.worktrees/`). Empirically the file tools refuse a *gitignored* worktree
   path ("path not found") but accept a non-gitignored one — validated with real
   sub-agents editing natively in `wt/`, no terminal fallback. This removes the
   fallback token-tax and restores native (fuzzy/batch) editing. Tradeoff: a
   non-gitignored worktree shows as `?? wt/` in the main checkout's `git status`
   (gitignore is what hid the old `.worktrees/` — the tools consult
   `git check-ignore`, so do NOT gitignore *or* `.git/info/exclude` the dir;
   either re-triggers the refusal). It's cosmetic: always stage explicit paths,
   never `git add .`, and `git worktree remove` after the run.
2. **Edit via the EXPLICIT worktree path** (`wt/<name>/src/foo.rs`), never a
   bare `src/foo.rs` (a bare path silently lands in the main tree). After each
   edit, run `git -C <main-checkout> status --short` and confirm no stray `src/`
   edits in main (a `?? wt/` line is expected). **Terminal fallback:** if a
   harness still refuses the path, edit through the terminal (cwd = the
   worktree) with anchored replacements — never a bare path.
3. Build/test the worktree's **own** freshly built binary, not the main one.

## Disjoint scoping

One writer per yak (yaks-working). For parallel work, hand each worker a
non-overlapping set of files — separate before serializing. If work is shared,
split it into child yaks first so each child has a single owner and a clean
file scope, then fan out.

**Disjoint scoping is about TYPES, not just files.** Disjoint files are
necessary but not sufficient: a change to a shared type (a struct with
exhaustive literal constructions across the codebase) breaks the *other* lane's
files at merge, even though the lanes never touched the same file. Editing such
a type is a cross-cutting change. Handle it one of two ways:

- Put the shared-type change in a **coordinator prep commit** on `main` first
  (add the field, fix every construction, land it), *then* fan the parallel
  wiring out on top; or
- Keep the whole change **within one lane**.

Never split a shared-type change across parallel lanes. Before fanning out, scan
for exhaustive constructions of any type a lane will modify (`grep 'TypeName {'`).

## Coordinator pre-flight (accreted from runs)

A tight checklist before fanning workers out; each item just points back at a
section above.

- **Scan shared types first** — `grep 'Type {'` for exhaustive constructions of
  any type a lane will touch; a shared-type edit goes in a prep commit or stays
  in one lane (Disjoint scoping).
- **Spawn workers fresh from `main`** so they start with the latest human
  feedback (Human-in-the-loop).
- **Assign disjoint file scopes** — one writer per yak (Disjoint scoping).
- **Expect human `.yaks/` drift and leave it untouched** — a worker's herd is
  its own branch; it reconciles at merge, not live (Worktrees are per-branch
  herds).
- **Verify each lane's branch is disjoint before merging.**
- **Squash-merge each lane with the yak id in the message** (Merge / integration).

## Attribution

Attribute notes so authorship is traceable across agents: `yaks update <id>
--as <actor> --note "..."`. The actor resolves `--as` → `$YAKS_ACTOR` (a
coordinator can pin it once per worker's env) → git `user.name`. It stamps the
note as `▸ <ts> [actor]`; committed status transitions are already attributed
by the git author, so this is for in-file/local-mode visibility. Attribution,
never ownership — the yak still belongs to no one. Single-agent work can ignore
it.

## Parallel run shape (claim → fan out → merge → reconcile)

The coordinator runs a batch in four beats:

1. **Claim, in one commit on `main`.** Create (or select) the batch's leaf
   yaks, move each to `shaving`, and stamp each with a short assignment note
   (owner tag, file scope, any setup context). Commit them together — e.g.
   `kick off run: shave A, B, C`. This “licks the cookie”: `main`'s `shaving`
   set now reflects exactly what's in flight (it otherwise wouldn't — a worker's
   own shave lives on its branch, invisible until merge), and each yak becomes
   self-describing for its worker.
2. **Fan out.** Cut a worktree per lane from that commit and spawn one worker
   each. Workers **skip the shave step** (their yak is already `shaving`); they
   just do the work and move `shaving → shorn` with an evidence note — ideally in
   a single commit.
3. **Merge back.** Squash-merge each lane (see below).
4. **Reconcile.** Any yak left `shaving` on `main` after the batch is an
   abandoned/stalled lane — `regrow` it (shaving→hairy) or re-run. `yaks doctor`
   surfaces stragglers.

## Merge / integration

The coordinator owns integration. One hard rule: **the yak id must appear in
whatever commit(s) land on `main`** — that is what `yaks commits` joins on, and
both a normal merge and `git merge --squash` satisfy it. Under the run shape
above the **claim commit already documents the batch**, so the parallel topology
no longer needs to live in the merge graph: **squash-merge is the default** for
single-commit lanes (cleaner history; provenance survives because the claim
commit and the worker commit both name the id, and `yaks commits` traces the
file across both). Reserve `--no-ff` for a lane that genuinely needs
multiple commits. Avoid per-yak cherry-picking across branches; to pull
main-side updates into a live branch use `git merge main`, all-or-nothing.

## PR-driven integration (coordinator owns the PRs)

Land a batch as GitHub PRs instead of local merges to `main`. **Mode decides the
shape** — settle it first (`git check-ignore .yaks`).

**Team mode** (`.yaks/` committed). The yak status-moves ride *inside* the PR
diff; yak ids stay allowed in commit messages; `yaks commits <id>` still recovers
provenance post-merge (via the file-follow — see below). Reviewers see `.yaks/`
churn in the diff; that's the tradeoff.

**Private mode** (`.yaks/` gitignored — use this whenever the repo has an external
tracker). The consequences cascade:

- The yak files are **not in the PR**, and yak ids must stay out of commit
  messages too (not just the PR body) — the whole `.yaks/` layer is invisible to
  the shared repo.
- **There is one herd, not per-branch herds.** A gitignored `.yaks/` isn't
  carried into worktrees, so workers get no copy to reconcile at merge. Point
  every worktree at the single shared herd — simplest is a symlink
  (`ln -s ../../.yaks wt/<name>/.yaks`), so discovery inside the worktree resolves
  to the one store. Yak surgery is therefore **live and shared**, not merged: the
  split-brain-herd problem disappears, and concurrent CLI writes to one herd take
  its place.
- Because the herd is shared, the **claim commit doesn't apply to yaks** (they
  aren't committed) — claiming is just moving the shared yak to `shaving`.

**The flow (coordinator owns `gh`; workers never touch it):**

1. Claim + fan out as usual — workers produce committed branches in their
   worktrees (team mode: plus the claim commit; private mode: just move the
   shared yak).
2. **Coordinator pushes each branch and opens the PR with id-free title/body.**
   Preflight the text first: `printf '%s' "$body" | yaks scan-ids` exits non-zero
   on any leaked id. Put the upstream link in with `yaks rollup --keys`, never a
   yak id.
3. Merge (squash by default, per Merge / integration).
4. **Stamp provenance back (private mode).** After merge, record the final
   squashed SHA on the yak (a note, or the `pull-request`/branch frontmatter
   fields) — the branch SHAs the worker knew are orphaned by the squash, and this
   recorded SHA is the *only* yak→commit link that exists in private mode.

Why coordinator-owns-PRs over each worker self-submitting: one `gh` auth, one
privacy-enforcement point, and one actor that knows the final merged SHA. Workers
stay pure "produce a branch" units.

**Provenance, by mode.**

- *Team:* `yaks commits <id>` joins on the yak **file** followed across the
  squashed commit. Squash rewrites the message (id-free, from the PR body), so the
  message-grep half breaks — the file-follow half survives. Anchor on the file
  move, not the message.
- *Private:* no git-side join is possible by design (the shared repo can't know
  yaks exist without leaking them). The yak carries its landed SHA; the
  coordinator stamps it post-merge (above).

**Guardrails.** `yaks scan-ids` on every PR title/body — and, in private mode,
over the landed commit-message range too. `yaks doctor` after the batch. Disjoint
scope as always. (A one-shot preflight that runs `scan-ids` over a PR body *and* a
commit range is a candidate primitive — yaks-eb5f.)

## Recovery (a lost or crashed worker)

A worker's work lives in its worktree/branch, so a lost agent session is **not**
lost work. If a worker crashes or returns unusable mid-flight, don't restart
from scratch — recover: `git -C wt/<name> status && git -C wt/<name> diff` to see
what's uncommitted, build/test the worktree's own binary to validate it, then the
coordinator commits + shears it (or discards and re-runs the lane). Validated
live: a mid-run harness crash left a compiling, passing implementation
uncommitted in its worktree, and the coordinator recovered it intact. The
worktree model is crash-resilient.

## Human-in-the-loop

Humans coordinate through the same notes. Raise a decision with `yaks ask <id>
--note "..."` (sets `needs`, drops the yak out of `next`); clear it with `yaks
answer <id> --note "..."` (human-reserved — an agent never clears its own block).
The human's queue is `yaks inbox`. Because yaks-working re-reads notes before
starting, feedback left on `main` is seen before work begins. Do not press past
a note that redirects the work.

**Human drift is the human's.** Expect the human to edit or create yaks in
`main`'s `.yaks/` mid-run. Leave their working-tree edits untouched — never
clobber a human note — and treat a human-created *untracked* yak as theirs to
introduce: track it only when they green-light it, otherwise leave it and flag
it. It reconciles like any other drift, at merge, not live.

**Across worktrees, HITL routes through the coordinator — not live.** A worker's
herd is its own branch; a human note on `main` does not reach an in-flight
worker, and chasing that is a trap. Instead: spawn workers **fresh from `main`**
(so they start with the latest human feedback), and when a worker hits a decision
it **hands back** — `yaks ask` on its leaf plus a clear final message — rather
than blocking and waiting. The coordinator relays to the human and re-spawns with
the answer, or makes the call. Live cross-worktree human→worker feedback is a
non-goal.
