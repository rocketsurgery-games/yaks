# CLI reference

Run `yaks <command> --help` for full flags. Most read commands accept the shared
**filter flags** and `--json`; most note-writing commands accept `--as <actor>`.

## Filter flags (shared)

`--status <s>` · `--type <t>` · `--priority <n>` · `--label <l>` · `--herd <h>`
(id prefix) · `--search <text>` · `--parent-of <id>` (descendants at any depth) · `--ready` ·
`--tangled` · `--needs`. They compose, and back `list`, `search`, `log`, and the
`bulk` selector.

## Read / query

| Command | What it does |
|---|---|
| `list` | List tasks (non-dead by default; `--all` includes dead). Status renders as `[H]`/`[S]`/`[N]`/`[X]` — hairy / shaving / shor**n** / dead. |
| `show <id>` | Show one yak: fields, references, children, body + notes. |
| `next` (alias `ready`) | Hairy yaks whose dependencies are all resolved — the work queue. |
| `tangled` (alias `blocked`) | Hairy yaks with at least one unresolved dependency. |
| `search <text>` | Substring search over id / title / description. |
| `stats` | Counts by status, type, priority. |
| `log` | Timestamped notes across a filtered set, oldest first (`--since <2h\|3d\|date>`, `--by <actor>`). |
| `refs <id>` | The yaks a task points at (parent, deps, id mentions), flagging danglers. |
| `commits <id>` | Git commits linked to a yak — those naming its id, and those that touched its file across status moves. |
| `rollup` | Group yaks by the external issue they roll up to (`--keys` lists the external keys). |
| `scan-ids [file]` | Scan text/stdin for real yak-ids; prints `line:col  id`, exits non-zero if any found — a leak check for private farms (pre-commit / PR hook). |

## Create / edit

| Command | What it does |
|---|---|
| `create '<title>'` | New hairy yak. Title is positional or `--title`; `--type`/`--priority`/`--parent`/`--herd`/`--labels`/`--depends-on`/`--source`/`--description`/`--verify`; `--json` prints id + file path. `--labels` (like `--add-label`/`--remove-label`/`--label` everywhere) splits on commas and whitespace — a label may contain neither, so `--labels ui,docs`, `--labels 'ui, docs'`, and `--labels ui docs` all give `[ui, docs]` (duplicates dropped). `--herd` sets the new yak's herd (id prefix; default: the config herd), so one `.yaks/` can hold several herds. |
| `update <ids…>` | Update fields/labels or append a `--note`; the same edit applies to every id. `--as <actor>` attributes the note. `--verify '<cmd>'` sets (or, empty, clears) the yak's verification command; `--source <url>` likewise sets (or, empty, clears) its external `source:`. |
| `dep add\|remove <id> <dep>` | Add / remove a dependency. |
| `reparent <ids…> --parent <id>` | Move yaks under a new parent (or `--unparent` to top-level). |
| `rename <old> <new>` | Rename a yak's id, updating every reference across the farm. |
| `rename-herd <old> <new>` | Rename a whole herd: migrate every id from one prefix to another (e.g. `yaksrs` → `yaks`). (`rename-prefix` still works as an alias.) |
| `merge <path> [--dry-run]` | Merge another farm's yaks (all statuses + artifacts) into this one, preserving ids and status, and declaring any incoming herd in this farm's `herds:` config so it shows up in the pickers. Non-destructive (source left intact); refuses on id collisions — reconcile the source's herd with `rename-herd` first. |
| `bulk <filter> <mutation>` | Filter-driven field edit. **Dry-run by default** — prints the matched set + the mutation and changes nothing without `--commit`. Requires ≥1 filter flag (never the whole farm) and ≥1 mutation flag (`--add-label`/`--remove-label`/`--set-priority`/`--set-type`/`--reparent`/`--unparent`). Field edits + reparent only — no state transitions. |

## Verification

| Command | What it does |
|---|---|
| `verify <ids…>` | Run each yak's verification command with live output, and record `verify: <cmd> -> PASS/FAIL (exit N)` as an attributed note. Exits non-zero if any fails. Explicit only — never auto-run. The command is the yak's own `verify:` field if set, else the config default resolved by the yak's labels (see below). The scriptable form of a yak's evidence contract: a check anyone (or CI) can re-run. |
| `attach <id> <path>` | Copy a local file under `.yaks/artifacts/<id>/` and link it in the yak's body — artifact evidence (screenshots, TUI frames) a human or coordinator can view. `--name <name>` stores it under a different filename (sanitized; the source's extension is kept if omitted). `--note`/`--as` record an attributed note. The non-scriptable, look-at-it form of evidence: an **external file** (committed in team mode — the farm un-ignores `.yaks/artifacts/`), never inlined. |
| `rename-attachment <id> <old> <new>` | Rename `.yaks/artifacts/<id>/<old>` (e.g. an ugly `paste-….png`) and rewrite every link to it across the farm. `<new>` is sanitized and keeps `<old>`'s extension if it omits one; refuses to overwrite an existing attachment. |

Default verify commands live in `.yaks/config.yaml` under a nested `verify:` map (label → command, plus an optional `default`); a yak with no explicit `verify:` field resolves its command from the first of its labels with an entry, else `default`. Name the project's levers once:

```yaml
verify:
  ui: cargo test -p yaks
  cli: cargo test -p yaks
  default: cargo test --workspace
```

Per-herd overrides live under a `herds:` map (one entry per id prefix), each optionally setting `default_type`, `default_priority`, or its own `verify:` map. Any setting cascades a single level — the herd's value if present, else the farm-global — and the `herds:` keys are the known-herd set the create/TUI picker offers:

```yaml
herd: core
verify:
  default: cargo test
herds:
  core:
    verify: { default: cargo test --workspace }
  web:
    default_type: feature
    verify: { default: npm test }
```

A lever is the pass/fail **gate** for its label — what `doctor --strict` enforces at shear. For UI yaks that's the tui snapshot tests (`cargo test -p yaks`); the SVG docshots are a separate *visual* channel, generated on demand (`cargo test -p yaks docshots -- --ignored`) and attached for human review, not a gate.

## State transitions (all accept multiple ids)

| Command | Move |
|---|---|
| `shave` (alias `work`) | → shaving (start) |
| `shorn` (alias `close`) | → shorn (done) |
| `regrow` (alias `reopen`) | shorn → hairy |
| `slaughter` | → dead (abandon) |
| `revive` | dead → hairy |

`slaughter` refuses a yak that still has live (non-dead) descendants, since that
would orphan them (it names them and exits non-zero). `slaughter <id> --family`
takes the whole family instead: every live descendant, deepest first, then the
yak itself.

## Human-in-the-loop

| Command | What it does |
|---|---|
| `ask <id> --note '<question>'` | Block a yak on a human: sets the `needs` field, dropping it out of `next`. |
| `answer <id> --note '<reply>'` | Clear the `needs` block (human-reserved). |
| `inbox` | Yaks awaiting a human — equivalent to `list --needs` across all statuses. |

## Farm admin & integrity

| Command | What it does |
|---|---|
| `init` | Create a `.yaks/` farm in the current directory. |
| `skills install` | Install the bundled agent skills (`yaks`, `yaks-tracker`) into a skills dir (default `~/.agents/skills`). Upgrades a cleanly-outdated copy; leaves a locally-edited one alone unless `--force`. Refuses to write onto yaks' own `skills/` source (even via a symlink, even with `--force`). |
| `skills status` | Per-skill verdict — `current` / `stale` / `adoptable` / `held` / `modified` / `unmanaged` / `source` — from the provenance stamp. `--dir` to inspect another skills dir. See [skills.md](skills.md). |
| `doctor` | Read-only integrity check: duplicate-status ids, dangling parent/deps, malformed labels (a legacy label containing a comma or space, e.g. `ui,docs` — any label edit on that yak re-splits it). Exits non-zero on any issue (CI-usable). `--strict` also flags shorn yaks with no recorded note, and shorn yaks whose `verify:` command did not last PASS (evidence-before-shear). |
| `tui` | Open the interactive terminal UI (see [tui.md](tui.md)). |

## Attribution

`--as <actor>` on note-writing commands stamps the note `▸ <ts> [actor]`. The
actor resolves `--as` → `$YAKS_ACTOR` → git `user.name`. Committed status
transitions are already attributed by the git author. Attribution, never
ownership — a yak belongs to no one.
