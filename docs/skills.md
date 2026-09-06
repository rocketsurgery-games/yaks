# Skills & workflows

Skills are prose guidance that rides *on top of* the tools — the least direction
that measurably changes an agent's behavior. yaks keeps the tools unopinionated
and puts methodology in skills, so different working styles fit the same core.

Two skills ship in the binary and install via `yaks skills install` (into
`~/.agents/skills` by default); two more are experimental and repo-internal
(under `skills/dev/`, not shipped).

| Skill | Ships? | Use it when… |
|---|---|---|
| **`yaks`** | ✓ | Managing tasks in a `.yaks/` herd — the commands, the workflow, solo vs team/private modes. |
| **`yaks-tracker`** | ✓ | Relating yaks to external issue trackers (Jira/Linear/GitHub) as a one-way roll-up projection. |
| **`yaks-working`** | dev | Taking one yak from hairy to shorn with a legible trail. |
| **`yaks-coordinating`** | dev | Coordinating a shared herd across parallel agents and humans. |

## The design test (what is a tool vs a skill)

If a thing is expressible as a CLI query or mutation over the task files, it
belongs in the **tool**. If it assumes sub-agents, worktrees, or a particular
orchestration, it belongs in a **skill**. The files and the CLI are the
contract; coordination is prose on top. A convention graduates from skill to
tool (a flag, a field, a `doctor` check) once it has earned its place.

## `yaks-working` — one yak, honestly

The per-yak discipline the other skills lean on:

- **Notes first.** Re-read a yak (freshest note first) before starting — a human
  or another agent may have added feedback or moved it.
- **One writer per yak.** Split shared work into child yaks so each has a single
  owner and a clean file scope.
- **Evidence before shear.** Don't move a yak to `shorn` without a note that
  records what was done and how it was verified (`doctor --strict` checks this).
- **Ask, don't guess.** On a decision that needs a human, `yaks ask` and hand
  back — never clear your own `needs` block.

## `yaks-coordinating` — careful parallelism

Conventions for running a small number (2–4) of agents over one herd reliably.
The load-bearing ideas, all validated by dogfooding:

- **Run shape: claim → fan out → merge → reconcile.** The coordinator makes one
  commit that moves the batch's yaks to `shaving` with per-yak assignment notes
  ("licks the cookie", so `main`'s `shaving` set reflects what's in flight),
  cuts a git worktree per lane, spawns one worker each (workers skip the shave —
  their yak is already shaving), squash-merges each lane back (the yak id in the
  message), and regrows any yak left stranded in `shaving`.
- **Disjoint scope is about *types*, not just files.** A change to a shared type
  breaks the other lane at merge even across disjoint files; put it in a
  coordinator prep-commit first, or keep it in one lane.
- **Human-in-the-loop routes through the coordinator.** Workers `ask` and hand
  back; the human answers on `main` (`yaks inbox`); the next spawn starts fresh
  from `main`. No live cross-worktree feedback.
- **Recovery.** A worker's work lives in its worktree, so a lost session isn't
  lost work — recover from the worktree rather than restarting.
- **Integrity.** `yaks doctor` after a batch catches merge damage; across many
  runs the disjoint-leaf model has kept herds corruption-free.

## Workflows at a glance

- **Solo:** `create` → `next` → `shave` → `update --note` → `shorn`. The herd is
  durable memory; notes are how you (or an agent) remember across sessions. Keep
  it private with one of the hiding options in [README](README.md#solo-vs-team-mode).
- **Team:** commit `.yaks/` with the code. Yak surgery lands alongside the change
  it describes; `yaks commits <id>` recovers provenance from git; ids may appear
  in commit messages but never in PR titles or external trackers (`scan-ids`
  guards that).
- **Parallel agents:** the coordinator drives the claim → fan-out → squash →
  reconcile shape above, with `ask`/`answer`/`inbox` for human decisions and
  `doctor` for integrity.
