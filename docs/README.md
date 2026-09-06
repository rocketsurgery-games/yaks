# yaks

**yaks** is a filesystem-native task tracker: a single self-contained Rust binary.
Tasks ("yaks") are markdown files with YAML frontmatter under a `.yaks/`
directory. A yak's **status is implicit in which subdirectory it lives in** —
there is no database and no separate state file. The files are the whole truth:
readable, greppable, diffable, and versioned in git alongside the code they
describe.

```
.yaks/
  hairy/     # not started
  shaving/   # in progress
  shorn/     # done
  dead/      # abandoned (slaughtered)
  config.yaml  schema
```

## The model

- **Files are authoritative; any index is derived.** Status is the directory.
  Hierarchy is a frontmatter `parent:` field. Ids are flat and stable.
- **Tiny state machine.** Four statuses, five transitions (`shave`, `shorn`,
  `regrow`, `slaughter`, `revive`). Everything else — priority, type, labels,
  dependencies, a `needs` block, notes — is metadata that pushes nuance out of
  the state machine and into fields you can filter on.
- **Notes are the work-trail.** Each yak accumulates timestamped, optionally
  attributed notes (`▸ <ts> [actor]`). The notes are the durable record of what
  happened and why; git history is the record of what *changed*.

## Quick start

```sh
yaks init                      # scaffold a .yaks/ herd here
yaks create 'Wire up auth'     # new hairy yak (prints its id)
yaks next                      # what's ready to work (deps met)
yaks shave <id>                # start it (→ shaving)
yaks update <id> --note '...'  # record progress
yaks shorn <id>                # finish it (→ shorn)
yaks tui                       # or drive it all interactively
```

## Where to go next

- **[CLI reference](cli.md)** — every command, grouped by what it's for.
- **[TUI guide](tui.md)** — the interactive terminal UI and its keys.
- **[Skills & workflows](skills.md)** — the agent skills that ride on top of the
  tools, and the workflows they encode (solo, team, and careful multi-agent
  parallelism).

## Solo vs team mode

- **Team mode:** `.yaks/` is committed with the code. The herd is shared, visible
  in git/PRs, and yak surgery merges alongside the changes it describes.
  Provenance is recoverable from git (`yaks commits <id>`).
- **Solo / private mode:** `.yaks/` is not committed to the code repo. Hide it
  via the root `.gitignore`, a self-contained `.yaks/.gitignore` of `*` (only
  when the herd is *not* its own git repo), `.git/info/exclude` (per-repo,
  untracked), or a global `core.excludesFile`. For a private herd synced across
  machines, give `.yaks/` its own `.git` on a private remote and hide it from the
  outer repo via `.git/info/exclude`. See the **yaks** skill for the full
  breakdown and footguns.
