---
id: yaks-f2aa
title: Pasting into description *very* slow
type: bug
priority: 1
created: '2026-08-30T19:05:01Z'
updated: '2026-09-23T21:43:54Z'
labels:
- ui
---

It works fine with the native paste affordance, but when you ctrl/cmd-v, it's excruciatingly slow, echoing out characters like a 300bps modem!

---
▸ 2026-09-23T21:35:37Z [coordinator]
Lightning-round lane 'paste' (worktree wt/paste, branch lane-paste). Scope: Cmd/Ctrl-V paste performance in editors (runtime event loop / bracketed paste / editor glue). Evidence contract: gate = cargo test -p yaks green (re-accept + eyeball any snapshot ripple); PLUS drive the headless TUI (toque) to the relevant state and yaks attach the frame (text, or docshot SVG when color matters). Judge: coordinator at merge review; subjective forks -> yaks ask and hand back.

![paste-evidence-f2aa](artifacts/yaks-f2aa/paste-evidence-f2aa.txt)

---
▸ 2026-09-23T21:43:26Z [lane-paste]
Before/after timing (release, TestBackend 160x50, 300 yaks, 1980-char paste) + headless frame of a bracketed paste into the edit-form description. Before = old loop (full redraw per key event, ~1-2.3 ms/char headless; far more on a real terminal with flush + emulator render). After = one frame per burst/paste (~1 ms total).

---
▸ 2026-09-23T21:43:54Z [lane-paste]
SHORN SUMMARY. Root cause: the event loop read ONE event per iteration and did a full frame render+flush after each, and bracketed paste was never enabled — so a terminal paste (Cmd-V / Ctrl-Shift-V) arrived as N key events = N full redraws (~1-2.3 ms/char headless in release; much more on a real terminal with the write+flush+emulator repaint, hence the 300bps echo). Key handling itself is ~20-50 ns/char; the redraw is 20000x that. (Note: Ctrl-V inside edtui's emacs profile is page-down, not a paste; the in-app paste is Ctrl-Y / vim p.) Fix: (1) runtime enables/disables bracketed paste; Event::Paste -> handle_paste bulk-inserts literally via new editor::paste_into (edtui InsertChar + SwitchMode(Insert) = one undo step, CRLF normalized, newlines->spaces in single-line fields) into Edit overlay / create-edit form rows; live-filter fields (search, detail find, fuzzy, drawer text rows) replay chars in Insert so their preview sync runs; a paste with no text field focused is dropped (it used to run as commands, and in vim Normal a paste ran as vim commands). (2) handlers::pump_events drains all already-queued events (50 ms budget, stops on quit) before the next redraw, so terminals without bracketed paste also get one frame per burst. Tests: 7 new (bulk paste, vim Normal literal, single-line flatten, search sync, paste-ignored-in-list, pump drain, pump stops on quit) + ignored paste_bench; cargo test -p yaks --release --bins 286 passed. Evidence: paste-evidence-f2aa.txt (before 2.0 s / after ~1 ms for 1980 chars + frame). Docs: docs/tui.md 'Pasting'. Upstream edtui note: EditorEventHandler::on_paste_event uses vim-p Paste (appends AFTER the cursor, wrong in Insert) and routes through state.clip.set_text (clobbers the system clipboard via arboard), so we bypass it; an upstream fix (insert at cursor in Insert, no clipboard write) would let us call it directly. PLEASE CONFIRM IN A REAL TERMINAL: Cmd-V a multi-line block into a description (vim Normal and Insert) — should land instantly in one go.
