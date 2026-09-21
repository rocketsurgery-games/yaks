use super::*;
use crate::tui::test_support::*;
use ratatui::style::{Color, Modifier};

#[test]
fn mark_toggles_selection_and_renders_gutter_dot() {
    let mut app = App::new(vec![
        task("t0", "first", Status::Hairy, 3, None),
        task("t1", "second", Status::Hairy, 3, None),
    ]);
    // Nothing marked yet: no selection dot on screen.
    assert!(!draw(&app, 80, 12).contains('\u{25cf}'));
    // `m` marks the cursor's yak; the gutter shows a filled dot.
    handle_key(&mut app, key('m'));
    assert!(app.selected.contains("t0"));
    assert!(draw(&app, 80, 12).contains('\u{25cf}'));
    // `m` again unmarks it, and the dot disappears.
    handle_key(&mut app, key('m'));
    assert!(!app.selected.contains("t0"));
    assert!(!draw(&app, 80, 12).contains('\u{25cf}'));
}

#[test]
fn tree_with_ghost_family() {
    // Hairy tab under `all` scope: Root A + A1 (focus), A2 (ghost, shorn)
    // pulled in as family.
    let mut app = sample();
    app.family_scope
        .insert("status:hairy".into(), view::FamilyScope::All);
    insta::assert_snapshot!(draw(&app, 72, 14));
}

#[test]
fn family_indicator_on_tab_row() {
    // Wide enough for the right-aligned farm indicator to sit past the tabs.
    // The default Hairy view inherits the global default: shown as `~remaining`.
    insta::assert_snapshot!(draw(&sample(), 100, 8));
}

#[test]
fn collapsed_root_hides_children() {
    let mut app = sample();
    app.collapsed.insert("a0".into());
    insta::assert_snapshot!(draw(&app, 72, 14));
}

#[test]
fn build_universe_pulls_ghost_descendants() {
    let mut app = sample();
    // `all` scope pulls in every descendant, including the completed a2.
    app.family_scope
        .insert("status:hairy".into(), view::FamilyScope::All);
    let rows = app.rows();
    let ids: Vec<&str> = rows.iter().map(|r| r.task.id.as_str()).collect();
    assert_eq!(ids, vec!["a0", "a1", "a2"]); // b0 (shaving) excluded from Hairy tab
    let a2 = rows.iter().find(|r| r.task.id == "a2").unwrap();
    assert!(a2.ghost, "shorn child should be a ghost in the Hairy tab");
    let a0 = rows.iter().find(|r| r.task.id == "a0").unwrap();
    assert!(a0.has_children && !a0.ghost);
}

#[test]
fn family_remaining_default_hides_completed_leaf() {
    // The default scope (remaining) drops the shorn leaf a2; open a1 stays.
    let app = sample();
    let ids: Vec<&str> = app.rows().iter().map(|r| r.task.id.as_str()).collect();
    assert_eq!(ids, vec!["a0", "a1"]);
}

// -- overlay rendering (farm-less; opening only needs `selected`) ------

#[test]
fn state_picker_overlay() {
    let mut app = sample();
    handle_key(&mut app, key('S'));
    insta::assert_snapshot!(draw(&app, 72, 14));
}

#[test]
fn priority_picker_overlay() {
    let mut app = sample();
    handle_key(&mut app, key('P'));
    insta::assert_snapshot!(draw(&app, 72, 14));
}

#[test]
fn type_picker_overlay() {
    let mut app = sample();
    handle_key(&mut app, key('T'));
    insta::assert_snapshot!(draw(&app, 72, 14));
}

#[test]
fn create_targets_the_reference_herd_in_a_multi_herd_farm() {
    // Two herds present -> a new yak inherits the selected row's herd (prefix).
    let mut app = App::new(vec![
        task("api-0001", "api", Status::Hairy, 3, None),
        task("web-0001", "web", Status::Hairy, 3, None),
    ]);
    app.open_create(false);
    match &app.overlay {
        Overlay::Create(f) => assert_eq!(f.herd.as_deref(), Some("api")),
        _ => panic!("expected create overlay"),
    }
}

#[test]
fn is_multi_herd_reflects_distinct_id_prefixes() {
    let single = App::new(vec![
        task("yak-0001", "a", Status::Hairy, 3, None),
        task("yak-0002", "b", Status::Hairy, 3, None),
    ]);
    assert!(!single.is_multi_herd());
    let multi = App::new(vec![
        task("web-0001", "a", Status::Hairy, 3, None),
        task("api-0001", "b", Status::Hairy, 3, None),
    ]);
    assert!(multi.is_multi_herd());
    // Exercise the per-herd id-colour render path (colours don't show in the
    // text snapshot, so just assert it renders the ids without panicking).
    let frame = draw(&multi, 80, 12);
    assert!(frame.contains("web-0001") && frame.contains("api-0001"));
}

#[test]
fn create_has_no_herd_target_in_a_single_herd_farm() {
    // One herd -> no explicit target; create falls through to the config default.
    let mut app = App::new(vec![
        task("yak-0001", "a", Status::Hairy, 3, None),
        task("yak-0002", "b", Status::Hairy, 3, None),
    ]);
    app.open_create(false);
    match &app.overlay {
        Overlay::Create(f) => assert!(f.herd.is_none()),
        _ => panic!("expected create overlay"),
    }
}

#[test]
fn slaughter_confirm_overlay() {
    // Move to a childless leaf (Child A1) so the confirm actually opens.
    let mut app = sample();
    handle_key(&mut app, key('j'));
    handle_key(&mut app, key('X'));
    assert!(matches!(app.overlay, Overlay::Confirm { .. }));
    insta::assert_snapshot!(draw(&app, 72, 14));
}

#[test]
fn slaughter_refused_with_children() {
    // Root A has a non-dead child (A1), so X should refuse and not open.
    let mut app = sample();
    handle_key(&mut app, key('X'));
    assert!(matches!(app.overlay, Overlay::None));
    insta::assert_snapshot!(app.notification.clone().unwrap());
}

fn open_body_editor(app: &mut App) {
    // A multiline comment editor, empty -> opens in Insert mode.
    app.overlay = Overlay::Edit(Editor::new(
        app.editor_vim,
        false,
        "Comment".into(),
        "",
        EditAction::Comment("a0".into()),
    ));
}

fn tab(app: &mut App) {
    handle_key(app, KeyEvent::new(KeyCode::Tab, KeyModifiers::NONE));
}

#[test]
fn tab_after_prefix_autocompletes_and_replaces_the_typed_token() {
    let mut app = sample(); // default ref_prefix "yak"
    open_body_editor(&mut app);
    // Type a partial ref, then Tab: the picker opens seeded by the tail.
    typ(&mut app, "yak-a1");
    tab(&mut app);
    assert!(matches!(app.overlay, Overlay::Fuzzy(_)));
    // Commit; the whole "yak-a1" is replaced by the picked id.
    handle_key(&mut app, KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE));
    match &app.overlay {
        Overlay::Edit(ed) => assert_eq!(ed.text(), "a1"),
        _ => panic!("expected to return to the editor"),
    }
}

#[test]
fn tab_in_a_content_block_without_context_does_not_navigate() {
    // Tab inside a content block is a normal editor key (Ctrl-N/P navigate
    // fields); it must not jump to the next field or open the picker.
    let mut app = sample();
    app.open_create(false);
    for _ in 0..8 {
        if matches!(&app.overlay, Overlay::Create(f) if f.is_content_row()) {
            break;
        }
        tab(&mut app);
    }
    assert!(matches!(&app.overlay, Overlay::Create(f) if f.is_content_row()));
    typ(&mut app, "hello");
    tab(&mut app);
    assert!(
        matches!(&app.overlay, Overlay::Create(f) if f.is_content_row()),
        "Tab in a content block must stay put, not navigate fields"
    );
}

#[test]
fn typing_prefix_dash_autocompletes_in_the_create_form_description() {
    // Regression: the create/edit form (Overlay::Create) is a separate key
    // path from the comment editor; the trigger must fire there too.
    let mut app = sample(); // default ref_prefix "yak"
    app.open_create(false);
    // Tab to the description content block (empty -> Insert mode).
    for _ in 0..8 {
        if matches!(&app.overlay, Overlay::Create(f) if f.is_content_row()) {
            break;
        }
        handle_key(&mut app, KeyEvent::new(KeyCode::Tab, KeyModifiers::NONE));
    }
    assert!(
        matches!(&app.overlay, Overlay::Create(f) if f.is_content_row()),
        "expected to reach the description content block"
    );
    typ(&mut app, "yak-");
    tab(&mut app);
    assert!(
        matches!(app.overlay, Overlay::Fuzzy(_)),
        "Tab after the prefix in the create form should open the ref picker"
    );
}

#[test]
fn single_esc_from_normal_closes_the_picker() {
    let mut app = sample(); // vim editor profile by default
    open_body_editor(&mut app);
    typ(&mut app, "yak-");
    tab(&mut app);
    assert!(matches!(app.overlay, Overlay::Fuzzy(_)));
    // First Esc: Insert -> Normal in the query; the picker stays open.
    handle_key(&mut app, KeyEvent::new(KeyCode::Esc, KeyModifiers::NONE));
    assert!(
        matches!(app.overlay, Overlay::Fuzzy(_)),
        "first Esc should only drop the query to Normal"
    );
    // A *non-rapid* second Esc (clear the double-Esc timer) still closes it.
    app.last_esc = None;
    handle_key(&mut app, KeyEvent::new(KeyCode::Esc, KeyModifiers::NONE));
    match &app.overlay {
        Overlay::Edit(ed) => assert_eq!(ed.text(), "yak-"),
        _ => panic!("a second Esc from Normal should close the picker"),
    }
}

#[test]
fn ref_picker_cancel_keeps_the_typed_prefix() {
    let mut app = sample();
    open_body_editor(&mut app);
    typ(&mut app, "yak-");
    tab(&mut app);
    assert!(matches!(app.overlay, Overlay::Fuzzy(_)));
    // Ctrl-C cancels the picker; we return to the editor with "yak-" intact.
    handle_key(
        &mut app,
        KeyEvent::new(KeyCode::Char('c'), KeyModifiers::CONTROL),
    );
    match &app.overlay {
        Overlay::Edit(ed) => assert_eq!(ed.text(), "yak-"),
        _ => panic!("expected to return to the editor"),
    }
}

#[test]
fn esc_cancels_picker() {
    let mut app = sample();
    handle_key(&mut app, key('P'));
    assert!(!matches!(app.overlay, Overlay::None));
    handle_key(&mut app, KeyEvent::new(KeyCode::Esc, KeyModifiers::NONE));
    assert!(matches!(app.overlay, Overlay::None));
    assert_eq!(app.notification.as_deref(), Some("cancelled"));
}

// -- editor overlay rendering (farm-less; open + render only) ----------

#[test]
fn label_editor_field() {
    // L opens a single-line field on the status line, seeded with labels.
    let mut app = editable();
    handle_key(&mut app, key('L'));
    assert!(matches!(app.overlay, Overlay::Edit(_)));
    insta::assert_snapshot!(draw(&app, 72, 14));
}

fn editor_state(app: &App) -> (EditorMode, String) {
    match &app.overlay {
        Overlay::Edit(ed) => {
            let st = ed.state.borrow();
            (st.mode, st.lines.to_string())
        }
        _ => panic!("no editor overlay open"),
    }
}

#[test]
fn x_and_shift_x_delete_chars() {
    // x deletes under the cursor, X deletes the previous char. (The yank to
    // the system clipboard is best-effort and not asserted here.)
    let mut app = editable();
    app.open_comment();
    typ(&mut app, "abcd");
    handle_key(&mut app, KeyEvent::new(KeyCode::Esc, KeyModifiers::NONE)); // Normal
    typ(&mut app, "0"); // col 0
    typ(&mut app, "x"); // remove 'a'
    assert_eq!(editor_state(&app).1, "bcd");
    typ(&mut app, "l"); // -> col 1 ('c')
    // X (Shift+X) removes the previous char; terminals send Shift with it.
    handle_key(
        &mut app,
        KeyEvent::new(KeyCode::Char('X'), KeyModifiers::SHIFT),
    );
    assert_eq!(editor_state(&app).1, "cd");
}

#[test]
fn single_line_vim_reaches_normal_mode() {
    // In vim mode a single-line field is modal: Esc switches to Normal and
    // never cancels, so the normal-mode keymap is usable; cancel is Ctrl-C
    // (or a rapid double-Esc).
    let mut app = editable();
    assert!(app.editor_vim);
    handle_key(&mut app, key('L')); // single-line labels editor (starts Insert)
    typ(&mut app, "aaa bbb");
    handle_key(&mut app, KeyEvent::new(KeyCode::Esc, KeyModifiers::NONE));
    // First Esc: still editing, now in Normal mode.
    let (mode, _) = editor_state(&app);
    assert_eq!(mode, EditorMode::Normal, "first Esc should enter Normal");
    // Normal-mode editing works: `dd` deletes the line.
    typ(&mut app, "dd");
    let (_, text) = editor_state(&app);
    assert_eq!(text, "", "normal-mode dd should clear the line");
    // Esc is modal in vim and never cancels on its own; Ctrl-C cancels.
    handle_key(
        &mut app,
        KeyEvent::new(KeyCode::Char('c'), KeyModifiers::CONTROL),
    );
    assert!(matches!(app.overlay, Overlay::None), "Ctrl-C cancels");
}

#[test]
fn change_line_routes_through_editor() {
    // cc (change whole line) reaches edtui via the fork.
    let mut app = editable();
    app.open_comment();
    typ(&mut app, "hello");
    handle_key(&mut app, KeyEvent::new(KeyCode::Esc, KeyModifiers::NONE)); // Normal
    typ(&mut app, "cc"); // clear the line, enter Insert
    typ(&mut app, "bye");
    assert_eq!(editor_state(&app).1, "bye");
}

#[test]
fn substitute_routes_through_editor() {
    // s (substitute char) reaches edtui via the fork.
    let mut app = editable();
    app.open_comment();
    typ(&mut app, "abc");
    handle_key(&mut app, KeyEvent::new(KeyCode::Esc, KeyModifiers::NONE)); // Normal
    typ(&mut app, "0s"); // start of line, substitute 'a'
    typ(&mut app, "X");
    assert_eq!(editor_state(&app).1, "Xbc");
}

#[test]
fn tilde_x_and_r_route_through_editor() {
    let mut app = editable();
    app.open_comment();
    typ(&mut app, "hello");
    handle_key(&mut app, KeyEvent::new(KeyCode::Esc, KeyModifiers::NONE)); // Normal
    typ(&mut app, "0"); // to col 0
    typ(&mut app, "~"); // Hello, cursor -> col 1
    assert_eq!(editor_state(&app).1, "Hello");
    // Cursor is on col 1 after ~; X (Shift+X) deletes the previous char 'H'.
    handle_key(
        &mut app,
        KeyEvent::new(KeyCode::Char('X'), KeyModifiers::SHIFT),
    );
    assert_eq!(editor_state(&app).1, "ello");
    typ(&mut app, "r"); // replace-char prefix
    typ(&mut app, "Y"); // 'e' -> 'Y'
    assert_eq!(editor_state(&app).1, "Yllo");
}

#[test]
fn editor_renders_markdown_highlights() {
    // Our hand-rolled highlighter runs at render for the editor; it must not
    // panic and must still show the text.
    let mut app = editable();
    app.open_comment();
    typ(&mut app, "# Heading");
    let out = draw(&app, 72, 14);
    assert!(out.contains("Heading"));
}

fn body_with_comments() -> Task {
    let mut t = task("c0", "Commented", Status::Hairy, 3, None);
    let b = crate::store::append_note(
        "The description.",
        "2026-01-01T00:00:00Z",
        None,
        "first note",
    );
    t.body = crate::store::append_note(&b, "2026-01-02T00:00:00Z", None, "second note");
    t
}

#[test]
fn vim_seeded_content_opens_in_normal_mode() {
    // Editing existing description/comment content lands in Normal (the vi
    // expectation when reviewing text).
    let mut app = App::new(vec![body_with_comments()]);
    assert!(app.editor_vim);
    app.open_edit();
    match &app.overlay {
        Overlay::Create(f) => {
            assert_eq!(f.blocks[0].editor.borrow().mode, EditorMode::Normal);
            assert_eq!(f.blocks[1].editor.borrow().mode, EditorMode::Normal);
        }
        _ => panic!("expected edit form"),
    }
}

#[test]
fn vim_new_content_opens_in_insert_mode() {
    // Fresh, empty content (new yak description, new comment) opens in Insert
    // so you can type immediately.
    let mut app = App::new(vec![task("t0", "t", Status::Hairy, 3, None)]);
    app.open_create(false);
    match &app.overlay {
        Overlay::Create(f) => assert_eq!(f.blocks[0].editor.borrow().mode, EditorMode::Insert),
        _ => panic!("expected create form"),
    }
    app.open_comment();
    match &app.overlay {
        Overlay::Edit(ed) => assert_eq!(ed.state.borrow().mode, EditorMode::Insert),
        _ => panic!("expected comment editor"),
    }
}

#[test]
fn e_on_comment_line_opens_form_focused_on_that_comment() {
    let mut app = App::new(vec![body_with_comments()]);
    handle_key(&mut app, key('l')); // enter the detail pane
    let starts = app.block_starts();
    assert_eq!(starts.len(), 3); // description + two comments
    app.detail_line = starts[2]; // second comment
    handle_key(
        &mut app,
        KeyEvent::new(KeyCode::Char('E'), KeyModifiers::NONE),
    );
    match &app.overlay {
        Overlay::Create(f) => assert_eq!(f.content_index(), Some(2)),
        _ => panic!("E should open the edit form"),
    }
}

#[test]
fn e_on_title_line_opens_form_on_the_title_row() {
    let mut app = App::new(vec![body_with_comments()]);
    handle_key(&mut app, key('l'));
    let lines = app.detail_dlines();
    let ln = lines
        .iter()
        .position(|l| l.text.starts_with("Title:"))
        .unwrap();
    app.detail_line = ln;
    handle_key(
        &mut app,
        KeyEvent::new(KeyCode::Char('E'), KeyModifiers::NONE),
    );
    match &app.overlay {
        Overlay::Create(f) => assert_eq!(f.row, 0),
        _ => panic!("E should open the edit form"),
    }
}

#[test]
fn ctrl_n_p_navigate_content_blocks_in_detail() {
    let mut app = App::new(vec![body_with_comments()]);
    handle_key(&mut app, key('l')); // detail; line cursor at top
    let starts = app.block_starts();
    let cn = KeyEvent::new(KeyCode::Char('n'), KeyModifiers::CONTROL);
    let cp = KeyEvent::new(KeyCode::Char('p'), KeyModifiers::CONTROL);
    handle_key(&mut app, cn);
    assert_eq!(app.detail_line, starts[1]); // first comment
    handle_key(&mut app, cn);
    assert_eq!(app.detail_line, starts[2]); // second comment
    handle_key(&mut app, cp);
    assert_eq!(app.detail_line, starts[1]); // back to first
}

#[test]
fn unfocused_description_preview_wraps() {
    // The edit form opens on the title row, so the description shows as the
    // dimmed preview. A long logical line must soft-wrap (like the focused
    // editor and the detail view) rather than truncate at the pane edge, so
    // its tail word stays visible.
    let mut t = task("e0", "Editable", Status::Hairy, 3, None);
    t.body = "alpha beta gamma delta epsilon zeta eta theta iota kappa lambda".into();
    let mut app = App::new(vec![t]);
    app.open_edit();
    assert!(
        !matches!(app.overlay, Overlay::None),
        "edit form should open"
    );
    let out = draw(&app, 40, 20);
    assert!(
        out.contains("lambda"),
        "wrapped tail word should be visible:\n{out}"
    );
}

#[test]
fn editor_header_shows_mode_tag() {
    let mut app = editable();
    app.open_comment(); // multiline editor, starts Insert
    let insert_frame = draw(&app, 72, 14);
    assert!(insert_frame.contains("INSERT"), "insert mode tag in header");
    handle_key(&mut app, KeyEvent::new(KeyCode::Esc, KeyModifiers::NONE));
    let normal_frame = draw(&app, 72, 14);
    assert!(normal_frame.contains("NORMAL"), "normal mode tag after Esc");
}

#[test]
fn single_line_emacs_esc_cancels_immediately() {
    // With the emacs profile there is no Normal mode, so Esc must still
    // cancel a single-line field on the first press.
    let mut app = editable();
    app.editor_vim = false;
    handle_key(&mut app, key('L'));
    assert!(matches!(app.overlay, Overlay::Edit(_)));
    handle_key(&mut app, KeyEvent::new(KeyCode::Esc, KeyModifiers::NONE));
    assert!(matches!(app.overlay, Overlay::None), "emacs Esc cancels");
}

#[test]
fn create_form() {
    // Open the create form, type a title, then move to the priority chip row.
    let mut app = editable();
    handle_key(&mut app, key('c'));
    typ(&mut app, "new idea");
    handle_key(&mut app, KeyEvent::new(KeyCode::Tab, KeyModifiers::NONE)); // -> type
    handle_key(&mut app, KeyEvent::new(KeyCode::Tab, KeyModifiers::NONE)); // -> priority
    insta::assert_snapshot!(draw(&app, 72, 14));
}

#[test]
fn help_overlay() {
    let mut app = sample();
    handle_key(&mut app, key('?'));
    insta::assert_snapshot!(draw(&app, 72, 16));
}

#[test]
fn priority_palette_matches_python() {
    // P1 red+bold, P2 magenta, P3 yellow, P4 green, P5 blue+dim (yaksrs-bce4).
    assert_eq!(priority_style(1).fg, Some(Color::Red));
    assert!(priority_style(1).add_modifier.contains(Modifier::BOLD));
    assert_eq!(priority_style(2).fg, Some(Color::Magenta));
    assert_eq!(priority_style(3).fg, Some(Color::Yellow));
    assert_eq!(priority_style(4).fg, Some(Color::Green));
    assert_eq!(priority_style(5).fg, Some(Color::Blue));
    assert!(priority_style(5).add_modifier.contains(Modifier::DIM));
}

#[test]
fn help_opens_and_closes() {
    let mut app = sample();
    handle_key(&mut app, key('?'));
    assert!(matches!(app.overlay, Overlay::Help(_)));
    // Scroll down, then Esc closes.
    handle_key(&mut app, key('j'));
    assert!(matches!(app.overlay, Overlay::Help(s) if s == 1));
    esc_key(&mut app);
    assert!(matches!(app.overlay, Overlay::None));
}

#[test]
fn edit_form_panel() {
    // E opens the shared form seeded from the task, with the description
    // content zone focused (Tab past the meta rows).
    let mut app = editable();
    handle_key(&mut app, key('E'));
    match &app.overlay {
        Overlay::Create(f) => assert!(f.is_editing()),
        _ => panic!("expected edit form"),
    }
    for _ in 0..4 {
        handle_key(&mut app, KeyEvent::new(KeyCode::Tab, KeyModifiers::NONE));
    }
    insta::assert_snapshot!(draw(&app, 72, 14));
}

fn typ(app: &mut App, s: &str) {
    for c in s.chars() {
        handle_key(app, key(c));
    }
}

#[test]
fn inline_search_field_and_recolor() {
    // '/' then "child" focuses the two matching children; Root A dims as
    // their ancestor, Root B is pruned.
    let mut app = sample();
    handle_key(&mut app, key('/'));
    typ(&mut app, "child");
    assert!(matches!(app.overlay, Overlay::Search(_)));
    insta::assert_snapshot!(draw(&app, 72, 14));
}

#[test]
fn inline_search_updates_filter_live() {
    let mut app = sample();
    handle_key(&mut app, key('/'));
    typ(&mut app, "child");
    assert_eq!(app.filter.search.as_deref(), Some("child"));
    let ids: Vec<&str> = app.rows().iter().map(|r| r.task.id.as_str()).collect();
    assert_eq!(ids, vec!["a0", "a1", "a2"]); // b0 pruned; a0 dimmed ancestor
    // Enter keeps the filter.
    handle_key(&mut app, KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE));
    assert!(matches!(app.overlay, Overlay::None));
    assert_eq!(app.filter.search.as_deref(), Some("child"));
    // Esc in the list clears the active filter.
    handle_key(&mut app, KeyEvent::new(KeyCode::Esc, KeyModifiers::NONE));
    assert!(!app.filter.content_active());
}

#[test]
fn inline_search_ctrl_c_restores_previous() {
    // In vim a lone Esc enters Normal, so cancel-and-restore is Ctrl-C.
    let mut app = sample();
    handle_key(&mut app, key('/'));
    typ(&mut app, "zzz");
    assert_eq!(app.filter.search.as_deref(), Some("zzz"));
    handle_key(
        &mut app,
        KeyEvent::new(KeyCode::Char('c'), KeyModifiers::CONTROL),
    );
    assert!(matches!(app.overlay, Overlay::None));
    assert!(app.filter.search.is_none());
}

#[test]
fn inline_search_esc_enters_normal_in_vim() {
    let mut app = sample();
    handle_key(&mut app, key('/'));
    typ(&mut app, "zz");
    handle_key(&mut app, KeyEvent::new(KeyCode::Esc, KeyModifiers::NONE));
    // Still open, now modal: the live filter is untouched.
    assert!(matches!(app.overlay, Overlay::Search(_)));
    if let Overlay::Search(sb) = &app.overlay {
        assert_eq!(sb.query.borrow().mode, EditorMode::Normal);
    }
    assert_eq!(app.filter.search.as_deref(), Some("zz"));
}

#[test]
fn drawer_text_row_esc_enters_normal_in_vim() {
    let mut app = sample();
    handle_key(&mut app, key('f')); // open filter drawer
    for _ in 0..3 {
        handle_key(&mut app, KeyEvent::new(KeyCode::Tab, KeyModifiers::NONE));
    }
    typ(&mut app, "ui"); // into the labels text row
    handle_key(&mut app, KeyEvent::new(KeyCode::Esc, KeyModifiers::NONE));
    assert!(
        matches!(app.overlay, Overlay::Drawer(_)),
        "a lone Esc on a text row stays in the drawer"
    );
    if let Overlay::Drawer(d) = &app.overlay {
        assert_eq!(d.labels.borrow().mode, EditorMode::Normal);
    }
}

#[test]
fn drawer_normal_jk_navigates_and_carries_mode() {
    let mut app = sample();
    handle_key(&mut app, key('f'));
    for _ in 0..3 {
        handle_key(&mut app, KeyEvent::new(KeyCode::Tab, KeyModifiers::NONE));
    }
    handle_key(&mut app, KeyEvent::new(KeyCode::Esc, KeyModifiers::NONE)); // labels -> Normal
    handle_key(&mut app, key('j')); // next text row, carrying Normal
    if let Overlay::Drawer(d) = &app.overlay {
        assert_eq!(d.row, 4);
        assert_eq!(d.search.borrow().mode, EditorMode::Normal);
    } else {
        panic!("drawer closed");
    }
    handle_key(&mut app, key('k')); // back up to labels, still Normal
    if let Overlay::Drawer(d) = &app.overlay {
        assert_eq!(d.row, 3);
        assert_eq!(d.labels.borrow().mode, EditorMode::Normal);
    }
}

#[test]
fn create_form_normal_j_navigates_rows() {
    let mut app = sample();
    handle_key(&mut app, key('c')); // create form, title row (Insert)
    handle_key(&mut app, KeyEvent::new(KeyCode::Esc, KeyModifiers::NONE)); // title -> Normal
    handle_key(&mut app, key('j')); // moves off the title row
    if let Overlay::Create(f) = &app.overlay {
        assert_eq!(f.row, 1);
    } else {
        panic!("form closed");
    }
    // In Insert, j types instead of navigating.
    handle_key(
        &mut app,
        KeyEvent::new(KeyCode::Char('k'), KeyModifiers::NONE),
    ); // back to title (chip k-nav)
    handle_key(&mut app, key('i')); // ensure Insert on title
    handle_key(&mut app, key('j'));
    if let Overlay::Create(f) = &app.overlay {
        assert_eq!(f.row, 0, "j types in Insert, does not navigate");
        assert!(f.title_text().contains('j'));
    }
}

#[test]
fn filter_drawer_overlay() {
    // f opens the drawer; toggle the first status chip to show selection.
    let mut app = sample();
    handle_key(&mut app, key('f'));
    handle_key(&mut app, key(' ')); // toggle status=hairy (row 0, chip 0)
    assert!(matches!(app.overlay, Overlay::Drawer(_)));
    insta::assert_snapshot!(draw(&app, 72, 16));
}

#[test]
fn drawer_toggle_applies_live_and_commits() {
    let mut app = sample();
    handle_key(&mut app, key('f')); // row 0 (status)
    handle_key(&mut app, key('j')); // -> row 1 (type)
    handle_key(&mut app, key(' ')); // toggle type=task (chip 0)
    assert_eq!(app.filter.types, vec!["task".to_string()]); // live preview
    enter_key(&mut app); // apply
    assert!(matches!(app.overlay, Overlay::None));
    assert_eq!(app.filter.types, vec!["task".to_string()]);
}

#[test]
fn drawer_cancel_restores_saved() {
    let mut app = sample();
    handle_key(&mut app, key('f'));
    handle_key(&mut app, key('j')); // row 1 (type)
    handle_key(&mut app, key(' ')); // toggle task (live)
    assert!(app.filter.content_active());
    esc_key(&mut app); // cancel reverts
    assert!(matches!(app.overlay, Overlay::None));
    assert!(!app.filter.content_active());
}

#[test]
fn drawer_clear_empties_filter() {
    let mut app = sample();
    app.filter.types = vec!["bug".into()];
    handle_key(&mut app, key('f')); // seeded from the active filter
    handle_key(&mut app, key('C')); // clear all
    assert!(!app.filter.content_active());
}

#[test]
fn drawer_text_row_typing_sets_search() {
    let mut app = sample();
    handle_key(&mut app, key('f'));
    // Navigate to the search text row (row 4) with Down (works on all rows).
    for _ in 0..4 {
        down_key(&mut app);
    }
    typ(&mut app, "root");
    assert_eq!(app.filter.search.as_deref(), Some("root"));
    enter_key(&mut app);
    assert_eq!(app.filter.search.as_deref(), Some("root"));
}

#[test]
fn detail_shows_children_and_links() {
    let mut app = linked();
    enter_key(&mut app); // list Enter -> focus detail
    assert_eq!(app.focus, Focus::Detail);
    insta::assert_snapshot!(draw(&app, 72, 16));
}

#[test]
fn detail_jumplist_follows_to_task() {
    let mut app = linked();
    enter_key(&mut app); // enter detail
    assert_eq!(app.focus, Focus::Detail);
    // Jumplist order for a0: child a1, child a2, body a1, body a2.
    assert!(app.detail_jumps().len() >= 2);
    tab_key(&mut app); // line cursor -> first link line (a1)
    enter_key(&mut app); // follow it -> a1
    // We stay in the detail pane, now showing the followed task (yaksrs-3f19).
    assert_eq!(app.focus, Focus::Detail);
    assert_eq!(app.selected_id().as_deref(), Some("a1"));
    assert_eq!(app.notification.as_deref(), Some("→ a1"));
}

#[test]
fn follow_link_reveals_collapsed_target() {
    // a1 is hidden under a collapsed a0; following the link must expand a0,
    // select a1, and stay in the detail pane (yaksrs-3f19).
    let mut app = linked();
    app.collapsed.insert("a0".to_string());
    assert!(app.rows().iter().all(|r| r.task.id != "a1"), "a1 hidden");
    enter_key(&mut app); // focus detail on a0
    tab_key(&mut app); // -> first link line (a1)
    enter_key(&mut app); // follow it -> a1
    assert_eq!(app.focus, Focus::Detail);
    assert_eq!(app.selected_id().as_deref(), Some("a1"));
    assert!(!app.collapsed.contains("a0"), "ancestor expanded");
}

#[test]
fn nav_history_back_and_forward() {
    // o/i retrace the link-follow chain (yaksrs-5d63).
    let mut app = linked();
    enter_key(&mut app); // detail on a0
    tab_key(&mut app); // -> first link line (a1)
    enter_key(&mut app); // follow it -> a1
    assert_eq!(app.selected_id().as_deref(), Some("a1"));
    handle_key(&mut app, key('o')); // back -> a0
    assert_eq!(app.selected_id().as_deref(), Some("a0"));
    assert_eq!(app.focus, Focus::Detail);
    handle_key(&mut app, key('i')); // forward -> a1
    assert_eq!(app.selected_id().as_deref(), Some("a1"));
    handle_key(&mut app, key('i')); // nothing further
    assert_eq!(app.notification.as_deref(), Some("no later yak"));
}

#[test]
fn nav_back_on_empty_history_is_noop() {
    let mut app = linked();
    enter_key(&mut app); // detail on a0, no history yet
    handle_key(&mut app, key('o'));
    assert_eq!(app.selected_id().as_deref(), Some("a0"));
    assert_eq!(app.notification.as_deref(), Some("no earlier yak"));
}

#[test]
fn detail_tab_cycles_link_lines() {
    // Tab snaps the line cursor onto successive link lines; Shift-Tab back.
    let mut app = linked();
    enter_key(&mut app);
    assert_eq!(app.detail_line, 0);
    tab_key(&mut app);
    let first = app.detail_line;
    assert!(first > 0, "moved onto a link line");
    tab_key(&mut app);
    assert!(app.detail_line > first, "advanced to the next link line");
    handle_key(
        &mut app,
        KeyEvent::new(KeyCode::BackTab, KeyModifiers::NONE),
    );
    assert_eq!(app.detail_line, first, "back to the first link line");
}

#[test]
fn detail_visual_selection_and_esc_clears() {
    let mut app = linked();
    enter_key(&mut app);
    assert_eq!(app.detail_anchor, None);
    handle_key(&mut app, key('v')); // anchor at the current line (0)
    assert_eq!(app.detail_anchor, Some(0));
    handle_key(&mut app, key('j')); // cursor -> 1
    handle_key(&mut app, key('j')); // cursor -> 2
    assert_eq!(app.selection_range(), Some((0, 2)));
    esc_key(&mut app); // Esc peels back the selection first
    assert_eq!(app.detail_anchor, None);
    assert_eq!(
        app.focus,
        Focus::Detail,
        "still in detail after clearing sel"
    );
}

#[test]
fn scroll_into_view_is_stable_and_minimal() {
    let mut app = App::new(vec![]);
    app.detail_page = 10; // viewport shows 10 rows
    app.detail_scroll = 5; // currently rows 5..15
    app.scroll_line_into_view(8); // already visible -> unchanged
    assert_eq!(app.detail_scroll, 5);
    app.scroll_line_into_view(2); // above -> scroll up to it
    assert_eq!(app.detail_scroll, 2);
    app.detail_scroll = 5;
    app.scroll_line_into_view(20); // below -> land it on the last row
    assert_eq!(app.detail_scroll, 20 - (10 - 1));
}

// -- view substrate (6b-i) --------------------------------------------

#[test]
fn starred_marker_and_tab_bar() {
    let mut app = sample();
    handle_key(&mut app, key('*')); // star a0
    insta::assert_snapshot!(draw(&app, 72, 14));
}

#[test]
fn default_views_and_tab_cycling() {
    let mut app = sample();
    assert_eq!(app.views.len(), 6); // 3 status + Recent + Inbox + Starred
    assert_eq!(app.view, 0);
    assert_eq!(app.filter.statuses, vec![Status::Hairy]);
    tab_key(&mut app); // -> Shaving
    assert_eq!(app.view, 1);
    assert_eq!(app.filter.statuses, vec![Status::Shaving]);
}

#[test]
fn recent_view_is_flat_over_all_tasks() {
    let mut app = sample();
    // Recent is index 3 (after the 3 status views).
    app.set_view(3);
    assert_eq!(app.active_view().key, "recent");
    assert!(app.active_view().is_flat());
    let rows = app.rows();
    assert_eq!(rows.len(), 4); // all sample tasks, flat
    assert!(rows.iter().all(|r| r.depth == 0 && !r.ghost));
}

#[test]
fn star_toggles_and_starred_view_lists_it() {
    let mut app = sample(); // cursor on a0
    handle_key(&mut app, key('*'));
    assert!(app.is_starred("a0"));
    // Starred view (index 5) lists the starred task.
    app.set_view(5);
    assert_eq!(app.active_view().key, "working-set");
    let ids: Vec<&str> = app.rows().iter().map(|r| r.task.id.as_str()).collect();
    assert_eq!(ids, vec!["a0"]);
    // Unstar removes it.
    app.set_view(0);
    handle_key(&mut app, key('*'));
    assert!(!app.is_starred("a0"));
}

#[test]
fn save_view_creates_and_activates_custom_view() {
    let mut app = sample();
    handle_key(&mut app, key('/')); // inline search
    typ(&mut app, "child");
    enter_key(&mut app); // keep filter (search=child)
    handle_key(&mut app, key('V')); // save view
    typ(&mut app, "kids");
    enter_key(&mut app); // commit name
    assert_eq!(app.views.len(), 7);
    assert_eq!(app.view, 6);
    let v = app.active_view();
    assert_eq!(v.name, "kids");
    assert_eq!(v.spec.search.as_deref(), Some("child"));
    assert!(!v.builtin && v.pinned);
}

// -- view picker (6b-ii) ----------------------------------------------

#[test]
fn view_picker_overlay() {
    let mut app = sample();
    handle_key(&mut app, key('v'));
    assert!(matches!(app.overlay, Overlay::ViewPicker(_)));
    insta::assert_snapshot!(draw(&app, 72, 16));
}

#[test]
fn picker_activates_selected_view() {
    let mut app = sample();
    handle_key(&mut app, key('v'));
    for _ in 0..3 {
        handle_key(&mut app, key('j'));
    }
    enter_key(&mut app);
    assert!(matches!(app.overlay, Overlay::None));
    assert_eq!(app.active_view().key, "recent");
}

#[test]
fn picker_unpin_removes_from_tab_bar() {
    let mut app = sample();
    handle_key(&mut app, key('v'));
    handle_key(&mut app, key('j')); // sel 1 = Shaving
    handle_key(&mut app, key('p')); // unpin
    assert!(!app.views[1].pinned);
    assert!(!app.pinned_indices().contains(&1));
}

#[test]
fn picker_move_keeps_active_view() {
    let mut app = sample(); // active = Hairy (0)
    handle_key(&mut app, key('v'));
    handle_key(&mut app, key('J')); // move Hairy down
    assert_eq!(app.views[0].status, Some(Status::Shaving));
    assert_eq!(app.views[1].status, Some(Status::Hairy));
    assert_eq!(app.active_view().status, Some(Status::Hairy)); // followed the move
}

#[test]
fn picker_rename_returns_to_picker() {
    let mut app = sample();
    handle_key(&mut app, key('v'));
    handle_key(&mut app, key('r')); // seeded with the current name
    typ(&mut app, "Fuzz");
    enter_key(&mut app);
    assert!(app.views[0].name.contains("Fuzz") && app.views[0].name != "Hairy");
    assert!(matches!(app.overlay, Overlay::ViewPicker(0)));
}

#[test]
fn picker_deletes_custom_but_not_builtin() {
    let mut app = sample();
    // Create a custom view first.
    handle_key(&mut app, key('/'));
    typ(&mut app, "x");
    enter_key(&mut app);
    handle_key(&mut app, key('V'));
    typ(&mut app, "mine");
    enter_key(&mut app);
    assert_eq!(app.views.len(), 7);
    // Delete it via the picker (active view is the custom one, last index).
    handle_key(&mut app, key('v'));
    handle_key(&mut app, key('d'));
    assert_eq!(app.views.len(), 6);
    // Deleting a built-in is refused.
    handle_key(&mut app, KeyEvent::new(KeyCode::Esc, KeyModifiers::NONE));
    handle_key(&mut app, key('v'));
    // selection is clamped to a built-in row; delete refuses.
    handle_key(&mut app, key('g')); // no-op; ensure sel 0 path
    let before = app.views.len();
    // move selection to top then delete
    handle_key(&mut app, key('k'));
    handle_key(&mut app, key('k'));
    handle_key(&mut app, key('k'));
    handle_key(&mut app, key('k'));
    handle_key(&mut app, key('k'));
    handle_key(&mut app, key('d'));
    assert_eq!(app.views.len(), before);
    assert_eq!(
        app.notification.as_deref(),
        Some("can't delete a built-in view")
    );
}

#[test]
fn esc_reverts_modified_filter_to_view() {
    let mut app = sample();
    handle_key(&mut app, key('/'));
    typ(&mut app, "zzz");
    enter_key(&mut app);
    assert!(app.is_view_modified());
    handle_key(&mut app, KeyEvent::new(KeyCode::Esc, KeyModifiers::NONE));
    assert!(!app.is_view_modified());
    assert_eq!(app.filter.statuses, vec![Status::Hairy]); // back to Hairy view spec
}

#[test]
fn detail_find_matches_and_cycles() {
    let mut app = linked();
    enter_key(&mut app); // focus detail
    handle_key(&mut app, key('/')); // open detail find
    typ(&mut app, "child");
    assert_eq!(app.detail_find.as_deref(), Some("child"));
    assert!(app.detail_find_matches().len() >= 2); // both child lines
    enter_key(&mut app); // keep the find
    assert!(matches!(app.overlay, Overlay::None));
    assert_eq!(app.focus, Focus::Detail);
    assert_eq!(app.detail_match, 0);
    handle_key(&mut app, key('n'));
    assert_eq!(app.detail_match, 1);
    handle_key(&mut app, key('N'));
    assert_eq!(app.detail_match, 0);
}

#[test]
fn detail_body_wraps_at_narrow_width() {
    let mut t = task("a0", "Root A", Status::Hairy, 2, None);
    t.body = "alpha beta gamma delta epsilon zeta eta theta iota kappa".into();
    let mut app = App::new(vec![t]);
    enter_key(&mut app); // focus detail
    // Wide: the body fits on a single row.
    let _ = draw(&app, 200, 24);
    let wide = app.detail_line_count();
    // Narrow: the body must wrap into extra rows, and the trailing word
    // (off the right edge if it ran off unwrapped) stays visible.
    let out = draw(&app, 44, 24);
    let narrow = app.detail_line_count();
    assert!(
        narrow > wide,
        "narrow pane should wrap the body into more rows"
    );
    assert!(
        out.contains("kappa"),
        "the last body word should still be visible"
    );
}

#[test]
fn detail_find_ctrl_c_restores() {
    let mut app = linked();
    enter_key(&mut app);
    handle_key(&mut app, key('/'));
    typ(&mut app, "zzz");
    assert_eq!(app.detail_find.as_deref(), Some("zzz"));
    handle_key(
        &mut app,
        KeyEvent::new(KeyCode::Char('c'), KeyModifiers::CONTROL),
    );
    assert!(matches!(app.overlay, Overlay::None));
    assert!(app.detail_find.is_none());
}

#[test]
fn detail_find_esc_enters_normal_in_vim() {
    let mut app = linked();
    enter_key(&mut app);
    handle_key(&mut app, key('/'));
    typ(&mut app, "zz");
    handle_key(&mut app, KeyEvent::new(KeyCode::Esc, KeyModifiers::NONE));
    assert!(matches!(app.overlay, Overlay::DetailFind(_)));
    if let Overlay::DetailFind(sb) = &app.overlay {
        assert_eq!(sb.query.borrow().mode, EditorMode::Normal);
    }
}

#[test]
fn detail_find_overlay() {
    let mut app = linked();
    enter_key(&mut app);
    handle_key(&mut app, key('/'));
    typ(&mut app, "child");
    insta::assert_snapshot!(draw(&app, 72, 16));
}

#[test]
fn detail_find_enter_lands_cursor_on_match() {
    // Enter commits the find on the current match (vi-like): the detail
    // cursor moves ONTO the match line and the scroll stays put, rather
    // than snapping back to where the cursor was before the search.
    let mut app = linked();
    enter_key(&mut app); // focus detail; cursor + scroll at the top
    assert_eq!(app.detail_line, 0);
    handle_key(&mut app, key('/'));
    typ(&mut app, "child"); // matches "Child A1" (line 2), "Child A2" (line 3)
    let m = app.detail_find_matches();
    assert!(m.len() >= 2);
    let match_line = m[app.detail_match].0;
    assert!(match_line > 0, "match should be below the pre-find cursor");
    assert_eq!(app.detail_scroll as usize, match_line); // find scrolled here
    enter_key(&mut app); // commit
    assert!(matches!(app.overlay, Overlay::None));
    assert_eq!(app.focus, Focus::Detail);
    // Cursor sits ON the current match; the find's scroll is preserved.
    assert_eq!(app.detail_line, match_line);
    assert_eq!(app.detail_scroll as usize, match_line);
}

#[test]
fn detail_find_enter_commits_the_cycled_match() {
    // Cycling with n before committing lands the cursor on that later
    // match, not always the first one.
    let mut app = linked();
    enter_key(&mut app);
    handle_key(&mut app, key('/'));
    typ(&mut app, "child");
    enter_key(&mut app); // commit on match 0
    handle_key(&mut app, key('n')); // advance to match 1
    assert_eq!(app.detail_match, 1);
    let second = app.detail_find_matches()[1].0;
    handle_key(&mut app, key('/')); // reopen (query + match index persist)
    enter_key(&mut app); // commit on the current match (1)
    assert!(matches!(app.overlay, Overlay::None));
    assert_eq!(app.detail_match, 1);
    assert_eq!(app.detail_line, second);
}

#[test]
fn detail_find_esc_restores_pre_find_position() {
    // Esc abandons the find and returns the cursor + scroll to where they
    // were before it opened (vi restores the origin on cancel).
    let mut app = linked();
    enter_key(&mut app); // focus detail
    handle_key(&mut app, key('j')); // move the cursor off the top
    let origin_line = app.detail_line;
    let origin_scroll = app.detail_scroll;
    assert!(origin_line > 0);
    handle_key(&mut app, key('/'));
    typ(&mut app, "child"); // scrolls the pane to the first match
    assert!(app.detail_find_matches().len() >= 2);
    assert_ne!(
        app.detail_scroll, origin_scroll,
        "find should move the view"
    );
    // Vim single-line field: first Esc drops to Normal, the rapid second
    // Esc is the cancel gesture.
    handle_key(&mut app, KeyEvent::new(KeyCode::Esc, KeyModifiers::NONE));
    handle_key(&mut app, KeyEvent::new(KeyCode::Esc, KeyModifiers::NONE));
    assert!(matches!(app.overlay, Overlay::None));
    assert!(app.detail_find.is_none());
    assert_eq!(app.detail_line, origin_line);
    assert_eq!(app.detail_scroll, origin_scroll);
}

#[test]
fn dep_picker_overlay() {
    // D on Root A: results (self excluded) in the right pane, query at foot.
    let mut app = sample();
    handle_key(&mut app, key('D'));
    assert!(matches!(app.overlay, Overlay::Fuzzy(_)));
    insta::assert_snapshot!(draw(&app, 72, 14));
}

#[test]
fn reparent_picker_overlay() {
    // R on Child A1 (has a parent) shows the clear-parent row first.
    let mut app = sample();
    handle_key(&mut app, key('j')); // move to a1
    handle_key(&mut app, key('R'));
    assert!(matches!(app.overlay, Overlay::Fuzzy(_)));
    insta::assert_snapshot!(draw(&app, 72, 14));
}

#[test]
fn needs_badge_renders_on_a_blocked_row() {
    // A yak carrying a `needs` block gets the ⏳ badge; an unblocked one
    // does not (the badge is per-row, on the right).
    let mut blocked = task("t0", "waiting on a human", Status::Hairy, 3, None);
    blocked.needs = Some("human".into());
    let app = App::new(vec![blocked, task("t1", "free", Status::Hairy, 3, None)]);
    let out = draw(&app, 80, 8);
    assert!(out.contains('\u{23f3}'), "hourglass badge present:\n{out}");
}

#[test]
fn inbox_view_lists_needs_blocked_across_statuses() {
    // The Inbox built-in view is a flat list of every yak awaiting a human,
    // regardless of status (a shorn-but-blocked yak still surfaces). It's
    // driven by the shared `needs_only` predicate, replacing the old modal
    // `i` toggle.
    let mut hairy = task("t0", "ask me", Status::Hairy, 3, None);
    hairy.needs = Some("human".into());
    let mut shorn = task("t1", "still blocked", Status::Shorn, 3, None);
    shorn.needs = Some("human".into());
    let free = task("t2", "unblocked", Status::Hairy, 3, None);
    let mut app = App::new(vec![hairy, shorn, free]);
    let idx = app
        .views
        .iter()
        .position(|v| v.key == "inbox")
        .expect("inbox view present in defaults");
    app.set_view(idx);
    assert!(app.active_view().is_flat());
    let mut ids: Vec<String> = app.rows().iter().map(|r| r.task.id.clone()).collect();
    ids.sort();
    assert_eq!(ids, vec!["t0".to_string(), "t1".to_string()]);
    // The free (unblocked) yak never appears in the inbox.
    assert!(!ids.contains(&"t2".to_string()));
    // The tab/picker count reflects the awaiting-a-human set, not the farm.
    assert_eq!(app.view_count(app.active_view()), 2);
    // Switching to the Hairy view drops the shorn yak again — inbox is a
    // view/filter, not a persistent override.
    app.set_view(0);
    let ids: Vec<String> = app.rows().iter().map(|r| r.task.id.clone()).collect();
    assert!(ids.contains(&"t0".to_string()) && ids.contains(&"t2".to_string()));
    assert!(
        !ids.contains(&"t1".to_string()),
        "shorn hidden in Hairy view"
    );
}

#[test]
fn drawer_inbox_chip_composes_with_status_scope() {
    // The inbox facet is the third chip in the drawer's `deps` row: toggling
    // it sets `needs_only` on the built spec without disturbing the rest of
    // the filter, so the inbox composes with any status scope.
    let base = FilterSpec {
        statuses: vec![Status::Hairy],
        ..Default::default()
    };
    let mut d = Drawer::from_filter(false, &base);
    d.row = 6; // deps row
    d.chip_idx = 2; // inbox chip
    d.toggle_chip();
    let spec = d.build_spec();
    assert!(spec.needs_only);
    assert_eq!(spec.statuses, vec![Status::Hairy], "status scope preserved");
    // Toggling it back off round-trips cleanly.
    let mut d2 = Drawer::from_filter(false, &spec);
    d2.row = 6;
    d2.chip_idx = 2;
    d2.toggle_chip();
    assert!(!d2.build_spec().needs_only);
}

// -- live mutation through a temp farm --------------------------------

mod live {
    use super::*;
    use std::fs;

    fn enter(app: &mut App) {
        handle_key(app, KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE));
    }

    fn tab(app: &mut App) {
        handle_key(app, KeyEvent::new(KeyCode::Tab, KeyModifiers::NONE));
    }

    fn back_tab(app: &mut App) {
        handle_key(app, KeyEvent::new(KeyCode::BackTab, KeyModifiers::NONE));
    }

    fn arrow_right(app: &mut App) {
        handle_key(app, KeyEvent::new(KeyCode::Right, KeyModifiers::NONE));
    }

    fn arrow_left(app: &mut App) {
        handle_key(app, KeyEvent::new(KeyCode::Left, KeyModifiers::NONE));
    }

    fn ctrl_s(app: &mut App) {
        handle_key(
            app,
            KeyEvent::new(KeyCode::Char('s'), KeyModifiers::CONTROL),
        );
    }

    fn ctrl_n(app: &mut App) {
        handle_key(
            app,
            KeyEvent::new(KeyCode::Char('n'), KeyModifiers::CONTROL),
        );
    }

    fn esc(app: &mut App) {
        handle_key(app, KeyEvent::new(KeyCode::Esc, KeyModifiers::NONE));
    }

    #[test]
    fn state_pick_transitions_and_reloads() {
        let (_dir, farm) = temp_farm(&[task("t0", "solo", Status::Hairy, 3, None)]);
        let mut app = App::with_farm(farm).unwrap();
        // S then n → shorn. The task leaves the Hairy tab.
        press(&mut app, "Sn");
        assert!(matches!(app.overlay, Overlay::None));
        let t = app.task("t0").unwrap();
        assert_eq!(t.status, Status::Shorn);
        assert_eq!(app.notification.as_deref(), Some("t0 → shorn"));
    }

    #[test]
    fn bulk_state_transition_shorns_the_marked_set() {
        let (_dir, farm) = temp_farm(&[
            task("t0", "one", Status::Hairy, 3, None),
            task("t1", "two", Status::Hairy, 3, None),
            task("t2", "three", Status::Hairy, 3, None),
        ]);
        let mut app = App::with_farm(farm).unwrap();
        // Mark all three rows (m, j, m, j, m) -- order-independent.
        press(&mut app, "mjmjm");
        assert_eq!(app.selected.len(), 3);
        // `S` with marks opens the bulk picker; `n` shorns the whole set.
        press(&mut app, "Sn");
        assert!(matches!(app.overlay, Overlay::None));
        for id in ["t0", "t1", "t2"] {
            assert_eq!(app.task(id).unwrap().status, Status::Shorn);
        }
        // The selection is cleared and the count is reported.
        assert!(app.selected.is_empty());
        assert_eq!(app.notification.as_deref(), Some("3/3 → shorn"));
    }

    #[test]
    fn bulk_slaughter_skips_yaks_with_children() {
        // p0 has a live child (c0); t0 is childless. Marking both and bulk-
        // slaughtering must skip the parent (would orphan c0) and only slay
        // the childless one -- mirroring the single-path guard (yaks-5c51).
        let (_dir, farm) = temp_farm(&[
            task("p0", "parent", Status::Hairy, 3, None),
            task("c0", "child", Status::Hairy, 3, Some("p0")),
            task("t0", "solo", Status::Hairy, 3, None),
        ]);
        let mut app = App::with_farm(farm).unwrap();
        // Mark p0 and t0 (skip over the child row c0 between them).
        press(&mut app, "mjjm");
        assert_eq!(app.selected.len(), 2);
        // `S` opens the bulk picker; `x` slaughters the set.
        press(&mut app, "Sx");
        assert!(matches!(app.overlay, Overlay::None));
        // The parent survives (skipped); the childless yak is dead.
        assert_eq!(app.task("p0").unwrap().status, Status::Hairy);
        assert_eq!(app.task("c0").unwrap().status, Status::Hairy);
        assert_eq!(app.task("t0").unwrap().status, Status::Dead);
        assert!(app.selected.is_empty());
        assert_eq!(
            app.notification.as_deref(),
            Some("1/2 → dead · 1 skipped: have children")
        );
    }

    #[test]
    fn detail_pane_mirrors_multi_select_mark() {
        // `m` marks the cursor's yak from the detail pane too (mirrored from
        // the list pane; yaks-5c51). The mark set is shared with the list.
        let (_dir, farm) = temp_farm(&[task("t0", "solo", Status::Hairy, 3, None)]);
        let mut app = App::with_farm(farm).unwrap();
        // `l` focuses the detail pane, then `m` marks t0.
        press(&mut app, "lm");
        assert!(matches!(app.focus, Focus::Detail));
        assert!(app.selected.contains("t0"));
        // `m` again unmarks it.
        press(&mut app, "m");
        assert!(app.selected.is_empty());
    }

    #[test]
    fn priority_pick_updates_and_reloads() {
        let (_dir, farm) = temp_farm(&[task("t0", "solo", Status::Hairy, 3, None)]);
        let mut app = App::with_farm(farm).unwrap();
        press(&mut app, "P1");
        assert_eq!(app.task("t0").unwrap().priority, 1);
        assert_eq!(app.notification.as_deref(), Some("t0 → p1"));
    }

    #[test]
    fn type_pick_updates_and_reloads() {
        let (_dir, farm) = temp_farm(&[task("t0", "solo", Status::Hairy, 3, None)]);
        let mut app = App::with_farm(farm).unwrap();
        press(&mut app, "Tb");
        assert_eq!(app.task("t0").unwrap().kind, "bug");
        assert_eq!(app.notification.as_deref(), Some("t0 → bug"));
    }

    #[test]
    fn slaughter_confirm_moves_to_dead() {
        let (_dir, farm) = temp_farm(&[task("t0", "solo", Status::Hairy, 3, None)]);
        let mut app = App::with_farm(farm).unwrap();
        press(&mut app, "Xy");
        // Dead stays in the model (so deps/ancestors still resolve) but is
        // hidden from the default Hairy view.
        assert_eq!(app.task("t0").unwrap().status, Status::Dead);
        assert!(app.rows().iter().all(|r| r.task.id != "t0"));
        assert_eq!(app.notification.as_deref(), Some("slaughtered t0"));
    }

    #[test]
    fn dead_is_loaded_but_hidden_until_filtered() {
        let (_dir, farm) = temp_farm(&[
            task("t0", "alive", Status::Hairy, 3, None),
            task("t1", "slain", Status::Dead, 3, None),
        ]);
        let mut app = App::with_farm(farm).unwrap();
        // Loaded into the model, but not shown on the default Hairy view.
        assert_eq!(app.task("t1").unwrap().status, Status::Dead);
        assert!(app.rows().iter().all(|r| r.task.id != "t1"));
        // Filtering to Dead surfaces it in the tree.
        app.filter.statuses = vec![Status::Dead];
        let ids: Vec<&str> = app.rows().iter().map(|r| r.task.id.as_str()).collect();
        assert_eq!(ids, vec!["t1"]);
        // Recent (flat) also hides dead by default.
        app.set_view(3);
        assert_eq!(app.active_view().key, "recent");
        assert!(app.rows().iter().all(|r| r.task.id != "t1"));
    }

    #[test]
    fn fuzzy_picker_esc_enters_normal_then_ctrl_c_cancels() {
        let (_dir, farm) = temp_farm(&[
            task("t0", "solo", Status::Hairy, 3, None),
            task("t1", "other", Status::Hairy, 3, None),
        ]);
        let mut app = App::with_farm(farm).unwrap();
        press(&mut app, "D"); // dependency picker on the selected task
        assert!(matches!(app.overlay, Overlay::Fuzzy(_)));
        esc(&mut app); // a lone Esc drops the query to Normal, not cancel
        assert!(matches!(app.overlay, Overlay::Fuzzy(_)));
        if let Overlay::Fuzzy(fp) = &app.overlay {
            assert_eq!(fp.query.borrow().mode, EditorMode::Normal);
        }
        handle_key(
            &mut app,
            KeyEvent::new(KeyCode::Char('c'), KeyModifiers::CONTROL),
        );
        assert!(matches!(app.overlay, Overlay::None));
    }

    #[test]
    fn fuzzy_normal_jk_moves_selection() {
        let (_dir, farm) = temp_farm(&[
            task("t0", "solo", Status::Hairy, 3, None),
            task("t1", "aaa", Status::Hairy, 3, None),
            task("t2", "bbb", Status::Hairy, 3, None),
        ]);
        let mut app = App::with_farm(farm).unwrap();
        press(&mut app, "D"); // dependency picker on t0
        esc(&mut app); // Normal
        let sel0 = match &app.overlay {
            Overlay::Fuzzy(fp) => fp.sel,
            _ => panic!("picker closed"),
        };
        press(&mut app, "j");
        let sel1 = match &app.overlay {
            Overlay::Fuzzy(fp) => fp.sel,
            _ => panic!("picker closed"),
        };
        assert_eq!(sel1, sel0 + 1, "j moves the selection down");
        press(&mut app, "k");
        let sel2 = match &app.overlay {
            Overlay::Fuzzy(fp) => fp.sel,
            _ => panic!("picker closed"),
        };
        assert_eq!(sel2, sel0, "k moves it back up");
    }

    #[test]
    fn slaughter_declined_keeps_task() {
        let (_dir, farm) = temp_farm(&[task("t0", "solo", Status::Hairy, 3, None)]);
        let mut app = App::with_farm(farm).unwrap();
        press(&mut app, "Xn");
        assert_eq!(app.task("t0").unwrap().status, Status::Hairy);
        assert_eq!(app.notification.as_deref(), Some("cancelled"));
    }

    #[test]
    fn state_pick_same_status_is_noop() {
        let (_dir, farm) = temp_farm(&[task("t0", "solo", Status::Hairy, 3, None)]);
        let mut app = App::with_farm(farm).unwrap();
        press(&mut app, "Sh");
        assert_eq!(app.task("t0").unwrap().status, Status::Hairy);
        assert_eq!(app.notification.as_deref(), Some("t0 already hairy"));
    }

    #[test]
    fn create_root_via_form() {
        let (_dir, farm) = temp_farm(&[]);
        let mut app = App::with_farm(farm).unwrap();
        press(&mut app, "c"); // open the create form (title row)
        press(&mut app, "foo"); // type the title
        tab(&mut app); // -> type row
        arrow_right(&mut app); // task -> bug (single-select cursor)
        ctrl_s(&mut app); // create
        assert!(matches!(app.overlay, Overlay::None));
        let created = app.all.iter().find(|t| t.title == "foo").expect("created");
        assert_eq!(created.kind, "bug");
        assert_eq!(created.priority, 3); // default p3
        assert_eq!(created.status, Status::Hairy);
        // The cursor lands on the new task.
        assert_eq!(app.selected().map(|t| t.title.as_str()), Some("foo"));
    }

    #[test]
    fn create_form_sets_priority_and_labels() {
        let (_dir, farm) = temp_farm(&[]);
        let mut app = App::with_farm(farm).unwrap();
        press(&mut app, "c"); // title row
        press(&mut app, "tuned");
        tab(&mut app); // -> type
        tab(&mut app); // -> priority (default p3 == idx 2)
        arrow_left(&mut app); // p3 -> p2
        tab(&mut app); // -> labels
        press(&mut app, "rust, tui");
        ctrl_s(&mut app); // create
        let created = app
            .all
            .iter()
            .find(|t| t.title == "tuned")
            .expect("created");
        assert_eq!(created.priority, 2);
        assert_eq!(created.labels, vec!["rust".to_string(), "tui".to_string()]);
    }

    #[test]
    fn reload_preserving_selection_picks_up_external_add() {
        let (_dir, farm) = temp_farm(&[
            task("t0", "first", Status::Hairy, 3, None),
            task("t1", "second", Status::Hairy, 3, None),
        ]);
        let mut app = App::with_farm(farm).unwrap();
        press(&mut app, "j"); // cursor -> t1
        assert_eq!(app.selected_id().as_deref(), Some("t1"));
        // Simulate an external write to the farm (as the file watcher would
        // observe), then refresh from disk.
        app.farm
            .as_ref()
            .unwrap()
            .create(NewTask {
                title: "third".into(),
                prefix: None,
                kind: Some("task".into()),
                priority: Some(3),
                parent: None,
                labels: vec![],
                depends_on: vec![],
                source: None,
                description: None,
                verify: None,
            })
            .unwrap();
        app.reload_preserving_selection();
        assert!(app.all.iter().any(|t| t.title == "third"), "picked up add");
        assert_eq!(app.selected_id().as_deref(), Some("t1"), "cursor kept");
    }

    #[test]
    fn priority_change_follows_selection_to_new_slot() {
        // yaks-f207: bumping the open yak's priority resorts the list; the
        // index-based cursor must follow it to its new slot rather than
        // point at whatever yak now occupies the old one.
        let (_dir, farm) = temp_farm(&[
            task("t0", "first", Status::Hairy, 1, None),
            task("t1", "second", Status::Hairy, 2, None),
            task("t2", "third", Status::Hairy, 3, None),
        ]);
        let mut app = App::with_farm(farm).unwrap();
        // Roots sort by priority ascending: t0, t1, t2. Cursor is on t0.
        assert_eq!(app.selected_id().as_deref(), Some("t0"));
        press(&mut app, "l"); // open t0 in the detail pane
        assert!(matches!(app.focus, Focus::Detail));
        press(&mut app, "P5"); // t0 -> p5, resorting it last: t1, t2, t0
        assert_eq!(app.task("t0").unwrap().priority, 5);
        // The cursor followed t0 to its new (last) slot.
        assert_eq!(
            app.selected_id().as_deref(),
            Some("t0"),
            "cursor follows the edited yak"
        );
        assert_eq!(app.cursor, 2, "t0 moved to the last slot");
        // Still in the list, so the detail pane stays open.
        assert!(matches!(app.focus, Focus::Detail));
    }

    #[test]
    fn priority_change_out_of_view_closes_detail() {
        // yaks-f207: when a priority change filters the open yak out of the
        // current view, the detail pane -- which tracks a list slot -- must
        // close (human-confirmed drop-out behavior (a): close, not keep).
        let (_dir, farm) = temp_farm(&[
            task("t0", "leaving", Status::Hairy, 1, None),
            task("t1", "survivor", Status::Hairy, 1, None),
        ]);
        let mut app = App::with_farm(farm).unwrap();
        // Constrain the live view to p1; both tasks match, cursor on t0.
        app.filter.priorities = vec![1];
        assert_eq!(app.selected_id().as_deref(), Some("t0"));
        press(&mut app, "l"); // open t0 in the detail pane
        assert!(matches!(app.focus, Focus::Detail));
        press(&mut app, "P5"); // t0 -> p5, dropping it out of the p1 view
        assert_eq!(app.task("t0").unwrap().priority, 5);
        assert!(
            app.rows().iter().all(|r| r.task.id != "t0"),
            "t0 filtered out of the view"
        );
        // Detail closed: focus is back on the list.
        assert!(
            matches!(app.focus, Focus::List),
            "detail closes when the yak drops out"
        );
    }

    #[test]
    fn create_child_sets_parent() {
        let (_dir, farm) = temp_farm(&[task("p0", "parent", Status::Hairy, 3, None)]);
        let mut app = App::with_farm(farm).unwrap();
        press(&mut app, "C"); // create child of the selected task
        press(&mut app, "kid");
        ctrl_s(&mut app); // create (defaults: task, p3)
        let created = app.all.iter().find(|t| t.title == "kid").expect("child");
        assert_eq!(created.parent.as_deref(), Some("p0"));
        assert_eq!(created.kind, "task");
    }

    #[test]
    fn create_form_backtab_from_description_focuses_previous_field() {
        // Regression (yaks-f433): shift-tab (BackTab) while editing the
        // description content block used to forward the key straight to
        // edtui, whose `KeyCode::from` is `unimplemented!` for BackTab and
        // panicked the whole TUI. It must instead focus the previous field.
        let (_dir, farm) = temp_farm(&[]);
        let mut app = App::with_farm(farm).unwrap();
        press(&mut app, "c"); // open the create form (title row)
        tab(&mut app); // -> type
        tab(&mut app); // -> priority
        tab(&mut app); // -> labels
        tab(&mut app); // -> description (content block)
        match &app.overlay {
            Overlay::Create(f) => assert!(f.is_content_row(), "on the description block"),
            _ => panic!("create form open"),
        }
        // Must not panic, and focus moves back to the labels row (3).
        back_tab(&mut app);
        match &app.overlay {
            Overlay::Create(f) => {
                assert_eq!(f.row, 3, "focus moved back a field");
                assert!(f.is_line_text_row(), "back on the labels row");
            }
            _ => panic!("create form still open"),
        }
    }

    #[test]
    fn create_form_drops_unconvertible_editor_key_without_panic() {
        // Belt-and-suspenders: any key edtui can't convert (here Insert)
        // must be dropped rather than forwarded, on both the content and
        // single-line rows. Reaching edtui with it would panic.
        let (_dir, farm) = temp_farm(&[]);
        let mut app = App::with_farm(farm).unwrap();
        press(&mut app, "c"); // title row (single-line text)
        handle_key(&mut app, KeyEvent::new(KeyCode::Insert, KeyModifiers::NONE));
        tab(&mut app); // -> type
        tab(&mut app); // -> priority
        tab(&mut app); // -> labels
        tab(&mut app); // -> description (content block)
        handle_key(&mut app, KeyEvent::new(KeyCode::Insert, KeyModifiers::NONE));
        match &app.overlay {
            Overlay::Create(f) => assert!(f.is_content_row(), "still on the description"),
            _ => panic!("create form still open"),
        }
    }

    #[test]
    fn create_empty_title_ctrl_s_is_noop() {
        // Ctrl-S with an empty title keeps the form open (`(need title)`).
        let (_dir, farm) = temp_farm(&[]);
        let mut app = App::with_farm(farm).unwrap();
        press(&mut app, "c");
        ctrl_s(&mut app);
        assert!(matches!(app.overlay, Overlay::Create(_)));
        assert!(app.all.is_empty());
    }

    #[test]
    fn create_cancelled_with_esc() {
        let (_dir, farm) = temp_farm(&[]);
        let mut app = App::with_farm(farm).unwrap();
        press(&mut app, "c");
        press(&mut app, "foo");
        esc(&mut app); // vim: a lone Esc drops the title field to Normal
        assert!(
            matches!(app.overlay, Overlay::Create(_)),
            "lone Esc is modal"
        );
        esc(&mut app); // rapid double-Esc requests cancel
        // Dirty (typed "foo"), so a discard confirmation appears first.
        assert!(matches!(app.overlay, Overlay::Confirm { .. }));
        press(&mut app, "y"); // confirm discard
        assert!(matches!(app.overlay, Overlay::None));
        assert!(app.all.is_empty());
        assert_eq!(app.notification.as_deref(), Some("changes discarded"));
    }

    #[test]
    fn labels_edit_commits() {
        let (_dir, farm) = temp_farm(&[task("t0", "solo", Status::Hairy, 3, None)]);
        let mut app = App::with_farm(farm).unwrap();
        press(&mut app, "L"); // empty labels field
        press(&mut app, "x, y");
        enter(&mut app);
        assert_eq!(
            app.task("t0").unwrap().labels,
            vec!["x".to_string(), "y".to_string()]
        );
    }

    #[test]
    fn edit_form_updates_description() {
        let (_dir, farm) = temp_farm(&[task("t0", "solo", Status::Hairy, 3, None)]);
        let mut app = App::with_farm(farm).unwrap();
        press(&mut app, "E"); // open the edit form seeded from t0
        tab(&mut app); // title -> type
        tab(&mut app); // -> priority
        tab(&mut app); // -> labels
        tab(&mut app); // -> description content zone
        press(&mut app, "hello");
        ctrl_s(&mut app);
        assert!(matches!(app.overlay, Overlay::None));
        assert_eq!(app.task("t0").unwrap().body, "hello");
        assert_eq!(app.task("t0").unwrap().title, "solo"); // untouched
    }

    #[test]
    fn comment_appends_timestamped_note() {
        let (_dir, farm) = temp_farm(&[task("t0", "solo", Status::Hairy, 3, None)]);
        let mut app = App::with_farm(farm).unwrap();
        press(&mut app, "M"); // open the multi-line comment editor
        press(&mut app, "a helpful note");
        ctrl_s(&mut app);
        assert!(matches!(app.overlay, Overlay::None));
        let body = &app.task("t0").unwrap().body;
        assert!(body.contains("a helpful note"), "note text present");
        assert!(body.contains('\u{25b8}'), "timestamp sigil present");
    }

    #[test]
    fn ask_raises_needs_and_records_the_question() {
        let (_dir, farm) = temp_farm(&[task("t0", "solo", Status::Hairy, 3, None)]);
        let mut app = App::with_farm(farm).unwrap();
        press(&mut app, "a"); // no needs yet -> ask prompt
        press(&mut app, "why is this blocked");
        enter(&mut app); // single-line commit
        assert!(matches!(app.overlay, Overlay::None));
        let t = app.task("t0").unwrap();
        assert_eq!(t.needs.as_deref(), Some("human"));
        assert!(
            t.body.contains("why is this blocked"),
            "question recorded as a note: {}",
            t.body
        );
    }

    #[test]
    fn answer_clears_needs_and_records_the_reply() {
        let mut seed = task("t0", "solo", Status::Hairy, 3, None);
        seed.needs = Some("human".into());
        let (_dir, farm) = temp_farm(&[seed]);
        let mut app = App::with_farm(farm).unwrap();
        press(&mut app, "a"); // needs is set -> answer prompt
        press(&mut app, "resolved offline");
        enter(&mut app);
        let t = app.task("t0").unwrap();
        assert!(t.needs.is_none(), "block cleared");
        assert!(
            t.body.contains("resolved offline"),
            "reply recorded as a note: {}",
            t.body
        );
    }

    #[test]
    fn attach_file_writes_artifact_and_links_body() {
        let (proj, farm) = temp_farm(&[task("t0", "solo", Status::Hairy, 3, None)]);
        let src = proj.join("shot.png");
        fs::write(&src, b"not-really-a-png").unwrap();
        let mut app = App::with_farm(farm).unwrap();
        press(&mut app, "A"); // attach prompt
        press(&mut app, src.to_str().unwrap());
        enter(&mut app); // single-line commit -> attach
        let body = &app.task("t0").unwrap().body;
        assert!(
            body.contains("![shot](artifacts/t0/shot.png)"),
            "body links artifact: {body}"
        );
        assert!(
            proj.join(".yaks/artifacts/t0/shot.png").is_file(),
            "artifact copied into the farm"
        );
    }

    #[test]
    fn edit_form_changes_type_and_priority() {
        let (_dir, farm) = temp_farm(&[task("t0", "solo", Status::Hairy, 3, None)]);
        let mut app = App::with_farm(farm).unwrap();
        press(&mut app, "E");
        tab(&mut app); // -> type
        arrow_right(&mut app); // task -> bug
        tab(&mut app); // -> priority (p3 == idx 2)
        arrow_left(&mut app); // p3 -> p2
        arrow_left(&mut app); // p2 -> p1
        ctrl_s(&mut app);
        let t = app.task("t0").unwrap();
        assert_eq!(t.kind, "bug");
        assert_eq!(t.priority, 1);
    }

    #[test]
    fn edit_form_cancel_leaves_task_unchanged() {
        let (_dir, farm) = temp_farm(&[task("t0", "solo", Status::Hairy, 3, None)]);
        let mut app = App::with_farm(farm).unwrap();
        press(&mut app, "E");
        press(&mut app, "zzz"); // edits the title field in place
        esc(&mut app); // vim: drops the title field to Normal (no cancel)
        esc(&mut app); // rapid double-Esc requests cancel
        assert!(
            matches!(app.overlay, Overlay::Confirm { .. }),
            "dirty -> confirm"
        );
        press(&mut app, "y"); // confirm discard
        assert!(matches!(app.overlay, Overlay::None));
        assert_eq!(app.task("t0").unwrap().title, "solo");
        assert_eq!(app.notification.as_deref(), Some("changes discarded"));
    }

    #[test]
    fn edit_form_edits_a_comment_in_place() {
        let (_dir, farm) = temp_farm(&[task("t0", "solo", Status::Hairy, 3, None)]);
        let mut app = App::with_farm(farm).unwrap();
        press(&mut app, "M"); // add a comment
        press(&mut app, "original note");
        ctrl_s(&mut app);
        let before = app.task("t0").unwrap().body.clone();
        assert!(before.contains('\u{25b8}'));
        // Edit: walk title→type→priority→labels→description→comment. Tab is
        // now a normal key inside content blocks, so field-nav uses Ctrl-N.
        // The seeded comment opens in Normal (921a), so `i` to insert.
        press(&mut app, "E");
        for _ in 0..5 {
            ctrl_n(&mut app);
        }
        press(&mut app, "iX");
        ctrl_s(&mut app);
        let body = &app.task("t0").unwrap().body;
        assert!(
            body.contains("Xoriginal note"),
            "comment edited in place: {body:?}"
        );
        assert!(body.contains('\u{25b8}'), "timestamp preserved: {body:?}");
    }

    #[test]
    fn edit_form_noop_preserves_comment_body() {
        let (_dir, farm) = temp_farm(&[task("t0", "solo", Status::Hairy, 3, None)]);
        let mut app = App::with_farm(farm).unwrap();
        press(&mut app, "M");
        press(&mut app, "keep me");
        ctrl_s(&mut app);
        let before = app.task("t0").unwrap().body.clone();
        press(&mut app, "E");
        ctrl_s(&mut app); // no edits: parse→assemble must round-trip losslessly
        assert_eq!(app.task("t0").unwrap().body, before);
    }

    #[test]
    fn edit_form_deletes_an_emptied_comment() {
        let (_dir, farm) = temp_farm(&[task("t0", "solo", Status::Hairy, 3, None)]);
        let mut app = App::with_farm(farm).unwrap();
        press(&mut app, "M");
        press(&mut app, "note to remove");
        ctrl_s(&mut app);
        assert!(app.task("t0").unwrap().body.contains('\u{25b8}'));
        let vim = app.editor_vim;
        press(&mut app, "E");
        match &mut app.overlay {
            Overlay::Create(f) => f.blocks[1].editor = multiline_field("", vim),
            _ => panic!("expected edit form"),
        }
        ctrl_s(&mut app);
        let body = &app.task("t0").unwrap().body;
        assert!(!body.contains('\u{25b8}'), "comment deleted: {body:?}");
        assert!(
            body.trim().is_empty(),
            "body empty after deletion: {body:?}"
        );
    }

    #[test]
    fn lone_esc_stays_in_the_comment_editor() {
        let (_dir, farm) = temp_farm(&[task("t0", "solo", Status::Hairy, 3, None)]);
        let mut app = App::with_farm(farm).unwrap();
        press(&mut app, "M");
        press(&mut app, "draft");
        esc(&mut app); // multiline: Esc drops to Normal, does NOT cancel
        assert!(matches!(app.overlay, Overlay::Edit(_)), "editor stays open");
    }

    #[test]
    fn double_esc_cancels_the_comment_editor() {
        let (_dir, farm) = temp_farm(&[task("t0", "solo", Status::Hairy, 3, None)]);
        let mut app = App::with_farm(farm).unwrap();
        press(&mut app, "M");
        press(&mut app, "draft");
        esc(&mut app); // -> Normal
        esc(&mut app); // rapid second Esc == Ctrl-C -> request cancel
        // The draft is dirty, so a discard confirmation appears first.
        assert!(matches!(app.overlay, Overlay::Confirm { .. }));
        press(&mut app, "y"); // confirm discard
        assert!(matches!(app.overlay, Overlay::None));
        assert_eq!(app.notification.as_deref(), Some("changes discarded"));
        assert!(app.task("t0").unwrap().body.is_empty(), "comment not saved");
    }

    #[test]
    fn double_esc_cancels_the_edit_form_from_a_content_row() {
        let (_dir, farm) = temp_farm(&[task("t0", "solo", Status::Hairy, 3, None)]);
        let mut app = App::with_farm(farm).unwrap();
        press(&mut app, "E");
        for _ in 0..4 {
            tab(&mut app); // title -> ... -> description (a content row)
        }
        esc(&mut app); // content row: Esc drops to Normal, form stays
        assert!(matches!(app.overlay, Overlay::Create(_)), "form stays open");
        esc(&mut app); // rapid second Esc cancels the whole edit
        assert!(matches!(app.overlay, Overlay::None));
        assert_eq!(app.notification.as_deref(), Some("edit cancelled"));
    }

    #[test]
    fn declining_the_discard_confirm_returns_to_the_editor() {
        let (_dir, farm) = temp_farm(&[task("t0", "solo", Status::Hairy, 3, None)]);
        let mut app = App::with_farm(farm).unwrap();
        press(&mut app, "c");
        press(&mut app, "foo");
        esc(&mut app);
        esc(&mut app); // dirty -> discard confirm
        assert!(matches!(app.overlay, Overlay::Confirm { .. }));
        press(&mut app, "n"); // decline -> back to the form, work intact
        match &app.overlay {
            Overlay::Create(f) => assert_eq!(f.title_text(), "foo", "draft preserved"),
            _ => panic!("expected the form to be restored"),
        }
    }

    #[test]
    fn clean_cancel_skips_the_confirm() {
        let (_dir, farm) = temp_farm(&[task("t0", "solo", Status::Hairy, 3, None)]);
        let mut app = App::with_farm(farm).unwrap();
        press(&mut app, "E"); // open edit, make no changes
        handle_key(
            &mut app,
            KeyEvent::new(KeyCode::Char('c'), KeyModifiers::CONTROL),
        );
        assert!(
            matches!(app.overlay, Overlay::None),
            "clean cancel is immediate"
        );
        assert_eq!(app.notification.as_deref(), Some("edit cancelled"));
    }

    #[test]
    fn colon_wq_commits_the_comment() {
        let (_dir, farm) = temp_farm(&[task("t0", "solo", Status::Hairy, 3, None)]);
        let mut app = App::with_farm(farm).unwrap();
        press(&mut app, "M");
        press(&mut app, "a note");
        esc(&mut app); // Normal
        press(&mut app, ":wq"); // open cmdline + type
        enter(&mut app); // run
        assert!(matches!(app.overlay, Overlay::None));
        assert!(app.task("t0").unwrap().body.contains("a note"));
    }

    #[test]
    fn colon_q_bang_discards_the_comment() {
        let (_dir, farm) = temp_farm(&[task("t0", "solo", Status::Hairy, 3, None)]);
        let mut app = App::with_farm(farm).unwrap();
        press(&mut app, "M");
        press(&mut app, "draft");
        esc(&mut app); // Normal
        press(&mut app, ":q!"); // force-quit, no discard confirm
        enter(&mut app);
        assert!(matches!(app.overlay, Overlay::None));
        assert!(app.task("t0").unwrap().body.is_empty());
    }

    #[test]
    fn colon_w_saves_the_edit_form() {
        let (_dir, farm) = temp_farm(&[task("t0", "solo", Status::Hairy, 3, None)]);
        let mut app = App::with_farm(farm).unwrap();
        press(&mut app, "E");
        for _ in 0..4 {
            tab(&mut app); // -> description content row (empty -> Insert)
        }
        press(&mut app, "hello");
        esc(&mut app); // Normal
        press(&mut app, ":w");
        enter(&mut app);
        assert!(matches!(app.overlay, Overlay::None));
        assert_eq!(app.task("t0").unwrap().body, "hello");
    }

    #[test]
    fn colon_unknown_command_notifies_and_stays_open() {
        let (_dir, farm) = temp_farm(&[task("t0", "solo", Status::Hairy, 3, None)]);
        let mut app = App::with_farm(farm).unwrap();
        press(&mut app, "M");
        esc(&mut app); // empty comment -> Normal
        press(&mut app, ":nope");
        enter(&mut app);
        assert!(matches!(app.overlay, Overlay::Edit(_)), "editor stays open");
        assert_eq!(app.notification.as_deref(), Some("unknown command: :nope"));
    }

    fn with_deps(id: &str, status: Status, deps: &[&str]) -> Task {
        let mut t = task(id, id, status, 3, None);
        t.depends_on = deps.iter().map(|s| s.to_string()).collect();
        t
    }

    #[test]
    fn dep_add_via_picker() {
        let (_dir, farm) = temp_farm(&[
            task("t0", "t0", Status::Hairy, 3, None),
            task("t1", "t1", Status::Hairy, 3, None),
        ]);
        let mut app = App::with_farm(farm).unwrap();
        assert_eq!(app.selected_id().as_deref(), Some("t0"));
        press(&mut app, "D"); // open picker (t0 excluded)
        enter(&mut app); // pick first candidate (t1)
        assert!(matches!(app.overlay, Overlay::None));
        assert_eq!(app.task("t0").unwrap().depends_on, vec!["t1".to_string()]);
        assert_eq!(app.notification.as_deref(), Some("t0 depends on t1"));
    }

    #[test]
    fn dep_cycle_target_is_excluded() {
        // t1 already depends on t0, so t0 -> t1 would cycle: t1 is not offered.
        let (_dir, farm) = temp_farm(&[
            task("t0", "t0", Status::Hairy, 3, None),
            with_deps("t1", Status::Hairy, &["t0"]),
        ]);
        let mut app = App::with_farm(farm).unwrap();
        assert_eq!(app.selected_id().as_deref(), Some("t0"));
        press(&mut app, "D");
        enter(&mut app); // no candidates -> nothing selected
        assert!(app.task("t0").unwrap().depends_on.is_empty());
        assert_eq!(app.notification.as_deref(), Some("nothing selected"));
    }

    #[test]
    fn reparent_via_picker() {
        let (_dir, farm) = temp_farm(&[
            task("p0", "p0", Status::Hairy, 3, None),
            task("t0", "t0", Status::Hairy, 3, None),
        ]);
        let mut app = App::with_farm(farm).unwrap();
        press(&mut app, "j"); // move to t0
        assert_eq!(app.selected_id().as_deref(), Some("t0"));
        press(&mut app, "R");
        enter(&mut app); // pick first candidate (p0)
        assert_eq!(app.task("t0").unwrap().parent.as_deref(), Some("p0"));
        assert_eq!(app.notification.as_deref(), Some("t0 reparented under p0"));
    }

    #[test]
    fn reparent_clear_to_root() {
        let (_dir, farm) = temp_farm(&[
            task("p0", "p0", Status::Hairy, 3, None),
            task("c0", "c0", Status::Hairy, 3, Some("p0")),
        ]);
        let mut app = App::with_farm(farm).unwrap();
        press(&mut app, "j"); // move to c0 (child of p0)
        assert_eq!(app.selected_id().as_deref(), Some("c0"));
        press(&mut app, "R"); // p0 excluded (current parent); only clear-parent row
        enter(&mut app);
        assert!(app.task("c0").unwrap().parent.is_none());
        assert_eq!(app.notification.as_deref(), Some("c0 moved to top level"));
    }
}

// -- ephemeral status line placement ----------------------------------

#[test]
fn status_notification_drops_below_tabs_when_it_would_overlap() {
    // The ephemeral yellow status line lives right-aligned on the tab row,
    // but must not scribble over the tabs. At a narrow width it can't fit
    // beside them, so it drops to the blank gap row directly below the tabs;
    // at a wide width it stays inline on the tab row. This is a pure
    // render-time decision, so the same App re-lays-out on width alone.
    const NOTE: &str = "quarterly-planning";
    let mut app = sample();
    app.notification = Some("saved view: quarterly-planning".into());

    // Narrow: no room beside the tabs -> dropped to the gap row (row 1).
    let narrow = draw(&app, 90, 8);
    let nlines: Vec<&str> = narrow.lines().collect();
    assert!(
        !nlines[0].contains(NOTE),
        "narrow: the tab row must not carry the notification\n{narrow}"
    );
    assert!(
        nlines[1].contains(NOTE),
        "narrow: the notification drops to the blank row below the tabs\n{narrow}"
    );
    insta::assert_snapshot!("status_notification_below_tabs_narrow", narrow);

    // Wide: fits beside the tabs -> stays inline on the tab row (row 0),
    // leaving the gap row blank.
    let wide = draw(&app, 160, 8);
    let wlines: Vec<&str> = wide.lines().collect();
    assert!(
        wlines[0].contains(NOTE),
        "wide: the notification stays inline on the tab row\n{wide}"
    );
    assert!(
        !wlines[1].contains(NOTE),
        "wide: the gap row below the tabs stays blank\n{wide}"
    );
    insta::assert_snapshot!("status_notification_inline_wide", wide);
}
