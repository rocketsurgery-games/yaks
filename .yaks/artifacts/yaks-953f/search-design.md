# Design: global search context (yaks-953f, folds in yaks-1bbd)

Design scout, 2026-09-23. Read-only study of `src/tui/{handlers,viewmodel,detail_nav,render,drawer}.rs`, `src/tui.rs`, `src/filter.rs`.

## Today (what the code actually does)

- **List search** (`/` in list, `open_search`, handlers.rs ~916): edits `App.filter.search` live; the query lives *inside* the view's live `FilterSpec`.
- **View switch** (`set_view`, viewmodel.rs:239): `self.filter = clone_spec(&views[i].spec)`, which **wipes** the search (and every other ad-hoc facet).
- **Header** (`render_tabs`, render.rs:400): yellow `filter: “q”` shown iff `is_view_modified()`. This header is always correct.
- **Detail find** (`/` in detail, `open_detail_find`, detail_nav.rs:182): a *separate* `App.detail_find: Option<String>` with its own `n`/`N` (`detail_find_jump`). It is reset to `None` by `open_task_in_detail` (link follow, i/o) and `detail_next_task` (J/K). It never sees the list query.
- **List `n`/`N`**: currently unbound (free).
- **Saved views** can carry their own `spec.search` (the drawer's `search` row; `actions.rs:41` saves `self.filter`).

## Root cause of the 953f desync (reproduced)

On search commit (Enter) the handler also sets `self.notification = Some("filter: q")` (handlers.rs:932-936).
**`App.notification` is never cleared anywhere in the codebase**. It only gets overwritten by the next message.
So after `/widget⏎ Tab` the real filter is gone (`filter=(all)`, header gone), but the stale
notification `filter: widget` stays in the tab row's right-aligned slot and *looks like* the filter header.

Headless repro (scratch farm with `alpha widget`, `beta gadget` hairy, `gamma widget` shaving):
`printf 'key /\ntype widget\nkey Enter\nkey Tab\nkey Tab\n' | yaks tui --headless --size 100x10`

```
frame 3 (after Enter), view=Hairy filter=“widget”:
 🦬  Hairy* (2)   🪒  Shaving (1) ...   filter: “widget”     <- real header (left, yellow)
filter: widget                                                  <- notification (dropped to gap row)
frame 4 (after Tab), view=Shaving filter=(all):
 🦬  Hairy (2)   🪒  Shaving (1) ...          filter: widget   <- STALE notification; search is gone
frame 5 (after Tab), view=Shorn filter=(all):  same stale "filter: widget"
```

So "header persists, search doesn't" is really "a never-expiring toast mimics the header." That is a
general notification-lifetime bug too: *every* toast (`→ id`, `t0 → shorn`, `copied …`) is sticky.

## Proposal

### 1. Single source of truth

Split the two concepts that `filter.search` currently conflates:

| state | lives on | owner | survives view switch? |
|---|---|---|---|
| view facets (types/prio/labels/ready/… **and a saved view's own `search`**) | `App.filter: FilterSpec` (unchanged) | the view; drawer edits it; `v`-save persists it | no (reset from spec, as today) |
| **active query** (what `/` types) | new `App.query: Option<String>` | the user, globally | **yes** (lean) |
| **search register** (last committed pattern, vim `@/`) | new `App.last_search: Option<String>` | written by any `/` commit (list or detail) | yes, whole session |

- `rows()` / `tree::build` apply `filter` **AND** `query` (factor the substring test out of
  `FilterSpec::matches` in `src/filter.rs` into `filter::text_matches(t, q)` and apply it as an extra
  predicate; no on-disk/format change, `FilterSpec` stays the saved-view contract).
- A saved view that has its own `spec.search` simply ANDs with the global query. The drawer's
  `search` row keeps editing the *view's* search (it is what gets saved); `/` never writes into the spec,
  so `/` no longer marks the view `*`-modified.
- `detail_find` is deleted; detail highlighting/jumping reads `last_search` (see 3).

### 2. Header and view switches

- Header: render the query as its own chip, e.g. `/“widget”` (distinct color from the facet `filter:`
  chip), derived *only* from `App.query`. The `filter:` chip keeps meaning "facets differ from the
  saved view". Both are pure functions of state, so they cannot desync.
- `set_view` leaves `query` untouched (lean: **persist**, it is the "global search context" the yak asks for).
  Counts in the tabs (`view_count`) should apply the query too so `Shaving (1)` means "1 match".
- Esc in the list is two-stage: first clears `query` (register keeps it, so `n` can bring it back),
  second reverts facets to the view (today's `revert_filter_to_view`).
- Drop the redundant `filter: q` toast on commit; the chip is the feedback.

### 3. Detail view

- On entering detail (l/Enter, link follow, i/o, J/K), if `last_search` is set, matches are
  **highlighted** (the existing `detail_scan` + render.rs:772 highlight path) and the match index resets to 0.
  Stop nulling the pattern on task change (only reset `detail_match`, scroll, line).
- Optional (lean yes): on *entering* detail from a filtered list, auto-scroll to the first match
  (vim `/` then open: you land on the hit). J/K/i/o keep scroll at top; `n` jumps.
- `/` in detail edits the register (`last_search`), **not** the list query, so searching for a word
  that only appears in a child title / note can't make the selected yak vanish from the list under you.
  Esc during detail `/` restores the prior register + origin (today's behavior).
- Detail footer/status shows `match 2/5 “widget”` when a pattern is active.

### 4. `n` / `N` semantics

| where | query/register state | `n` / `N` |
|---|---|---|
| detail | register set | next/prev match in this yak (wrap), as today but using the register |
| detail | register set, no match in this yak | (lean) jump to next/prev yak in the list that matches, landing on its first match — "continue searching in the details" (1bbd) |
| list | `query` active | move cursor to next/prev row (all rows match; cheap consistency) — or leave unbound (question) |
| list | `query` empty, register set | **re-apply the register as the query** ("pull the last search back in", vim-like); `N` same |
| anywhere | nothing ever searched | notification `no previous search` |

### 5. Relationship to yaks-158d (global nav history)

A 158d history entry should snapshot `{view, filter, query, cursor/selected id, focus, detail pos}`;
`last_search` is *not* part of history (it is a register, like vim). Keeping `query` separate from the
view spec makes that snapshot trivial. No dependency either way; do 953f phases 1-2 first.

## Phased plan

**Phase 1: fix the desync only (small, shippable, no UX decisions beyond Q1).**
- Make notifications transient: clear `self.notification` at the top of `handle_key` (before dispatch),
  so a toast lives until the next keystroke. (Alternative, narrower: clear it only in `set_view`; see Q1.)
- Remove the `filter: {q}` toast on search commit (header already shows it; keep `search cleared`).
- Regression test in `src/tui/tests.rs`: `/widget⏎ Tab`, assert tab-row text contains no `filter:` and
  `app.filter.search` is None; plus insta snapshot. Audit tests that assert a notification after an
  extra key (tests.rs asserts `app.notification` right after the action, so most should hold).
- Docs: none user-facing beyond maybe `docs/tui.md` wording about notifications.

**Phase 2: global query.** Add `App.query` + `last_search`; move `/` off `filter.search`; query chip;
persist across `set_view`; tab counts apply query; two-stage Esc; list `n`/`N` re-apply. Update
`help_content`, `help_hint`, `docs/tui.md`, docshots scene for the chip.

**Phase 3: detail propagation (yaks-1bbd).** Delete `detail_find`, drive highlight/`n`/`N` from
`last_search`; stop resetting on J/K/i/o; match counter; cross-yak `n` if Q3 says yes. Help + docs.

## Open questions (asked on the yak)

1. Phase 1 scope: clear toasts on every keypress globally (lean) vs only on view switch?
2. Should the `/` query persist across view switches (lean: yes, as a separate chip; Esc clears it) or be cleared (today's behavior, just fixing the stale header)?
3. Detail `/`: edits only the search register (lean) vs also re-filters the list? And should `n` past the last match in a yak roll on to the next matching yak (lean: yes)?
4. List `n`/`N` when a query is active: move to next row (lean) or leave unbound, keeping `n` only as "re-apply last search"?
