---
id: yaksrs-efa9
title: Single-line editors don't have a vi mode
type: bug
priority: 2
created: '2026-08-23T05:06:54Z'
updated: '2026-08-28T04:48:16Z'
---

I think we're using edtui for both, but [esc] takes you straight out of the editor altogether. It should be at least _optionally_ possible to use vi controls in single-line fields.

---
▸ 2026-08-28T04:48:16Z
Implemented. Single-line editors are now modal in vim across all overlays, matching the Edit-overlay pattern: a lone Esc drops the field to Normal (edtui) so motions (h/l/w/b/0/$/x/i/a...) are reachable, and Ctrl-C or a rapid double-Esc cancels. Covered: inline search (/), detail find, the fuzzy dep/reparent picker, and the filter drawer text rows (labels/search/parent); the create/edit form title/labels already worked. Emacs mode unchanged (lone Esc cancels). Added a shared App::field_cancel helper; the drawer still closes on a lone Esc on chip rows. Drawer footer hint is now vim-aware. Tests: search + detail-find cancel switched to Ctrl-C, plus new Esc-enters-Normal coverage for search, detail-find, fuzzy, and drawer.
