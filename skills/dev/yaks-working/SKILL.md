---
name: yaks-working
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

**Ask to *record* a judgment call, not only when you're stuck.** Beyond "ask when
you can't judge," raise a genuine design fork through `ask`/`answer` even when you
hold a clear lean — state the lean in the question and let the human's answer
stand as the decision. That turns a choice you'd otherwise make silently into an
attributed, durable thread on the yak: the fork, the reasoning, and who called it
— provenance for later yak archaeology. Reserve it for real forks (architecture,
semantics, scope), not trivia.

## While you work

- **Append progress as you go.** `yaks update <id> --note "what you found /
  decided / changed"`. Short, factual, one event per note. This running log is
  what future sessions and agents rely on.
- **One writer per yak.** If two actors need to touch the same work at once,
  split it into child yaks first so each has a single owner.

## Before you shear

- **Evidence contract first.** The definition-of-done — what will count as proof —
  is authored *before* the work: by the coordinator at claim time, or by the
  working agent itself in solo mode. You shear against it, not against vibes.
- **Evidence over assertion.** Do not shear on "it compiles", a self-report, or a
  green proxy. Verify against the *real artifact* via the project's verification
  lever — drive the TUI/browser, reproduce the state, read the actual value — and
  record it in a note: a command and its output, a test name, a file path, a
  commit SHA — or `yaks attach` the artifact itself (a screenshot, a TUI frame)
  when the evidence is something to look at. Attach keeps it an **external file**
  under `.yaks/artifacts/` (committed in team mode), so it reaches the reviewer;
  never paste a large or binary artifact (SVG included) inline into a note or
  description. Note that `yaks attach` also **modifies the tracked yak file** (it
  links the artifact in the body), so expect the yak `.md` to show as changed and
  stage it deliberately alongside the artifact. Evidence is general: the outcome achieved and
  seen, appropriate to the yak's kind (a research/decision yak's evidence is its
  finding/rationale; dead yaks are exempt).
- **Scriptable evidence is a `verify:` command.** When the check is a command (a
  test, a build, an artifact producer), store it on the yak (`--verify`) and run
  `yaks verify <id>`: it records the PASS/FAIL as a note and, because it's stored,
  anyone (coordinator, CI, a later agent) can re-run it. `doctor --strict` then
  enforces that a shorn yak's `verify:` last passed.
- **Authoring a *new* snapshot/golden needs your tool's accept step.** A fresh
  snapshot fails its first run by design (the tool writes a `.new` and errors),
  so know the accept path before you shear — e.g. with Rust `insta`, `cargo insta
  accept` or `INSTA_UPDATE=always cargo test` (plain `cargo test` writes
  `.snap.new` and fails; one test emitting two new snapshots can't self-bootstrap
  in a single plain run).
- **No lever? Ask, don't shear on faith.** If the project has no way to verify
  this kind of change (a TUI you can't snapshot, UI you can't drive, state you
  can't reproduce), `yaks ask <id>` the human whether to go build one rather than
  shearing on trust.
- **Don't self-shear what you can't judge.** Who judges is part of the contract.
  A script judges when your `verify:` passes — self-shear. A subjective call (look
  & feel) is the human's: `yaks attach` the artifact and `yaks ask`. Something an
  agent can eyeball but you shouldn't grade your own homework on: attach it and
  hand to the coordinator. Only self-shear when the judge is a passing script or the call
  is genuinely yours.
- Write a short shorn summary (what was done, what was learned, any yaks spawned,
  the evidence), then `yaks shorn <id>`.
- **Team mode:** stage the shorn yak move together with the code that completed
  it and commit them in one commit. That commit is also what later lets you trace
  the change back to this yak (provenance, `yaks-2610`).
- **The repo's own commit hooks are not yours.** A repo may run pre-commit hooks
  (husky/lint-staged, formatters, test gates) on your commit. Their output — and
  any failure they raise — is the repo's, not a yaks problem: fix the flagged
  code and re-commit; don't mistake hook noise for a yaks error.

## Optional: attribution (multi-agent)

When more than one agent shares a herd, name yourself in notes so others can
trace who did what. Attribution is first-class: `yaks update <id> --as <actor>
--note "..."` stamps the note `[actor]` (the actor resolves `--as` →
`$YAKS_ACTOR` → git `user.name`). Attribution, never ownership — the yak still
belongs to no one. Single-agent work can ignore it. See yaks-coordinating for
the multi-worker picture.
