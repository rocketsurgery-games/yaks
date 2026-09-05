---
name: yak
description: Yaks task tracking workflow. Use when a .yaks/ directory exists in the project. Provides commands and guidance for managing filesystem-native tasks stored as markdown files with YAML frontmatter.
activation:
  - .yaks/ directory exists in the project
---

# Yaks — Task tracking workflow

This project tracks work with Yaks. Tasks are markdown files with YAML frontmatter in `.yaks/`. You MUST follow this workflow to keep task state accurate.

## Running yaks

Yaks is a single self-contained binary — a plain command-line tool. Run it directly from your shell; there are **no slash commands**. Use the first invocation that works in your environment:

1. **`yaks <cmd>`** — if `yaks` is on `PATH` (installed via `npm i -g @rocketsurgery/yaks`, or a cargo-dist shell/Homebrew installer). Prefer this.
2. **`npx @rocketsurgery/yaks <cmd>`** — zero-install, if Node is available.
3. **`./target/release/yaks <cmd>`** — when working inside a checkout you've built with `cargo build --release`.

The npm package is `@rocketsurgery/yaks` (the unscoped `yaks` was taken, so it's published under the `rocketsurgery` org); the command it installs is `yaks`. Every example below is written as `yaks <cmd>` — substitute whichever invocation works for you. The CLI is stateless: each call is independent, there's nothing to keep running.

**Starting a fresh herd.** If there's no `.yaks/` directory yet, create one with `yaks init` (run in the repo root). It scaffolds `.yaks/{hairy,shaving,shorn,dead}/` plus a `config.yaml`; tune the defaults with `--prefix`, `--type`, and `--priority`. It refuses to clobber an existing herd, so it's safe to run.

Add `--json` to any query command (`list`, `show`, `next`, `tangled`, `search`, `stats`, `rollup`, `inbox`) for machine-readable output. `yaks create --json` also prints the new yak's id and file path, which is handy when you create a yak and immediately act on it.

## Terminology (say it right)

The three states are **adjectives** describing a yak: **hairy** (not started), **shaving** (in progress), **shorn** (done). The dead state (**slaughtered**) is hidden.

The **verbs** are the transitions, and they don't all match their state's spelling — say them the way the commands are named:
- **shave** a yak: hairy → shaving (start work).
- **shear** a yak / mark it **shorn**: shaving → shorn (finish). (Grammatically "shorn" is the past participle of *shear*, not *shave* — that's fine, it's a deliberate bit of archaic flavor.)
- **regrow**: shorn → hairy. **slaughter**: → dead. **revive**: dead → hairy.

"Yak shaving" is the background meme (endless incidental tasks); the tool's states and verbs are the precise vocabulary — prefer them over loose phrasing so humans and agents stay unconfused.

## Hard rules

1. **NEVER write code without an active shaving yak.** Before touching any code — even a one-line fix — you must have a yak in shaving state. If you don't, stop and `yaks shave <id>` one first (create it if needed). No exceptions.
2. **ALWAYS shear when a yak's work is done.** Run `yaks shorn <id>` as soon as the task is complete. In **team mode** (below), stage the shorn yak file alongside the code that completed it and commit them together. In **local-only mode**, never commit yak files at all.
3. **NEVER leak yak IDs into external-facing surfaces** (PR titles/descriptions, external issue trackers, and — in local-only mode — commit messages). See "Keep yaks private," below.

## Two workflows: local or team

Yaks runs in one of two modes, with different habits. **Figure out which mode you're in before you commit anything** — the signal is whether `.yaks/` is tracked by git:

- `.yaks/` is gitignored or otherwise untracked → **local-only** (a private scratchpad).
- `.yaks/` is committed alongside the code → **team** (a shared tracker).

To check: `git check-ignore .yaks` printing a path means local-only; `git ls-files .yaks` listing files means team. If a fresh checkout is genuinely ambiguous, default to local-only — the safer assumption.

**Local-only.** The yak files live only on this machine; they're planning memory, not shared history.
- Never `git add` yak files or include them in commits.
- Keep yaks invisible to everyone else: don't mention them — or their IDs — in commit messages, PR titles/descriptions, code comments, or external trackers. Describe the change in plain terms ("add retry logic"), not "shorn yak-1234".

Pick the hiding method that fits — they differ in blast radius:
- **Root `.gitignore`** (add a `.yaks/` line): simplest, but the ignore rule is itself committed, so the team sees that a herd exists.
- **`.yaks/.gitignore` containing `*`**: self-contained — the herd hides itself with no edit to the repo root. Use this **only** for a plain, non-nested local-only herd; the `*` also blinds any git repo *inside* `.yaks/`, so it's the wrong tool for the multi-machine pattern below.
- **`.git/info/exclude`**: per-repo and untracked, so nothing about the herd touches the committed tree. This is the right choice when `.yaks/` is itself a nested repo.
- **Global `core.excludesFile`**: ignore `.yaks/` across every repo on the machine at once — handy when you keep private herds in many projects.

> **Footgun:** `git clean -fdx` in the outer repo deletes an ignored/excluded `.yaks/` (and, in the nested case, its git history) — `-x` sweeps ignored files too. A gitignored `.yaks/` is also **not** carried into fresh clones or other git worktrees, so it's simply absent there. Push a private herd often, or you can lose it.

**Local-only across machines.** To sync a private herd between machines without committing it to the code repo, give `.yaks/` its **own** git repo on a private remote, nested inside the project:
- `cd .yaks && git init`, add a private remote, and commit the herd there. Run all herd git ops from inside `.yaks/`.
- Hide the nested repo from the **outer** repo with `.git/info/exclude` — never the `*` trick, which would also blind the herd's own repo. The outer repo then ignores `.yaks/` cleanly instead of flagging it as an embedded repo.
- Habit: **pull before, push after** a work session so machines stay in sync. yaks needs no configuration for this — discovery finds `.yaks/` exactly as always.

**Team.** The yak files are part of the repo — treat them like code.
- Commit the shorn yak move together with the code that completed it (hard rule 2).
- Yak IDs are fine in commit messages and other in-repo references — collaborators can resolve them from the committed files.

> This repo is **team mode**: `.yaks/` is committed, and yak IDs in commit messages are expected.

## Keep yaks private (external surfaces)

Whichever mode you're in, yaks are a **private, fine-grained layer**. Keep yak IDs and `[yaks:…]` markers out of anything a broader audience reads — **pull-request titles/descriptions and external issue trackers** (Jira, Linear, GitHub Issues). This one keeps going wrong under light guidance, so treat it as firm: leaking a yak ID upstream is almost never right.

Enforce it mechanically with **`yaks scan-ids`**: it reads a file and/or piped stdin and exits non-zero if any token is a real yak-id in this herd (printing each as `line:col  id`). Wire it into a pre-commit or PR hook to catch a leaking id before it ships — especially valuable in local-only mode, where yak IDs must stay out of commit messages too.

Most projects that use an external tracker don't use yaks team-wide; yaks roll **up** to those issues. When a PR or issue needs a reference, use the **external** key, not the yak ID: run `yaks rollup --keys` over the shipping set and paste that — the forge links the PR to the issue natively. The **yak-tracker** skill covers this projection in full.

## You are working alongside a human

The herd is shared. A human may be editing yaks in the `yaks tui` (or by hand) **while you work** — creating yaks, moving them between states, jotting notes. So expect **working-tree drift in `.yaks/`** that you didn't cause: a touched `updated:` timestamp, a yak moved `hairy ↔ shaving`, a new file you didn't create. This is **normal and expected**, not an error to flag or fix.

- Don't revert, restage, or "clean up" `.yaks/` changes you didn't make.
- When committing (team mode), stage **only** the specific yak files your work touched — never `git add .yaks` wholesale.
- Treat such drift as signal, not noise: a yak the human just moved to shaving, or a note they added, often tells you what they care about right now. If it seems to redirect the work, follow it or ask.

## Asking a human (ask / answer / inbox)

When you hit a decision only a human should make — an ambiguous requirement, a risky tradeoff, a missing credential — **don't guess**. `yaks ask <id> --note "the question"` blocks the yak on a human: it sets the `needs` field, records your question as an attributed note, and drops the yak out of `yaks next` so you (or another agent) won't pick it back up while it waits. The human replies with `yaks answer <id> --note "the decision"`, which clears the block and returns the yak to `next`. `yaks inbox` lists everything currently awaiting a human. Attribute the note with `--as <actor>` (or set `$YAKS_ACTOR`) so the log shows who asked.

In `yaks tui` the same flow is one key: `a` asks or answers the selected yak in place, an **Inbox** view lists the awaiting-a-human queue, blocked yaks carry a ⏳ badge with a warning accent, and `m` marks rows for a bulk state change over the selection.

## Workflow

1. **Session start** — run `yaks list` and `yaks next` to see current state.
2. **Before writing code** — `yaks shave <id>` (create the yak first if needed).
3. **While working** — append progress notes with `yaks update <id> --note "what you found / decided / changed"`. This builds a running log in the markdown body so future sessions have context.
4. **When the work is done** — `yaks update <id> --note "…"` with a brief shorn summary (what was done, what was learned, any yaks spawned), then `yaks shorn <id>`. In team mode, stage the shorn yak move together with the code and commit them in one commit whenever practical.

## Parent/child state rules

A parent yak's state should reflect its children:
- When you shave a child, shave the parent too (if it's still hairy).
- When you shear the last unshorn child, shear the parent too.
- NEVER leave a hairy parent with shorn children — that means work was done but the parent doesn't reflect it.

## Labels — keep them few and purposeful

Labels are for slicing the herd later (`yaks list --label ui`), not for elaborate classification. Absent discipline, agents invent sprawling taxonomies that help no one.

- **Reuse before inventing.** Check what already exists (`yaks list`, `yaks stats`) and prefer an existing label over a near-synonym (`ui` vs `interface` vs `frontend` — pick one).
- **Prefer broad, durable areas** (`ui`, `search`, `docs`, `skills`, `rust`) over hyper-specific one-offs. A label earns its keep only if you'd plausibly filter on it.
- **A few is plenty.** Zero labels is fine; more than ~2–3 on one yak is usually a smell.
- **Tracker labels:** when a yak maps to an external issue, a label named for the tracker (`jira`, `github`, `linear`) makes the upstreamed yaks easy to find. The **yak-tracker** skill covers this.

## Commands

Run these directly from the shell (see **Running yaks** above for the exact invocation).

| Command | What it does |
|---------|-------------|
| `yaks init` | Scaffold a new `.yaks/` herd in the current directory. `--prefix`, `--type`, `--priority`, `--emacs`. Works without an existing herd |
| `yaks create` | Create a new task (in hairy). Title is positional (`yaks create "Fix the login crash"`; `--title` still works for back-compat); `--type`, `--priority`, `--parent`, `--labels`, `--depends-on`, `--source`, `--description`, `--json` (print the new id + path) |
| `yaks list` | List tasks with optional filters (`--all` also includes dead) |
| `yaks show` | Show full details of a task |
| `yaks refs` | List what a task points at (parent, deps, id mentions), flagging danglers |
| `yaks update` | Update fields, labels (`--add-label`/`--remove-label`), or append a `--note`. Accepts multiple ids (same edit to each); `--as <actor>` attributes a note |
| `yaks ask` | Block a yak on a human decision (sets `needs`, drops it from `next`); records the question as an attributed `--note` |
| `yaks answer` | Clear a yak's `needs` block and record the reply; returns it to `next` |
| `yaks inbox` | List yaks awaiting a human (the `needs` inbox) — `list --needs` across all statuses |
| `yaks shave` | Start shaving a yak (hairy → shaving) [alias: `work`] |
| `yaks shorn` | Mark a yak shorn (shaving → shorn) [alias: `close`] |
| `yaks regrow` | Regrow a shorn yak (shorn → hairy) [alias: `reopen`] |
| `yaks slaughter` | Slaughter a yak (move to hidden `dead/`) — for ideas you won't pursue or tasks that got obviated |
| `yaks revive` | Revive a dead yak back to hairy |
| `yaks next` | Hairy tasks whose deps are all resolved [alias: `ready`] |
| `yaks tangled` | Hairy tasks with at least one unresolved dep [alias: `blocked`] |
| `yaks search` | Substring search over id/title/description |
| `yaks dep` | Add/remove a dependency between tasks |
| `yaks reparent` | Move a task under a new `--parent` (or `--unparent` to top-level) |
| `yaks rename` | Rename a yak + rewrite every reference to it across the herd |
| `yaks rename-prefix` | Migrate all yaks from one id prefix to another; `--dry-run` to preview |
| `yaks stats` | Show task statistics |
| `yaks rollup` | Group yaks by the external issue they roll up to (`--keys` for just the keys) |
| `yaks doctor` | Read-only herd-integrity check (duplicate-status ids, dangling parent/dep refs); exits non-zero on issues |
| `yaks scan-ids` | Scan a file and/or stdin for tokens that are real yak-ids in this herd — a private-mode leak check; exits non-zero if any are found |
| `yaks tui` | Open the interactive terminal UI |

The state-transition verbs (`shave`, `shorn`, `regrow`, `slaughter`, `revive`) and `reparent` accept **multiple ids** in one call, applying the same move to each. Attribute any note with `--as <actor>` (else `$YAKS_ACTOR`, else the git user); attribution never implies ownership.

## Task format

Tasks live in `.yaks/hairy/`, `.yaks/shaving/`, or `.yaks/shorn/` as `.md` files. Slaughtered tasks live in `.yaks/dead/` and are excluded from every default query — pass `--all` (or `--status dead`) to `list` to find them. Status is implicit from the directory. Metadata is YAML frontmatter; the markdown body is the description.

```markdown
---
id: yak-a1b2              # flat, opaque, and stable — never encodes hierarchy
title: Fix the login crash
type: bug
priority: 2
created: "2026-02-16T10:00:00Z"
updated: "2026-02-16T10:30:00Z"
parent: yak-c3d4         # optional; present only on child tasks
depends_on:
  - yak-e5f6
labels:
  - auth
source: https://jira.example.com/browse/PROJ-123  # optional external issue URL
---

Details go here.
```

Child tasks use `--parent <id>` on create. Every ID is flat (`{prefix}-{4hex}`) and stable for the task's whole life; the parent/child relationship lives in the `parent:` frontmatter field, not in the ID. Move a task with `yaks reparent <id> --parent <new>` (or `--unparent`), which just rewrites that one field. `yaks show` displays parent and children automatically.

> Older herds may still contain dotted IDs (e.g. `yak-a1b2.1`) created before this change. Those dots are now just opaque characters — the `parent:` field is authoritative — so don't parse IDs to infer hierarchy.

The prefix, default type, and default priority come from `.yaks/config.yaml` (falling back to `yak` / `task` / `3`).

### External source linking

Use `--source <url>` on create or update to link a yak to an external issue (Jira, GitHub Issues, Linear, etc.). The URL is stored in the `source` frontmatter field. The relationship is a **one-way projection**: the yak points at the external issue, never the reverse, and the external tracker stays unaware of yaks.

Many yaks can roll up to one external issue. `yaks rollup` groups yaks by their source (a yak with no `source:` inherits its nearest ancestor's, so one stamp on an umbrella yak covers the subtree); `yaks rollup --keys` lists the external keys to paste into a PR body. For seeding a yak from an external issue or drafting a status update back to one, see the **yak-tracker** skill.

## Referencing other yaks

Beyond the structural links (`parent:`, `depends_on:`), you can mention one yak from another's title, description, or a note just by writing its **full id** — `{prefix}-{4hex}`, using **this herd's configured prefix** (in `.yaks/config.yaml`; don't hardcode a prefix you saw in another repo). A mention is recognized by matching the token against real yak ids, so:

- **Always write the full id** (`yak-0af1`), never the bare 4-hex shorthand (`0af1`). Only the full form is detected and linked; a bare tail reads as ordinary prose.
- `[[yak-0af1]]` wiki-brackets work too and render as a bare link.
- Because matching is against real ids (not a prefix regex), mentions keep resolving even in a herd mid-migration with mixed prefixes.

In `yaks tui`, a mention is highlighted and followable (Tab / `[` / `]` to cycle, Enter to follow).

Use **`yaks refs <id>`** to see everything a yak points at — parent, dependencies, and id mentions in its text — with any dangling formal reference flagged. It's a quick integrity check before or after edits.

### Renaming safely

`yaks rename <old> <new>` renames a yak and rewrites **every** reference to it (parent, `depends_on`, and id mentions in bodies/titles) across the herd, matching whole ids only so lookalike prose is left untouched. `yaks rename-prefix <old> <new>` does the same for a whole prefix at once (e.g. migrating `yaksrs` → `yaks`) and updates `.yaks/config.yaml`. Both take **`--dry-run`** — always preview a bulk rename first.

## Filtering

Every query command (`list`, `search`, `next`, `tangled`) shares the same filter flags. AND across dimensions; within a repeatable flag, OR:

- `--status S` / `--type T` / `--priority P` / `--label L` (all repeatable)
- `--search Q` — substring match on title/description/id
- `--ready` / `--tangled` — dep-state filters
- `--parent-of ID` — only descendants of ID

Examples:
- `yaks list --type bug --type feature --priority 1` — urgent bugs or features
- `yaks list --label auth --search retry` — auth-labeled tasks mentioning "retry"
- `yaks next --type bug` — ready bugs only
