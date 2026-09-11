//! Embedded text editors (edtui glue) and their rendering.
//!
//! Everything here reduces to an `EditorEventHandler` + a `RefCell<EditorState>`:
//! the standalone `Editor` overlay (comments, labels, ask/answer, …) and the
//! create-form content blocks both build on the same primitives. Split out of
//! `tui.rs` (yaks-b1cc / b1cc/2).

use std::cell::RefCell;

use edtui::{EditorEventHandler, EditorMode, EditorState, EditorTheme, EditorView, Lines};
use ratatui::Frame;
use ratatui::crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use ratatui::layout::{Constraint, Layout, Rect};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Paragraph};

use super::{disp_width, markdown};

/// `EditorState` is `RefCell`-wrapped because `EditorView` needs `&mut` at
/// render time, while our `render(&App, ..)` borrows the app immutably.
pub(crate) struct Editor {
    pub(crate) state: RefCell<EditorState>,
    pub(crate) handler: EditorEventHandler,
    pub(crate) single_line: bool,
    /// Whether this editor uses the vim keybinding profile. When true, `Esc`
    /// leaves Insert for Normal mode instead of closing the overlay, so the
    /// full normal-mode keymap is reachable even in single-line fields.
    pub(crate) vim: bool,
    /// Prompt label (bottom-line prefix for fields; header for the body panel).
    pub(crate) label: String,
    pub(crate) action: EditAction,
}

pub(crate) enum EditAction {
    Labels(String),
    Comment(String),
    Attach(String),
    SaveView,
    RenameView {
        index: usize,
    },
    /// Raise a `needs` block on this id (default `human`) and record the typed
    /// question as an attributed note. TUI counterpart of CLI `ask`.
    Ask(String),
    /// Clear the `needs` block on this id and record the typed reply. TUI
    /// counterpart of CLI `answer`.
    Answer(String),
}

/// What a context-sensitive `E` in the detail pane should edit, derived from
/// the line the cursor sits on.
#[derive(Clone, Copy)]
pub(crate) enum EditTarget {
    Title,
    Type,
    Priority,
    Labels,
    Status,
    /// A content block: `0` = description, `1..` = comments.
    Content(usize),
}

pub(crate) fn make_handler(vim: bool) -> EditorEventHandler {
    if vim {
        EditorEventHandler::vim_mode()
    } else {
        EditorEventHandler::emacs_mode()
    }
}

impl Editor {
    pub(crate) fn new(
        vim: bool,
        single_line: bool,
        label: String,
        initial: &str,
        action: EditAction,
    ) -> Self {
        let mut state = EditorState::new(Lines::from(initial));
        state.set_single_line(single_line);
        // Vim users expect to land in Normal when reviewing existing multiline
        // content; an empty editor (and every single-line field) opens in Insert
        // so you can type immediately.
        state.mode = if vim && !single_line && !initial.is_empty() {
            EditorMode::Normal
        } else {
            EditorMode::Insert
        };
        Editor {
            state: RefCell::new(state),
            handler: make_handler(vim),
            single_line,
            vim,
            label,
            action,
        }
    }

    pub(crate) fn text(&self) -> String {
        self.state.borrow().lines.to_string()
    }
}

/// Insert `s` at an edtui cursor by replaying it as Insert-mode keystrokes, so
/// edtui advances the cursor and records undo exactly as if the user typed it.
/// Works on any embedded editor (the comment `Editor` and the create-form
/// content blocks both reduce to a handler + a `RefCell<EditorState>`).
pub(crate) fn insert_into(handler: &mut EditorEventHandler, state: &RefCell<EditorState>, s: &str) {
    state.borrow_mut().mode = EditorMode::Insert;
    for c in s.chars() {
        let k = KeyEvent::new(KeyCode::Char(c), KeyModifiers::NONE);
        handler.on_key_event(k, &mut state.borrow_mut());
    }
}

/// Delete the `n` chars before the cursor by replaying Backspace. Used to drop
/// the typed `<prefix>-` before an autocompleted id is inserted in its place.
pub(crate) fn delete_into(
    handler: &mut EditorEventHandler,
    state: &RefCell<EditorState>,
    n: usize,
) {
    state.borrow_mut().mode = EditorMode::Insert;
    for _ in 0..n {
        let k = KeyEvent::new(KeyCode::Backspace, KeyModifiers::NONE);
        handler.on_key_event(k, &mut state.borrow_mut());
    }
}

/// Whether edtui can convert this key without panicking. edtui's
/// `KeyCode: From<crossterm::event::KeyCode>` is `unimplemented!` for anything
/// outside this whitelist (e.g. `BackTab`, `Insert`, the F-keys), so forwarding
/// such a key would panic the whole TUI. Guard every edtui forward with this.
pub(crate) fn edtui_can_handle(code: KeyCode) -> bool {
    matches!(
        code,
        KeyCode::Char(_)
            | KeyCode::Enter
            | KeyCode::Esc
            | KeyCode::Backspace
            | KeyCode::Delete
            | KeyCode::Tab
            | KeyCode::Left
            | KeyCode::Right
            | KeyCode::Up
            | KeyCode::Down
            | KeyCode::Home
            | KeyCode::End
            | KeyCode::PageUp
            | KeyCode::PageDown
    )
}

/// The run of ref-chars ending at the cursor — the partial yak id being typed —
/// as `(token, char_len)`.
pub(crate) fn token_before_cursor(state: &RefCell<EditorState>) -> (String, usize) {
    let st = state.borrow();
    let full = st.lines.to_string();
    let line = full.split('\n').nth(st.cursor.row).unwrap_or("");
    let before: Vec<char> = line.chars().take(st.cursor.col).collect();
    let mut i = before.len();
    while i > 0 && crate::refs::is_ref_char(before[i - 1]) {
        i -= 1;
    }
    let tok: String = before[i..].iter().collect();
    let len = before.len() - i;
    (tok, len)
}

/// If the ref-token just before the cursor is a `<prefix>-…` completion context,
/// return `(chars_to_replace, tail)` — the whole `<prefix>-…` length to drop on
/// commit, and the text typed after `<prefix>-` to seed the picker query with.
pub(crate) fn completion_context(
    state: &RefCell<EditorState>,
    prefix: &str,
) -> Option<(usize, String)> {
    let (tok, len) = token_before_cursor(state);
    tok.strip_prefix(&format!("{prefix}-"))
        .map(|tail| (len, tail.to_string()))
}

/// Short mode label shown next to a vim editor so the current mode is visible.
pub(crate) fn mode_tag(mode: EditorMode) -> &'static str {
    match mode {
        EditorMode::Normal => "NORMAL",
        EditorMode::Insert => "INSERT",
        EditorMode::Visual => "VISUAL",
        EditorMode::Search => "SEARCH",
    }
}

pub(crate) fn mode_style(mode: EditorMode) -> Style {
    let color = match mode {
        EditorMode::Normal => Color::Green,
        EditorMode::Insert => Color::Yellow,
        EditorMode::Visual => Color::Magenta,
        EditorMode::Search => Color::Cyan,
    };
    Style::new().fg(color).add_modifier(Modifier::BOLD)
}

/// Recompute markdown highlights for a multi-line editor buffer and stash them
/// on the state so edtui paints them at render (Normal *and* Insert), in logical
/// coords so they survive edtui's own wrapping. Our own hand-rolled highlighter
/// — no syntect, no C deps. Cheap enough to redo every frame.
pub(crate) fn set_md_highlights(state: &mut EditorState) {
    let text = state.lines.to_string();
    let mut hl = markdown::Highlighter::new();
    let mut highlights = Vec::new();
    for (row, line) in text.split('\n').enumerate() {
        for sp in hl.line(line) {
            if sp.len == 0 {
                continue;
            }
            highlights.push(edtui::Highlight::new(
                edtui::Index2::new(row, sp.start),
                edtui::Index2::new(row, sp.start + sp.len - 1),
                sp.style,
            ));
        }
    }
    state.set_highlights(highlights);
}

/// Theme for embedded editors. The cursor cell is styled per mode so Normal vs
/// Insert is visible even without a real hardware cursor shape: a solid block
/// in Normal/Visual, an underline (bar-like) in Insert.
pub(crate) fn editor_theme(mode: EditorMode) -> EditorTheme<'static> {
    let cursor = match mode {
        EditorMode::Insert => Style::new().add_modifier(Modifier::UNDERLINED),
        _ => Style::new().bg(Color::White).fg(Color::Black),
    };
    EditorTheme::default()
        .hide_status_line()
        .block(Block::default())
        .cursor_style(cursor)
}

pub(crate) fn render_editor_panel(ed: &Editor, frame: &mut Frame, area: Rect) {
    let [head, body] = Layout::vertical([Constraint::Length(1), Constraint::Min(0)]).areas(area);
    let mode = ed.state.borrow().mode;
    let label_style = Style::new().fg(Color::Cyan).add_modifier(Modifier::BOLD);
    // Header: prompt label on the left, a mode tag reserved at the right edge
    // (vim only) so it stays visible even when the label is long.
    let tag = mode_tag(mode);
    let tag_w = (disp_width(tag) + 1) as u16;
    if ed.vim && head.width > tag_w {
        let [label_area, tag_area] =
            Layout::horizontal([Constraint::Min(0), Constraint::Length(tag_w)]).areas(head);
        frame.render_widget(
            Paragraph::new(Span::styled(ed.label.clone(), label_style)),
            label_area,
        );
        frame.render_widget(
            Paragraph::new(Line::from(vec![
                Span::raw(" "),
                Span::styled(tag, mode_style(mode)),
            ])),
            tag_area,
        );
    } else {
        frame.render_widget(
            Paragraph::new(Span::styled(ed.label.clone(), label_style)),
            head,
        );
    }
    let mut state = ed.state.borrow_mut();
    set_md_highlights(&mut state);
    let view = EditorView::new(&mut state).theme(editor_theme(mode));
    frame.render_widget(view, body);
}

/// Render `label` + a single-line edtui field across one row.
pub(crate) fn render_query_line(
    label: &str,
    state: &RefCell<EditorState>,
    frame: &mut Frame,
    area: Rect,
) {
    let label_w = (label.chars().count() as u16).min(area.width);
    let [lab, fld] =
        Layout::horizontal([Constraint::Length(label_w), Constraint::Min(0)]).areas(area);
    frame.render_widget(
        Paragraph::new(Span::styled(
            label.to_string(),
            Style::new().fg(Color::Yellow).add_modifier(Modifier::BOLD),
        )),
        lab,
    );
    let mut st = state.borrow_mut();
    let mode = st.mode;
    frame.render_widget(
        EditorView::new(&mut st)
            .theme(editor_theme(mode))
            .single_line(true),
        fld,
    );
}

pub(crate) fn render_line_editor(ed: &Editor, frame: &mut Frame, area: Rect) {
    let mode = ed.state.borrow().mode;
    // Reserve room at the right for a mode tag (vim only), so a single-line
    // field also shows Normal vs Insert now that Normal mode is reachable there.
    if ed.vim {
        let tag = mode_tag(mode);
        let tag_w = (disp_width(tag) + 1) as u16;
        if area.width > tag_w {
            let [main, tag_area] =
                Layout::horizontal([Constraint::Min(0), Constraint::Length(tag_w)]).areas(area);
            render_query_line(&ed.label, &ed.state, frame, main);
            frame.render_widget(
                Paragraph::new(Line::from(vec![
                    Span::raw(" "),
                    Span::styled(tag, mode_style(mode)),
                ])),
                tag_area,
            );
            return;
        }
    }
    render_query_line(&ed.label, &ed.state, frame, area);
}
