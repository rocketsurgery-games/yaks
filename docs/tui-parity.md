# TUI parity with the Python original

Goal: make the Rust `yaks tui` match the Python (curses) TUI closely, *then*
choose where to intentionally evolve. This is the running catalogue of concrete
differences found by differential testing.

## Method

Same stdin script + same herd, driven through both implementations:

- Rust:   `yaks tui --headless --size WxH [--style]`
- Python: `uv run tools/py_tui_capture.py --yak <yaks>/scripts/yak.py --herd <herd> --size WxH [--style] --launch <python-with-pyyaml>`

Both emit the same framed grid (`=== frame N · WxH · … ===`), so frames diff
directly. `--style` adds an aligned base36 style grid + legend that makes
attribute-encoded state (selection, focus, blocked markers, links) visible.

Gotchas: run at ≥100 columns (Python's emoji tab bar wraps at 72); pyte renders
the curses ACS vertical divider as `x` (DEC line-drawing not mapped — a capture
artifact, not a real difference); mutation flows must run on a scratch copy of
the herd. Baseline captures below use `tests/fixtures/herd` at 100 wide with
isolated `XDG_CONFIG_HOME`/`XDG_CACHE_HOME`.

Tags: **[match]** = change Rust to match Python · **[investigate]** = need more
data · **[keep]** = already matching / intentional.

---

## 1. Overall layout & panes — [done] (yaksrs-2b56)

List is full-width in list focus; `l` splits into list(1/3)+detail(2/3); blank
gap row under the tabs. (Right-pane overlays still borrow a split until they're
relocated to their Python homes — tracked with those overlays.)

- **Python:** list mode is a **full-width list**; pressing `l` enters a detail
  mode that splits into list (left) + detail (right).
- **Rust:** **always** two-pane (list + detail), even in list focus.
- Action: give the list the full width when `focus=list`; split only in detail
  focus.

## 2. View / tab bar — [done] (yaksrs-7103)

- **Python:** ` 🦬 Hairy (3)  🪒 Shaving (1)  🐑 Shorn (1)  🕒 Recent (5)  ⭐ Starred (0)` —
  per-view emoji, `(N)` count in parens, two-space separators, active tab drawn
  black-on-white bold, followed by a **blank separator line**.
- **Rust:** `Hairy 3 · Shaving 1 · Shorn 1 · Recent 5 · Starred 0` — no emoji,
  count after the name, `·` separators, cyan-bold active, no separator line.
- Action: emoji + `(N)` + two-space separators + blank separator line; match the
  active-tab highlight.

## 3. Status glyphs — [done] (yaksrs-7103)

- **Python:** 🦬 hairy · 🪒 shaving · 🐑 shorn · (💀 dead).
- **Rust:** `[H]` `[S]` `[N]` `[X]`.
- Action: adopt the emoji glyphs. Note width-2 emoji have real layout cost
  (drives the 72-col tab-bar wrap).

## 4. List row format — [done] (yaksrs-dd68); tree [done] (#5)

- **Python:** ` {id}   p{pri} {type-word}   {glyph} {title}{…}{[labels]}` — id
  shown, spelled-out type (`task`/`bug`/`feature`/`idea`), glyph before the
  title, labels right-aligned. Blocked items get a magenta `*` prefix.
- **Rust:** `{indent}{chevron} [{glyph}] p{pri} {title}` (+ `★` when starred) —
  no id, no type word, no labels; tree chevron + indentation instead.
- Action [match]: adopt the row layout (id, type word, glyph, right-aligned
  labels, blocked `*`).
- [investigate]: the magenta `*` marks blocked/tangled tasks (fixture `fix-0004`
  has an unresolved dep on shaving `fix-0002`). Confirm the exact rule.

## 5. Tree / ghost family — [done: verified at parity] (yaksrs-0807)

Investigated head-to-head on a purpose-built fixture (clear hairy parent/child
chains + a shorn ghost ancestor + a shaving ghost descendant), driving both TUIs
through the shared headless protocol. **They match.** Both `tree.rs::build` and
Python `tree.build_tree` implement the identical algorithm (universe = anchors +
ancestors + descendants; `ghost = not in focus`; shaving-first child ordering),
and render identically:

```
 tf-alpha p2 task     🦬 Alpha root
   tf-ab  p3 task     🪒 Alpha child B (shaving)   ← ghost (shaving descendant)
   tf-aa  p3 task     🦬 Alpha child A
 tf-gamma p2 task     🐑 Gamma root (shorn)        ← ghost (shorn ancestor)
   tf-ga  p3 task     🦬 Gamma child (hairy)
```

Same rooting, same 2-space-per-depth indentation, same ghost pulls (shorn
ancestor `gamma` and shaving descendant `ab`), same child order, on both the
Hairy and Shorn tabs. The original "Python looked flat" observation did not
reproduce — it was against data whose parent/child links weren't established.

Chevrons appear only on **collapsed** parents (as `▶ N`); Rust collapse hides
the subtree and shows `▶ N` correctly. Python's collapse could not be verified
through the pyte capture harness (a redraw/timing artifact leaves the children
on screen next to the chevron and then drops the chevron on the next redraw),
but Python's `apply_collapse` implements the same hide+count logic. No Rust
change needed.

Known non-divergence: the one-space difference after the status emoji in the two
captures (`🦬  Alpha` vs `🦬 Alpha`) is the wide-emoji continuation-cell capture
artifact (Rust `TestBackend` grid vs pyte), not a real rendering difference.

## 6. Cursor / selection — [match]

- **Python:** cursor = reverse-video row (state picker confirms the cursor is on
  `fix-0001`); a **separate magenta `*`** marks blocked items.
- **Rust:** cursor = cyan/reverse highlight; no blocked marker.
- Action: add the blocked marker; verify selection styling parity.

## 7. Detail pane — [done] (yaksrs-425c)

`Task: {id}` header; blank; capitalized 13-wide `Title:/Status:/Type:/Priority:/
Created:/Updated:/Labels:` (humanized dates via `humanize_date`); then conditional
`Depends on:` / `Blocks:` (reverse deps) / `Parent:` / `Children:` sections.
The deps/blocks/parent/children ref lines use the same status **emoji**
(🦬/🪒/🐑/💀, via `Status::emoji()`) as the list + tab bar (yaksrs-f2cf); link
offsets stay char-indexed and `render_dline` styles them as relative span flow,
so the width-2 emoji doesn't skew the id highlight.

- **Python:** `Task: {id}` header; blank; `Title:` `Status:` `Type:` `Priority:`
  `Created:` `Updated:` `Labels:` (labels ~13-wide, capitalized); blank; then
  `Depends on:` / `Blocks:` (reverse deps) / `Parent:` / `Children:` sections;
  humanized dates (`Dec 31, 2025 19:00`).
- **Rust:** `id` `title` `type` `priority` `labels` `depends on` `children`
  `source` `body` (labels 9-wide, lowercase); forward deps only; raw ISO; no
  dates shown.
- Action: header, capitalized padded labels, `Status`, humanized
  `Created`/`Updated`, `Blocks:` (reverse deps), `Parent:`/`Children:`.

## 8. Create (c / C) — [done: right-pane form] (yaksrs-d13b)

Done. Rust now has a real create **form** (`Overlay::Create`), modeled on the
filter drawer and placed in the **right pane** (same intentional divergence as
the drawer, §9 — a wide/short terminal has columns to spare, not rows). It
shares the drawer's `right_divider` rule and the generalized
`render_chip_row`/`render_text_row` helpers.

- **Python:** full-screen modal **form** — `title` / `type` / `priority` /
  `labels` fields + a `description` section; `Tab`/`j`/`k` move between rows,
  `←→` pick chips, `Enter` edits a field; shows a `(need title)` hint; `Esc`
  cancels.
- **Rust:** right-pane form — rows `title` / `type` / `priority` / `labels` /
  `description`; `Tab`/`↑↓` (and `j`/`k` on chip rows) move rows; `←→` (and
  `h`/`l`) pick chips as **single-select** (the cursor *is* the value, unlike
  the drawer's Space-toggle multi-select); text rows edit in place. `Enter`
  **creates** (guarded on a non-empty title — the status hint flips between
  `(need title)` and `Enter create`); `Esc` cancels.
- **Intentional divergences:** right-pane placement (matches §9); `Enter` =
  create rather than Python's per-field `Enter:edit` submit gesture (which is
  ambiguous in a single-modal form); header reads `New yak` / `New task (child
  of …)` (semantic, not byte-parity). Type/priority/labels/description all feed
  `NewTask` (priority defaults to p3).

## 9. Filter drawer (f) — [divergence: right-side drawer + delimiter] (yaksrs-d416)

Intentional divergence from Python: the filter drawer (and the fuzzy/view
pickers + multiline editor) stay in the RIGHT pane rather than a top drawer,
because a wide/short terminal has rows to spare but a top drawer steals them.
Every right-pane surface now shares the detail pane's left-divider rule (via
`right_divider`) so it no longer blends into the list. Original Python note kept
for reference:

- **Python:** a **top drawer above the list** (list stays visible below);
  `[x]`/`[ ]` **checkbox** chips for status/type/priority/deps; `labels`/`search`
  /`parent` text rows; footer `Enter:commit  Esc:revert  C:clear`.
- **Rust:** drawer in the **right (detail) pane**; `▸` + green-highlight chips;
  footer hint on the status line.
- Action: move to a top drawer with checkbox chips.

## 10. Single-key pickers (S / P / T) — [keep]

- Python state picker: `State for {id}: h=hairy s=shaving n=shorn x=slaughter
  (Esc=cancel)` — **identical** to Rust. Priority/type pickers very likely match
  too (verify).

## 11. Help lines — [done] (yaksrs-1472); dynamic f:filter/Esc:clear hint matched

- Per-state help text differs in both wording and the key list, e.g.
  - Python list: `Tab:view  j/k:move  l:detail  v:views  c/C:new  E:edit  X:del  S:state  D:dep  f:filter  Esc:clear`
  - Rust list:   `j/k · c new · E edit · S/P/T/L/X · D/R · / find · f filter · * star · V save · Tab view · q`
- Action: adopt Python's per-state help strings.

## 12. Remaining overlays — [reviewed] (yaksrs-a083)

Captured head-to-head on the fixture herd. Summary:

- **Labels (`L`) — [match].** Both a bottom-line single-line editor
  (`Labels for {id}:`). Python shows a trailing `[I]` vim-mode indicator that
  edtui doesn't echo; cosmetic.
- **Inline search (`/`) — [match].** Both open a live-filtering inline search
  (Python `_open_inline_search`; the tab gains the `*` modified marker as you
  type). Rust additionally echoes the query on the status line (`/…`); Python
  reverts the status line to the help hint. Harmless enhancement.
- **Slaughter confirm (`X`) — [match, reworded].** Same shape
  `{verb} {id} ({title})? (y/N):`. Rust says **Slaughter** (on-brand yak
  vocabulary) where Python says *Delete*. Intentional wording keep.
- **Fuzzy dep/reparent (`D`/`R`) — [match; glyphs fixed].** Candidate rows now
  use the status **emoji** (🦬/🪒/🐑/💀) like the list/detail (was `[H]`/`[S]`/
  `[N]` bracketed letters — fixed here for consistency with §7/f2cf). Placement
  is the intentional right-pane divergence (§9). `R` differs in gesture: Python
  first prompts `p=pick parent, u=unparent`, then opens the picker; Rust opens
  the picker directly and offers a synthetic **(clear parent)** row when the
  task has a parent — fewer steps, same outcome. Acceptable divergence.
- **View picker (`v`) — [match; glyphs fixed].** Rows now use 📌 (pinned) and 🔒
  (builtin) like Python (was `*` / `(builtin)` text). Right-pane placement +
  status-line help hint are the intentional divergence (§9).
- **Edit (`E`) — [divergence: body-only vs full form].** Python's `E` reopens
  the **whole task form** (title/type/priority/labels + description), i.e. the
  create form in edit mode. Rust's `E` edits only the **description body**
  (other fields go through `L`/`P`/`T`/`S`). Now that the create form exists
  (d13b), the natural fix is a shared create/edit form; filed as a follow-up
  (see §8's form + the new edit-form yak). Not changed under a083.
