# yaks

A filesystem-native task tracker. Tasks are plain markdown files with YAML
frontmatter, kept in a `.yaks/` directory inside your project — no database, no
daemon, no server. A task's status is implicit in *which folder it lives in*, so
your task list is just files you can read, grep, edit, and commit alongside your
code.

yaks ships as a single self-contained binary. Startup is effectively instant,
which matters because the workflow is lots of small command invocations.

```text
.yaks/
  hairy/     todo          (a hairy yak, not yet shaved)
  shaving/   in progress   (you're shaving it)
  shorn/     done          (shorn)
  dead/      abandoned     (slaughtered; hidden from normal queries)
```

## Install

**npm** (prebuilt binary for your platform; the command is `yaks`):

```sh
npm i -g @rocketsurgery/yaks     # then: yaks --help
# or zero-install:
npx @rocketsurgery/yaks list
```

**From source** (needs a Rust toolchain):

```sh
git clone https://github.com/rocketsurgery-games/yaks
cd yaks
cargo build --release
./target/release/yaks --help
```

## Quick start

A herd is just a `.yaks/` directory. Create one at your project root and start
tracking:

```sh
mkdir .yaks
yaks create --title "Wire up the login form" --type feature --priority 2
yaks list
yaks shave <id>     # start work  (hairy -> shaving)
yaks shorn <id>     # finish      (shaving -> shorn)
```

That's the whole loop: **shave** a yak before you work on it, **shear** it
(`shorn`) when it's done.

## Concepts

**States are adjectives, verbs are transitions.** A yak is *hairy* (todo),
*shaving* (in progress), or *shorn* (done); *slaughtered* yaks go to a hidden
`dead/`. You **shave** (hairy→shaving), **shear** / mark **shorn**
(shaving→shorn), **regrow** (shorn→hairy), **slaughter**, and **revive**.

**Task file.** Frontmatter for metadata, the markdown body for the description:

```markdown
---
id: yak-a1b2
title: Wire up the login form
type: feature          # bug | feature | task | idea
priority: 2            # 1 urgent … 5 lowest (default 3)
created: "2026-02-16T10:00:00Z"
updated: "2026-02-16T10:30:00Z"
parent: yak-c3d4       # optional; only on child tasks
depends_on: [yak-e5f6] # optional
labels: [auth]         # optional
source: https://…      # optional external issue URL
---

Longer description goes here.
```

IDs are flat and stable (`{prefix}-{4hex}`). Hierarchy lives in the `parent:`
field, not in the ID. Dependencies are ids in `depends_on:`; a yak is *ready*
when all of them are shorn (or dead), and *tangled* otherwise.

## Commands

| Command | What it does |
|---------|--------------|
| `yaks create` | Create a task (`--title`, `--type`, `--priority`, `--parent`, `--labels`, `--depends-on`, `--source`, `--description`) |
| `yaks list` | List tasks; filter by `--status/--type/--priority/--label/--search`, `--ready`, `--tangled`, `--parent-of`, `--all` |
| `yaks show <id>` | Full detail for one task, with parent + children |
| `yaks update <id>` | Change fields/labels, set `--description`, or append a `--note` |
| `yaks shave <id>` | hairy → shaving (alias: `work`) |
| `yaks shorn <id>` | shaving → shorn (alias: `close`) |
| `yaks regrow <id>` | shorn → hairy (alias: `reopen`) |
| `yaks slaughter <id>` / `revive <id>` | move to / from the hidden `dead/` |
| `yaks next` / `tangled` | ready tasks / dependency-blocked tasks |
| `yaks search <q>` | substring search over id/title/description |
| `yaks dep` / `reparent` | edit dependencies / move under a new parent |
| `yaks rollup` | group yaks by the external issue they roll up to (`--keys` for a PR body) |
| `yaks stats` | task statistics |
| `yaks tui` | open the interactive terminal UI |

Add `--json` to any query command for machine-readable output.

## Interactive TUI

`yaks tui` opens a full-screen browser over the herd — views by state, a detail
pane, inline create/edit, dependency and reparent pickers, search, and an
embedded modal editor (vim or emacs keybindings, per `.yaks/config.yaml`). It
auto-refreshes when the files change underneath it, so it stays in sync if you
(or an agent) edit yaks from elsewhere.

## Use with an AI coding agent

yaks includes an **agent skill** so assistants drive it correctly (shave before
coding, shear when done, keep task state honest). It's a plain skill — it just
shells out to the `yaks` CLI (or `npx`), so there's nothing to run as a plugin.

Install the skill straight from the binary — no clone required:

```sh
yaks skills install                          # -> ~/.agents/skills  (yak + yak-tracker)
yaks skills install --dir ~/.claude/skills   # any agent's skills dir; --force to overwrite
```

It activates when a `.yaks/` directory is present, and shells out to the `yaks`
binary (or `npx @rocketsurgery/yaks`), so make sure one of those is on the
agent's `PATH`.

Prefer a universal, multi-agent skills manager? The skills are standard
Anthropic-format `SKILL.md` files, so [openskills](https://github.com/numman-ali/openskills)
installs them too:

```sh
npx openskills install rocketsurgery-games/yaks
```

## Public and private herds

Because a herd is just a `.yaks/` directory, *you* decide whether it's shared or
private by choosing whether git tracks it.

**Public (team).** Commit `.yaks/` alongside the code. The task list travels with
the repo, shows up in PRs and `git log`, and yak moves merge with the change that
completed them. This project works this way — it tracks its own work in a
committed herd.

**Private (local-only).** Keep `.yaks/` out of the code repo and it becomes a
personal scratchpad no one else sees. Hide it whichever way fits:

- a `.yaks/` line in the root `.gitignore` — simplest, but the rule is committed;
- a `.yaks/.gitignore` containing `*`, so the herd hides itself with no change to
  the repo root — for a plain, non-nested herd only;
- `.git/info/exclude`, which is per-repo and never committed;
- a global `core.excludesFile`, to ignore `.yaks/` across every project at once.

**Private across machines.** To carry a private herd between machines without
committing it to the code repo, give `.yaks/` its own git repo on a private
remote, nested inside the project:

```sh
cd .yaks
git init && git remote add origin <your-private-remote>
# work from inside .yaks/ for herd git ops; pull before, push after
```

Hide the nested repo from the outer repo with `.git/info/exclude` (not the `*`
trick, which would also blind the herd's own repo). yaks needs no configuration —
it discovers `.yaks/` exactly as before.

> **Heads up:** `git clean -fdx` in the outer repo will delete an ignored or
> excluded `.yaks/`, including a nested herd's history. Push a private herd
> often, and remember a gitignored `.yaks/` won't appear in fresh clones or other
> worktrees.

## Configuration

Optional per-project config lives in `.yaks/config.yaml`:

```yaml
prefix: yak            # id prefix (default "yak")
default_type: task     # default --type
default_priority: 3    # default --priority
vim_mode: true         # TUI editor keybindings: vim (true) or emacs (false)
```

A user-global `~/.config/yaks/config.yaml` is merged underneath per-project
values.

## License

Apache-2.0.
