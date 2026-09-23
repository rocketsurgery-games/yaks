---
id: yaks-97a2
title: Mouse support
type: feature
priority: 3
created: '2026-08-23T21:47:18Z'
updated: '2026-09-23T21:44:29Z'
labels:
- ui
needs: human
---

At least basic clicks; preferably scroll wheel. Bonus points for forward/back buttons.

---
▸ 2026-09-23T21:35:38Z [coordinator]
Lightning-round lane 'mouse' (worktree wt/mouse, branch lane-mouse). Scope: basic mouse: click-select, wheel scroll (runtime + handlers). Evidence contract: gate = cargo test -p yaks green (re-accept + eyeball any snapshot ripple); PLUS drive the headless TUI (toque) to the relevant state and yaks attach the frame (text, or docshot SVG when color matters). Judge: coordinator at merge review; subjective forks -> yaks ask and hand back.

---
▸ 2026-09-23T21:38:42Z [lane-mouse]
Plan: new src/tui/mouse.rs (impl App: handle_mouse + HitMap recorded at render in one RefCell field). EnableMouseCapture in runtime. Wheel: list moves selection, detail scrolls. Left-click: list row select (click selected row again = open detail like Enter), detail line cursor, tab switch. Finding: crossterm 0.29 parse_cb drops buttons 8/9 (back/forward) with 'We do not support other buttons' — not reachable without upstream change.

![mouse-click-wheel-frames](artifacts/yaks-97a2/mouse-click-wheel-frames.txt)

---
▸ 2026-09-23T21:43:26Z [lane-mouse]
Headless before/after frames (TestBackend + state header): left-click selects list row (cursor 0->1, sel a0->a1); 12x wheel-down over list moves selection t00->t12 with scroll; 2x wheel-down over detail scrolls 0->6 (sticky header appears). Plain text hides the selection bg, hence the state lines. Human should sanity-check in a real terminal (mouse capture, Shift-drag selection).

---
▸ 2026-09-23T21:43:26Z [lane-mouse]
Done. src/tui/mouse.rs: HitMap (list rect+offset, detail content rect, tab rects) recorded by render into one App field hits: RefCell<HitMap>, reset per frame; App::handle_mouse. runtime: EnableMouseCapture/DisableMouseCapture + Event::Mouse dispatch. Behavior: wheel over list moves selection by 1 (with detail open: steps tasks like J/K, detail stays); wheel over detail scrolls 3 lines, dragging the line cursor into view; click row selects+focuses list, click selected row again opens detail; click detail line moves cursor, click again follows link; click tab switches view; ignored while any overlay is open. Back/forward buttons: NOT possible — crossterm 0.29 parse_cb returns an error for buttons 8/9. Docs: docs/tui.md Mouse para (incl. Shift/Option-drag for native selection), README, help_content Mouse section. Tests: 6 in tui::mouse::tests; cargo test -p yaks --bins 285 passed. Open question for human: click-a-list-row while the detail is open closes the detail (spec: focus the list) — alternative is to keep detail open and show the clicked task; and whether capture should be toggleable.

---
▸ 2026-09-23T21:44:29Z [coordinator]
Shipped (merged to main). Two follow-up calls for you, plus please try it in a real terminal: (1) Clicking a list row while the detail is open currently focuses the list and closes the detail. Alternative: keep the detail open and show the clicked yak (my lean, since it matches J/K in detail). (2) Mouse capture blocks native text selection (Shift/Option-drag still works). Add a toggle key/config? My lean: not yet, unless it bites you. Also: back/forward mouse buttons aren't possible, because crossterm 0.29 drops buttons 8/9.
