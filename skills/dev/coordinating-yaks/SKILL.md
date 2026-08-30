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

## Disjoint scoping

One writer per yak (working-a-yak). For parallel work, hand each worker a
non-overlapping set of files — separate before serializing. If work is shared,
split it into child yaks first so each child has a single owner and a clean
file scope, then fan out.

## Optional attribution

When several agents share one herd, prefix notes with an actor tag so authorship
is traceable: `yaks update <id> --note "[wtA] merged onto main, tests green"`.
Single-agent work can ignore it. (`--as <actor>` is the candidate primitive,
`yaks-3901`.)

## Human-in-the-loop

Humans coordinate through the same notes. Leave feedback as a note — optionally
with a `needs-human` convention agents watch for. Because working-a-yak requires
re-reading the yak before starting (notes first), that feedback is seen before
work begins rather than after. Do not press past a note that redirects the work.
