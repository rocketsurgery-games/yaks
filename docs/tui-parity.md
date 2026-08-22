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

## 4. List row format — [done] (yaksrs-dd68); tree still [investigate] (#5)

- **Python:** ` {id}   p{pri} {type-word}   {glyph} {title}{…}{[labels]}` — id
  shown, spelled-out type (`task`/`bug`/`feature`/`idea`), glyph before the
  title, labels right-aligned. Blocked items get a magenta `*` prefix.
- **Rust:** `{indent}{chevron} [{glyph}] p{pri} {title}` (+ `★` when starred) —
  no id, no type word, no labels; tree chevron + indentation instead.
- Action [match]: adopt the row layout (id, type word, glyph, right-aligned
  labels, blocked `*`).
- [investigate]: the magenta `*` marks blocked/tangled tasks (fixture `fix-0004`
  has an unresolved dep on shaving `fix-0002`). Confirm the exact rule.

## 5. Tree / ghost family — [investigate]

- **Rust:** explicit tree — chevron `▾`, indentation; the shorn ghost `Gamma`
  is pulled in with its child `Delta` indented beneath it.
- **Python:** appears flat for this data (no chevron/indent), shorn `Gamma`
  shown as an ordinary row.
- Action: compare on a herd with clear hairy parent/child chains to see whether
  Python indents + shows chevrons and how it renders ghosts/collapse — Rust may
  be over- or under-showing family here.

## 6. Cursor / selection — [match]

- **Python:** cursor = reverse-video row (state picker confirms the cursor is on
  `fix-0001`); a **separate magenta `*`** marks blocked items.
- **Rust:** cursor = cyan/reverse highlight; no blocked marker.
- Action: add the blocked marker; verify selection styling parity.

## 7. Detail pane — [done] (yaksrs-425c)

`Task: {id}` header; blank; capitalized 13-wide `Title:/Status:/Type:/Priority:/
Created:/Updated:/Labels:` (humanized dates via `humanize_date`); then conditional
`Depends on:` / `Blocks:` (reverse deps) / `Parent:` / `Children:` sections.
Known nit: deps/children still use letter glyphs (H/N/S) vs the list's emoji
(yaksrs- follow-up).

- **Python:** `Task: {id}` header; blank; `Title:` `Status:` `Type:` `Priority:`
  `Created:` `Updated:` `Labels:` (labels ~13-wide, capitalized); blank; then
  `Depends on:` / `Blocks:` (reverse deps) / `Parent:` / `Children:` sections;
  humanized dates (`Dec 31, 2025 19:00`).
- **Rust:** `id` `title` `type` `priority` `labels` `depends on` `children`
  `source` `body` (labels 9-wide, lowercase); forward deps only; raw ISO; no
  dates shown.
- Action: header, capitalized padded labels, `Status`, humanized
  `Created`/`Updated`, `Blocks:` (reverse deps), `Parent:`/`Children:`.

## 8. Create (c / C) — [match]

- **Python:** full-screen modal **form** — `title` / `type` / `priority` /
  `labels` fields + a `description` section; `Tab`/`j`/`k` move between rows,
  `←→` pick chips, `Enter` edits a field; shows a `(need title)` hint; `Esc`
  cancels.
- **Rust:** sequential bottom prompts — a title line editor, then a `t/b/f/i`
  single-key type picker.
- Action: build the create form.

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

## 12. Remaining overlays to review — [investigate]

Not yet captured/compared: edit (`E`), labels (`L`), slaughter confirm (`X`),
inline search (`/`), fuzzy dep/reparent (`D`/`R`), view picker (`v`). Capture in
the next pass and extend this catalogue.
