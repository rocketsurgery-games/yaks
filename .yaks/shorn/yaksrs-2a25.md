---
id: yaksrs-2a25
title: 'TUI slice 5a: structured navigable detail pane + jumplist'
type: task
priority: 3
created: '2026-08-21T01:36:15Z'
updated: '2026-08-21T01:55:46Z'
parent: yaksrs-86a3
labels:
- rust
- phase2
---

Richer detail pane. Parse task-id references and URLs in the detail body/fields into a navigable link set; a jumplist lets you tab through them and Enter jumps (selecting that task / opening the URL). Detail-pane incremental search (/ within detail) to find text in a long body. Visual-yank: select a range in the detail view and copy it (clipboard via the clipboard primitive when available; otherwise a stub). Keep render pure. Snapshot detail with highlighted links + a jumplist state.

---
▸ 2026-08-21T01:49:29Z
Splitting slice 5. 5a: rebuild the detail pane as structured lines where parent, children (currently not shown at all), dependencies, and task-id references in the body become navigable links; a jumplist (Tab/Shift-Tab cycles link targets, Enter jumps to the referenced task — selecting it and switching to its tab). URLs detected + highlighted; opening deferred. 5b (new): detail-pane incremental search. Visual-yank needs the clipboard (deferred a49c) — folding it there rather than a standalone yak.

---
▸ 2026-08-21T01:55:46Z
Done. New tui/detail.rs builds a structured detail model: id/title/type/priority fields, parent (link), depends-on section (one link per dep, missing deps flagged), children section (NEW — was not shown before), source, and body lines with inline task-id refs ([[id]] wiki + bare, hand-rolled scan against known ids) and http(s) URLs detected as links. jumplist() flattens targets in reading order. App: detail_link cursor; in detail focus Tab/] and [/BackTab cycle links, Enter follows (task -> select_task jumps to it across tabs + focuses list; URL -> notify). render_detail rewritten to style link spans (blue underline; current = cyan) with no wrap so columns stay valid. 81 tests (was 75): 3 detail-model unit tests + 3 TUI nav tests + snapshot. Warning-free.
