# yaks-158d design: global back/forward navigation

Design scout, 2026-09-23. Read-only study of `main` (7112fba) plus the in-flight
`lane-navpos` diff for yaks-28b4.

## 1. Where we are today

- `App.nav_back` / `nav_fwd` are two stacks, pushed **only** by `follow_link`
  (Enter on a yak link in the detail pane). `o` / `i` are bound **only** in
  detail focus. They are not reset when you enter the detail pane; they survive
  for the whole session.
- yaks-28b4 (lane-navpos, in flight) turns each stack entry into
  `NavEntry { id, line, scroll }`, snapshotting the entry as you *leave* it
  (`current_nav_entry`) and clamping on restore (`restore_nav_entry`). That
  "snapshot on departure, restore and clamp on return" model is the right one.
  158d keeps it and widens what gets snapshotted.
- **Root cause of the "i/o loses my list context" complaint.** Every jump goes
  through `open_task_in_detail` -> `select_task`, which calls `set_view(<the
  target's status view>)`. `set_view` resets the live filter to that view's
  saved spec and sets `cursor = 0`. Say you're in Recent, a custom view, or a
  searched/filtered list. Follow a link, then press `o`: you land in the yak's
  *status* view, with your filter/search gone and the list cursor re-derived.
  The list you came from is never recorded, so it can't be restored.
- **Latent bug (the same seam).** If the target has no pinned status view (e.g.
  a Dead yak; Inbox isn't a status view either), `select_task` doesn't switch
  views. The target also isn't in `rows()`, so `cursor` stays where it was and
  the detail pane shows the **previously selected yak** under a "-> id"
  notification. The detail pane is just `rows()[cursor]`, so it can't show a
  yak that isn't in the current list. `restore_nav_entry` in 28b4 guards the
  restore side of this. `follow_link` itself does not.
- The auto-reload (`reload_preserving_selection`, runs when the fs watcher marks
  the view dirty and no overlay is open) re-finds the cursor by id and never
  touches history. Entries hold ids, not indices, so reloads are harmless to
  history.

## 2. What a nav entry snapshots

Extend 28b4's `NavEntry` (the `id/line/scroll` part becomes `detail`):

```rust
struct NavEntry {
    list: ListCtx,
    focus: Focus,                 // List or Detail at departure
    detail: Option<DetailPos>,    // Some when focus == Detail
}
struct ListCtx {
    view_key: String,             // View::key (stable; index is not)
    filter: FilterSpec,           // the LIVE filter, incl. unsaved edits + search
    cursor_id: Option<String>,    // selected yak (authoritative)
    cursor_idx: usize,            // fallback when cursor_id is gone
    list_offset: usize,           // list viewport scroll (App.list_offset)
}
struct DetailPos { id: String, line: usize, scroll: u16 }  // == 28b4's NavEntry
```

Deliberately **not** snapshotted:
- `collapsed` and `family_scope`. They're persisted, farm-wide preferences, not
  navigation state. On restore, `expand_ancestors(cursor_id)` still runs so the
  row is visible.
- Visual selection, the multi-select `selected` set, overlays.
- `detail_find`. It's reset on every yak open today. If yaks-953f/1bbd make
  search context global, the search must stay *outside* the entry (see the Q3
  seam below).

## 3. What pushes (the "jump" rule)

Use vim's jumplist semantics: only *discontinuous* moves push. Continuous
movement (list `j/k/d/u/g/G`, detail line motions, detail `J/K`) doesn't push.
Nothing is lost that way, because the current entry is re-snapshotted when you
leave it.

| Action | Push? |
|---|---|
| Follow a yak link (detail Enter) | yes (today) |
| View switch: Tab / `[` `]` / view picker `v` / save-view `V` | yes (phase 2) |
| Programmatic jumps: `select_id` after create, dep/reparent picker "go to", any future goto-yak | yes (phase 3) |
| Enter/leave detail (`l`/Enter, `h`/Esc) | **no**. It updates the current entry's `focus`/`detail` in place |
| Search or drawer commit, Esc-revert filter | **no** (proposed; see Q1). The entry's `filter` is re-captured on departure |
| Auto-reload, `apply_edit` drop-out closing detail | never |

Mechanics:
- Push = snapshot the live state, push it to `back`, clear `fwd`. Skip the push
  if the new state has the same `(view_key, cursor_id, detail.id)` as the
  snapshot.
- Back/forward = snapshot the live state onto the opposite stack, then restore
  the popped entry.
- Cap `back` at about 100 entries and drop the oldest.

## 4. `i`/`o` and families (the human's note)

This is the "bonus points" case, and phase 1 fixes it outright. Take a family
chain parent -> child -> grandchild walked by links, starting in Recent with a
search. Each `o` step restores the *list* each hop was taken from, not just
the yak.

- Walking back out to the first hop puts you in Recent, with your search and
  your list cursor/offset where you left them.
- `h`/Esc from there drops you onto the list you actually came from.

Link-follow itself still needs *some* list that contains the target. The
current rule is "switch to its status view", and that's what makes the list
jump around. Proposal (Q2): **prefer the current list**.
- If the target is already in `rows()`, just move the cursor. No view switch,
  no filter reset.
- Otherwise fall back to the status view as today. That switch is now
  recoverable with `o`.
- Otherwise (no status view, e.g. dead), open the target's status view with a
  one-shot filter that includes its status. At minimum, refuse with a
  notification instead of showing the wrong yak. That fixes the latent bug.

A detached detail pane (showing a yak not in any list) would be the "pure" fix.
It's a much bigger change because `selected()` would stop being `rows()[cursor]`.
Not recommended now.

## 5. Restore semantics (moved or gone)

Restore in this order: view, filter, cursor, offset, focus, detail position.

1. **View**: look it up by `view_key`. If it was deleted, fall back to the
   yak's status view, else view 0, with a notification.
2. **Filter**: restore the snapshotted `FilterSpec` exactly. An unsaved edit
   shows the usual view-modified marker, and Esc reverts it as usual.
3. **Cursor**: find `cursor_id` in the rebuilt rows (`expand_ancestors` first).
   - Found: select it.
   - **Moved** (e.g. it went hairy -> shorn and the restored filter excludes
     it): for a list entry, clamp to `cursor_idx` and notify "`<id>` moved to
     shorn". For a detail entry, the yak matters more than the list, so fall
     back to the §4 find-a-list rule, and notify.
   - **Gone** (deleted/merged): clamp to `cursor_idx`, notify "`<id>` gone", and
     land on the list (focus List). Don't prune eagerly; only validate lazily at
     restore time. Optionally skip over a run of gone entries.
4. **Offset**: restore `list_offset`. The List widget re-clamps it to keep the
   cursor visible.
5. **Detail**: 28b4's clamp of `line`/`scroll` (content may have shrunk).

Renames: an id rename (herd move `H`) makes old entries "gone". That's
acceptable; mapping renames isn't worth it.

## 6. Key bindings

- Keep `o` = back and `i` = forward, and bind them in **list focus too**. Both
  are free there today. That's what makes the history global rather than a
  detail-pane feature. It also lets you back out of a view switch.
- Avoid Ctrl-O / Ctrl-I as the primary keys. Ctrl-I is Tab on terminals without
  the kitty protocol, and Tab already switches tabs/links. Ctrl-O could be an
  alias.
- Notification on each jump: `<- Recent . yaks-1234` (view name + id), so a
  cross-view hop is legible.
- Docs parity: `help_content` (both panes), `help_hint` (the list pane's bottom
  bar gains `o/i back/fwd`), `docs/tui.md`, and the `skills/yaks` TUI section
  if it lists keys.

## 7. Session persistence: recommend NO (for the stack)

- History is ephemeral intent, and ids go stale between sessions (agents
  shear/rename yaks while you're away). Restoring a stale stack at startup is
  more surprising than helpful, like a browser restoring back-history.
- It would also be the first history-shaped state in the rebuildable UI cache.
- **Cheap, and worth doing later:** persist only the **last position** (one
  `ListCtx` + focus) in `cache::UiState`, so `yaks tui` reopens where you quit.
  That's a separate, optional phase.
- A "recently viewed" list (the yak-597c deferral) can be derived from the
  in-session history without persisting it.

## 8. Phased plan (each phase shippable on its own)

- **P0 (in flight, yaks-28b4):** `NavEntry{id,line,scroll}` + clamp on restore.
  Land it first. P1 builds on it directly.
- **P1: the list context rides the stack.** Wrap 28b4's struct as
  `DetailPos`. Add `ListCtx` + `focus` to `NavEntry`.
  - `current_nav_entry` snapshots `view` key, filter, `selected_id`, cursor,
    `list_offset`.
  - `restore_nav_entry` restores the list context *instead of* going through
    `select_task`'s status-view switch.
  - Only link-follow pushes, as today.
  - This alone fixes the human's i/o complaint.
  - Test: in Recent with a search, follow two links, `o o`, then assert view +
    filter + cursor + frame.
- **P2: global keys + view switches push.** `o`/`i` in list focus. Tab/`[`/`]`,
  the view picker and save-view push. Help, bottom bar and docs updated.
- **P3: link-follow prefers the current list** (§4), plus the dead/unlisted
  fix (it never shows the wrong yak). Programmatic jumps (`select_id`, goto)
  push.
- **P4: robustness.** Moved/gone notifications, deleted-view fallback, cap,
  dedupe. Tests drive an external change through `reload_preserving_selection`
  between pushes.
- **P5 (optional):** persist last position only; a "Recently viewed" pseudo-view
  fed from the history (closes the 597c "looked at but didn't change" gap).

## 9. Interaction with global search (yaks-953f / 1bbd)

Here the `filter.search` field lives inside `ListCtx.filter`, so back restores
whatever search that list had. If 953f moves to a single global search context
(vim-style "last search"), `ListCtx` should snapshot the filter *minus* search,
and the global search stays live across jumps. Keep one function,
`snapshot_filter()`, as the seam so that decision is a one-line change.
