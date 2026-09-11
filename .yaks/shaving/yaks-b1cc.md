---
id: yaks-b1cc
title: Split src/tui.rs monolith into a render/ module tree
type: task
priority: 3
created: '2026-09-10T23:45:35Z'
updated: '2026-09-11T01:39:46Z'
labels:
- meta
- ui
needs: human
---

src/tui.rs is ~7K lines / 270KB, funnelling nearly all UI work through one file and defeating file-disjoint parallel scoping (see the coordinating-skill yak). Splitting render_* and the handle_*_key handlers into a render/ (and handlers/) module tree would give future parallel batches real file-level disjointness. Corollary of dogfooding the UI batch.

---
▸ 2026-09-11T01:29:48Z [coordinator]
RESEARCH & PLAN. tui.rs = 7043 lines. MAP: (1) imports + mod decls (1-57); (2) shared enums Focus/Overlay/PickAction/ConfirmAction (58-115); (3) editor/edtui glue: Editor + EditAction/EditTarget + make_handler/insert_into/delete_into/edtui_can_handle/token_before_cursor/completion_context + impl Editor + render helpers mode_tag/mode_style/set_md_highlights/editor_theme (116-267, 4007-4062); (4) overlay/form types: FuzzyPick/FuzzyAction/SearchBox (270-311); Drawer + consts + text_field/multiline_field/toggle + impl Drawer (313-484); ContentBlock/CreateForm + kind_index/pri_index + impl CreateForm (494-660); impl SearchBox/FuzzyPick + fuzzy_candidates/fuzzy_total (692-781); (5) App struct (782-847) + impl App = 109 methods ~2400 lines (848-3215) THE monster; (6) runtime run/event_loop/setup_watcher (3216-3295) + setup/restore (4975-5005); (7) free fn handle_key (3296); (8) render layer render + render_tabs/list/detail/status/view_picker/help + ~20 helpers (3463-4974, ~1500 lines); (9) #[cfg(test)] mod tests (5006-7043, ~2037 lines) incl shared helpers draw/sample/editable/linked + a 'live' harness (temp_herd/press). SNAPSHOTS: src/snapshots/ = 22 insta files ALL tui unit-test snaps (yaks__tui__tests__*). tests/snapshots/ = CLI integration snaps; only cli__headless_session.snap is TUI-adjacent and correctly belongs with the CLI tests (drives the tui --headless ENTRYPOINT, not tui internals) -- leave it. GOALS: (a) shrink tui.rs to a thin root (App + shared enums + mod decls + core ctors); logic in cohesive src/tui/ submodules; (b) CO-LOCATE tests + insta snapshots with their code (migrate src/snapshots/ -> per-module src/tui/**/snapshots/); (c) BEHAVIOR-PRESERVING: every phase compiles, all tests pass, snapshots move+rename with IDENTICAL content (the proof); (d) unlock future file-disjoint parallel UI work. INVARIANTS: SERIAL refactor (every phase edits tui.rs) -> ONE lane, sequential children (linear deps), NOT a worktree fan-out. impl App splits as multiple 'impl App' blocks across submodules (legal; child mods see App private fields). insta: moving a test changes its snapshot NAME (module path) + LOCATION -- move+rename the .snap then run tests to confirm they still MATCH; never let content drift silently. Per-phase verify: cargo test -p yaks + no stray .snap.new. docs<->reality: update AGENTS.md 'Layout' in the final phase. TARGET src/tui/: editor.rs, drawer.rs, create.rs, fuzzy.rs(+search), render/{mod,list,detail,status}.rs, viewmodel.rs, detail_nav.rs, handlers.rs, actions.rs, runtime.rs, test_support.rs; tui.rs keeps App + Focus/Overlay + mod decls.

---
▸ 2026-09-11T01:39:46Z [coordinator]
Arc execution decision (provenance): merge cadence for the persistent worktree. (A) squash-merge each completed phase to main at its checkpoint -- keeps main's herd honest, de-risks integration per phase, natural checkpoints (branch re-syncs via 'git merge main' after each); or (B) one merge at arc end -- simpler history, but main's b1cc children look stale for the whole arc. Coordinator lean: (A) per-phase checkpoints -- matches the serial, checkpointed nature and surfaces integration issues early. Also FYI my execution lean: coordinator drives each phase directly in the worktree (avoids the sub-agent file-tool SOP tax on this delicate snapshot-preserving refactor), spawning sub-agents only for bulk mechanical moves. Your call on cadence (A/B).
