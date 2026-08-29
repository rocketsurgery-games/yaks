---
id: yaks-22aa
title: 'TUI slice 4b: filter drawer (f) — status/type/priority/labels/search/parent/deps facets'
type: task
priority: 3
created: '2026-08-21T01:38:07Z'
updated: '2026-08-21T01:48:23Z'
parent: yaks-86a3
labels:
- rust
- phase2
---

The full multi-row filter drawer reproducing Python _DrawerState: chip rows (status, type, priority, deps ready/tangled) + text rows (labels, search, parent), row/chip navigation, live preview as you edit, Enter commits / Esc reverts to the pre-open spec. Builds on the filter plumbing from 4a. Snapshot the open drawer.

---
▸ 2026-08-21T01:48:23Z
Done. Overlay::Drawer(Drawer) reproduces Python _DrawerState: 7 rows — status/type/priority chips, labels/search/parent text (edtui single-line, RefCell), deps ready/tangled chips. Nav: Up/Down/Tab/Ctrl-N/P (and j/k on chip rows) move rows; Left/Right/h/l move the chip cursor; Space toggles; C clears all; Enter applies; Esc reverts to the pre-open spec. Every edit rebuilds App.filter via build_spec() for live preview (tree re-colors as you go). Rendered in the right pane with a ▸ current-row marker and a help hint on the status line. Since FilterSpec is not Clone, added a local clone_spec() for the revert snapshot. 75 tests (was 70): drawer render snapshot + toggle/apply/cancel/clear/text-row behavior. Warning-free. Completes slice 4.
