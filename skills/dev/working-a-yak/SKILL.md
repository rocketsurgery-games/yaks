---
name: working-a-yak
description: Minimal, harness-agnostic conventions for taking one yak from hairy to shorn with a legible trail. Experimental and repo-internal (yaks dogfooding); not shipped.
---

# Working a yak (experimental)

Repo-internal conventions for working a single yak with a trail a human or
another agent can trust later. Deliberately minimal. Prefer the smallest habit
that keeps the herd honest; do not grow this into a heavyweight flow.

Run the yaks CLI directly. In this checkout use `./target/release/yaks` (the
`yaks` on `PATH` may be older); elsewhere use `yaks`.

## Before you start

1. **Re-read the yak now, not from memory.** `yaks show <id>`. A human or another
   agent may have added notes, left feedback, or moved it since you last looked.
   Read the latest note first; it is the freshest signal.
2. **Confirm it is actually ready.** Dependencies resolved (`yaks next` /
   `yaks tangled`), and no note asks for something to happen first. If a note
   redirects the work, follow it or ask rather than pressing on.
3. **Claim it.** `yaks shave <id>` moves it to shaving and signals to everyone
   sharing the herd that it is taken. Shave its parent too if the parent is still
   hairy.

**When a decision needs a human, ask and hand back — don't block-and-wait.**
`yaks ask <id> --note "<question>"` records the question, sets the yak's `needs`
block, and drops it from `yaks next` until it is resolved; then return control
rather than spinning. Pending questions surface in the human's `yaks inbox`.
Clearing the block is human-reserved (`yaks answer <id>`) — never answer your own
ask.

## While you work

- **Append progress as you go.** `yaks update <id> --note "what you found /
  decided / changed"`. Short, factual, one event per note. This running log is
  what future sessions and agents rely on.
- **One writer per yak.** If two actors need to touch the same work at once,
  split it into child yaks first so each has a single owner.

## Before you shear

- **Evidence over assertion.** Do not shear on "it compiles" or a self-report.
  Record the real evidence in a note: a command and its output, a test name, a
  file path, a screenshot path, or a commit SHA.
- Write a short shorn summary (what was done, what was learned, any yaks spawned,
  the evidence), then `yaks shorn <id>`.
- **Team mode:** stage the shorn yak move together with the code that completed
  it and commit them in one commit. That commit is also what later lets you trace
  the change back to this yak (provenance, `yaks-2610`).

## Optional: attribution (multi-agent)

When more than one agent shares a herd, name yourself in notes so others can
trace who did what: prefix a note with an actor tag, e.g. `[opus] rebased onto
main, tests green`. Single-agent work can ignore this. A first-class
`--as <actor>` is a candidate primitive under `yaks-3901`.
