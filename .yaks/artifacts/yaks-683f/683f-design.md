# yaks-683f design: draft preservation for edit surfaces

Scout: design-scout, 2026-09-23. Builds on yaks-275f (shorn), which added a
`Discard changes? (y/N)` confirm on dirty cancel (`App::request_cancel`,
`App.dirty_cancel`, `ConfirmAction::DiscardEdit`, `form_is_dirty`, in
`src/tui/handlers.rs`). 275f protects against a *mis-keyed* cancel. It does not
cover a cancel you meant to make ("I'll finish this later"), a quit, a killed
terminal or ssh drop, or a panic. That gap is what 683f is for.

## 1. Surfaces

| Surface | Overlay / action | Draft? | Why |
|---|---|---|---|
| Create form (`c`/`C`) | `Overlay::Create`, `edit_id: None` | **yes (P1)** | Most typing, and nothing exists on disk yet. |
| Edit form (`E`) | `Overlay::Create`, `edit_id: Some(id)` | **yes (P1)** | Long description rewrites. |
| Comment (`M`) | `Overlay::Edit(EditAction::Comment)`, multiline | **yes (P1)** | Multi-line prose. Already 275f-guarded. |
| Ask / Answer | `EditAction::Ask/Answer`, single-line | P2 | One line and cheap to retype. Not 275f-guarded either. Add both together if wanted. |
| Attach / labels / rename / save-view | single-line | **no** | Trivial input. Drafts would just be clutter. |

Rule: a surface gets a draft exactly when it gets the 275f dirty-confirm. Both
features then share one definition of "work worth protecting".

## 2. Keying

- **Edit form:** `edit:<id>`. **Comment:** `comment:<id>`. (P2: `ask:<id>`,
  `answer:<id>`.) One draft per (surface, id). A newer draft replaces the older one.
- **Create:** `create:<parent-id or "root">`. A create draft has no id yet, so it
  is keyed by where it would land. `c` under yak X offers X's pending create
  draft. `c` at root offers the root one. Only one create draft per parent
  (no list of drafts in P1).
- **Id renames** (herd move, yaks-71d1) orphan a draft. That's acceptable: the
  orphan is still listed in the P3 drafts picker, and the GC below removes it.

## 3. Storage

- **Location:** per-user and per-farm, next to `views.json`:
  `$XDG_CONFIG_HOME/yaks/<slug>/drafts/<key>.json`. Reuse
  `views_store::config_dir` (make it `pub(crate)`). Never under `.yaks/`,
  because files there are authoritative and committed. Config home rather than
  cache home: a draft is user intent, which is the same reasoning views_store uses.
- **One file per draft.** Writes are atomic (write `*.tmp`, then `rename`).
  Best-effort like views_store: no panics, errors are ignored or shown in a
  notification.
- **Payload:** `{version, key, surface, id?, parent?, saved_at, base: {updated,
  body_hash}, fields: {title, kind, priority, labels, herd, blocks:[{kind,
  text}]}}` for forms, or `{..., text}` for a comment. `base` records what the
  draft was forked from, which the staleness check needs.
- **When to save (crash safety):**
  - P1: save **on dismiss**, in `request_cancel` when dirty, and on app quit while a
    dirty overlay is open (`q`/`:q` paths plus a `Drop`/panic-hook flush).
  - P2: add **periodic autosave**, piggybacked on the existing 250 ms
    `event::poll` loop in `runtime.rs`. Save when the overlay is dirty, its
    content hash changed, and at least 2 s have passed since the last save. That
    covers SIGKILL and a closed terminal, which on-dismiss saving cannot.
- **Clear:** delete the draft on a successful commit (`commit_form` /
  `commit_edit`), on an explicit "discard", and on `:q!`.
- **GC:** at startup, drop drafts older than 30 days and drafts whose id no
  longer exists or is `dead`. Also expose a `yaks drafts --prune` later if needed.

## 4. Restore UX

- **Auto-restore, no prompt.** Opening `E`/`M`/`c` on a key that has a draft opens
  the form **pre-filled from the draft**. A header badge reads
  `draft · saved 2h ago`, and the bottom line gets a notification:
  `restored draft — Ctrl-R for original`. Rationale: a prompt before every open
  is friction, and a restored draft is one keystroke away from being reverted.
- **Revert:** `Ctrl-R` inside the form (or `:e!` in vim, matching vim's
  "revert buffer") discards the draft and re-seeds from the current yak.
- **Visible indicator elsewhere:** in the tree row and the detail header, a small
  `✎` marker on yaks that have a pending draft, so drafts don't go stale unseen.
  P3: add a `Drafts` view or picker (reusing `Overlay::Fuzzy`) that lists all
  drafts, including orphaned create drafts.

## 5. Composition with 275f confirm-on-cancel

Replace the current `Discard changes? (y/N)` with a three-way pick on dirty
cancel:

```
Unsaved changes: k=keep draft  d=discard  Esc=back to editing
```

- `k`, or Enter as the **default**, saves the draft, closes the form, and shows
  `draft saved`.
- `d` deletes any draft, closes the form, and shows `changes discarded`.
- `Esc`/`n` restores the stashed overlay, exactly as a 275f decline does today.
- `:q!` still force-discards with no prompt.

This keeps 275f's "no silent loss" guarantee while making the safe choice the
default. Implement it as a new `PickAction::DirtyCancel`, or extend `ConfirmAction`
with keys. `dirty_cancel` stash semantics are unchanged. An alternative (see Q1) is
to drop the prompt and always keep the draft silently, the way a browser does.

## 6. Stale drafts (the yak changed underneath)

The watcher already defers reloads while an overlay is open, so staleness only
matters **at restore**. The case where a yak changes while the editor is open and
the commit clobbers it is a separate problem (see Q4).

- On restore, compare `base.updated` and `base.body_hash` to the current yak.
- If they match, restore silently.
- If only fields the draft didn't touch changed (for example, the status moved),
  restore silently.
- If the base changed, still restore the draft, but show the badge in warning
  colour: `draft (yak changed since) — Ctrl-R original · Ctrl-D diff`. P1 ships
  just the warning plus Ctrl-R. The diff or 3-way merge comes later or never.
- Comment drafts are append-only, so they are never stale in a meaningful way. Restore them as-is.
- If the yak is missing or dead, don't restore. Leave the draft for the drafts picker or GC.

## 7. Phased plan

**Phase 1 (small, shippable, roughly one PR):**
1. `src/tui/drafts_store.rs`: `save(root, key, &Draft)`, `load`, `delete`,
   `list`. JSON, atomic rename, reusing `views_store::config_dir`.
2. `Draft::from_form(&CreateForm)` / `CreateForm::apply_draft`, plus a comment variant.
3. Dirty cancel becomes the keep/discard/back pick, with keep as the default.
   Quit with a dirty overlay saves the draft.
4. Opening `E`/`M`/`c` auto-restores, with a header badge and notification.
   `Ctrl-R` reverts. The draft is deleted on commit.
5. Staleness: an `updated` mismatch produces the warning badge only.
6. Docs parity: `docs/tui.md`, `help_content` (Ctrl-R, the keep/discard pick),
   and `skills/yaks` if it mentions cancel. Tests: headless toque for dirty
   cancel → k → reopen → restored, plus snapshot of the badge. Use
   `XDG_CONFIG_HOME` pointed at a tempdir, like the views_store tests.

**Phase 2:** periodic autosave (crash and kill safety) and a panic-hook flush.
Ask/Answer drafts, which also get the 275f guard.

**Phase 3:** `✎` markers in the tree and detail pane, a Drafts picker/view,
startup GC, and optional diff view for stale drafts. A CLI `yaks drafts` only if
there's demand; drafts are a TUI concern.

## Open questions (also posted via `yaks ask`)

1. Keep a prompt on dirty cancel (keep/discard/back, keep default), or always
   keep silently and drop the 275f prompt? *Lean: keep the 3-way prompt with keep
   as default.* It's explicit and it preserves 275f.
2. Restore automatically with a badge and Ctrl-R, or ask "restore draft?" first?
   *Lean: auto-restore.*
3. Phase 1 saves only on dismiss and quit, or also autosaves periodically from the
   start? *Lean: dismiss/quit in P1, autosave in P2.* The 250 ms loop makes P2 cheap.
4. Is a mid-edit external change (the commit clobbers a concurrent edit) in scope
   here, or a separate yak? *Lean: separate yak.* 683f only warns at restore.
