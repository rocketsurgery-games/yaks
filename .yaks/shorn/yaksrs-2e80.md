---
id: yaksrs-2e80
title: In vi mode, make single-line fields modal
type: task
priority: 3
created: '2026-08-26T12:51:14Z'
updated: '2026-08-26T13:47:49Z'
parent: yaksrs-fc85
depends_on:
- yaksrs-a031
labels:
- ui
---

Esc is canceling the entire yak edit by default, so if we *do* already support modal editing in single-line fields, it's getting lost in the process.
It would be nice if this were true by default, across the UI. This would require Ctrl-C / Double-Esc to cancel (vi only, not emacs), but I think that's ok.

---
▸ 2026-08-26T13:02:04Z
Fix site: in the edit form, handle_create_key treats Esc on any non-content row as 'cancel the whole form' (the Esc && !is_content branch). That overrides the first-Esc->Normal behavior single-line fields otherwise have. Needs the non-Esc cancel path (yaksrs-a031) first.

---
▸ 2026-08-26T13:47:49Z
Implemented. In vim, single-line fields are now fully modal: a lone Esc only drops to Normal and never cancels; cancel is Ctrl-C or a rapid double-Esc (yaksrs-a031). Edit overlay: esc_cancels simplified to single_line && Esc && !vim (dropped the 'second Esc in Normal cancels' path). Create/edit form: the lone-Esc-cancels branch is now emacs-only (Esc && !vim && !is_content), so in vim Esc drops title/labels fields to Normal and is a no-op on chip rows. Footer hint is vim-aware (EscEsc/Ctrl-C cancel). Scope: yak-edit surfaces (Edit overlay + create/edit form); the transient pickers (filter drawer, inline search, fuzzy) keep single-Esc-to-close since that's standard even for vim users. Emacs unchanged. Tests updated (create/edit cancel now via double-Esc; single_line_vim_reaches_normal_mode cancels via Ctrl-C); 2 form snapshots regenerated (footer only). 148 bin green, 0 warnings.
