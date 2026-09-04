---
name: coordinating-yaks
description: How agents and humans coordinate over a shared herd across different agent harnesses, without a heavyweight process. Experimental and repo-internal (yaks dogfooding); not shipped.
---

# Coordinating yaks (experimental)

Repo-internal conventions for coordinating work over a shared herd — across
harnesses, parallel agents, and humans. Deliberately minimal. The habits below
scale down to one agent and up to many; reach for the smallest one that keeps
the herd honest, and let working-a-yak carry the per-yak trail.

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

**File-tool SOP (validated across runs).** Some agent harnesses root their
file-editing tools at the main checkout, not at the terminal's working
directory, even when the worktree lives under the project root. A bare
`src/foo.rs` edit then silently lands in the main tree while the terminal `cd`
sees the worktree. The SOP that reliably prevents it:

1. Edit through the **explicit** worktree path (`.worktrees/<name>/src/foo.rs`).
   **Fallback:** if the harness's edit tool *refuses* that path because
   `.worktrees/` is gitignored, make the edit through the **terminal** (cwd = the
   worktree) with anchored replacements — never fall back to a bare path. Across
   three runs the main tree stayed `src`-clean either way.
2. After each edit, run `git -C <main-checkout> status --short` and confirm the
   main tree shows no stray `src/` edits.
3. Build/test the worktree's **own** freshly built binary, not the main one.

Refinement: a linked worktree's `.yaks/` never shows up in the main checkout's
`git status` (git skips nested worktrees), so the main-clean guard effectively
reduces to “no `src/` changes in main.” Followed faithfully, this gotcha has not
bitten in practice.

## Disjoint scoping

One writer per yak (working-a-yak). For parallel work, hand each worker a
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
- **`--no-ff` merge with the yak id in the message** (Merge / integration).

## Attribution

Attribute notes so authorship is traceable across agents: `yaks update <id>
--as <actor> --note "..."`. The actor resolves `--as` → `$YAKS_ACTOR` (a
coordinator can pin it once per worker's env) → git `user.name`. It stamps the
note as `▸ <ts> [actor]`; committed status transitions are already attributed
by the git author, so this is for in-file/local-mode visibility. Attribution,
never ownership — the yak still belongs to no one. Single-agent work can ignore
it.

## Merge / integration

The coordinator owns integration. One hard rule: **the yak id must appear in
whatever commit(s) land on `main`** — that is what `yaks commits` joins on, and
both a normal merge and `git merge --squash` satisfy it. Above that rule it is
topology taste: `--no-ff` merges are **preferred in team mode** because they
preserve the parallel-lane topology and richer `yaks commits --follow` history
(the yak's own notes already carry the fine-grained work-trail). Avoid per-yak
cherry-picking across branches; to pull main-side updates into a live branch use
`git merge main`, all-or-nothing.

## Human-in-the-loop

Humans coordinate through the same notes. Raise a decision with `yaks ask <id>
--note "..."` (sets `needs`, drops the yak out of `next`); clear it with `yaks
answer <id> --note "..."` (human-reserved — an agent never clears its own block).
The human's queue is `yaks inbox`. Because working-a-yak re-reads notes before
starting, feedback left on `main` is seen before work begins. Do not press past
a note that redirects the work.

**Across worktrees, HITL routes through the coordinator — not live.** A worker's
herd is its own branch; a human note on `main` does not reach an in-flight
worker, and chasing that is a trap. Instead: spawn workers **fresh from `main`**
(so they start with the latest human feedback), and when a worker hits a decision
it **hands back** — `yaks ask` on its leaf plus a clear final message — rather
than blocking and waiting. The coordinator relays to the human and re-spawns with
the answer, or makes the call. Live cross-worktree human→worker feedback is a
non-goal.
