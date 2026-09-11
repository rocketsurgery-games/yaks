//! The fuzzy task picker (add-dep / reparent / ref-autocomplete) and the inline
//! search box (list search + detail find). Split out of `tui.rs`
//! (yaks-b1cc / b1cc/4). Both are single-line edtui editors; App owns their key
//! handling and the render layer paints their query lines via
//! `editor::render_query_line`, so only the types, their constructors, the
//! candidate ranking, and the results list live here.

use std::cell::RefCell;
use std::collections::HashSet;

use edtui::{EditorEventHandler, EditorMode, EditorState, Lines};
use ratatui::Frame;
use ratatui::layout::{Constraint, Layout, Rect};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{List, ListItem, ListState, Paragraph};

use super::{App, Overlay, make_handler};
use crate::model::Task;

/// A filter-as-you-type picker over the task set. The query is an edtui
/// single-line editor; candidates are ranked substring matches (Python's
/// `fuzzy_pick_task` semantics). `RefCell` for the same render-purity reason.
pub(crate) struct FuzzyPick {
    pub(crate) label: String,
    pub(crate) query: RefCell<EditorState>,
    pub(crate) handler: EditorEventHandler,
    /// Ids never offered (self, existing deps/parents, cycle- or loop-forming).
    pub(crate) exclude: HashSet<String>,
    /// When true, a synthetic top row clears the parent (reparent to root).
    pub(crate) allow_none: bool,
    pub(crate) sel: usize,
    pub(crate) action: FuzzyAction,
    /// The overlay to restore when this picker closes (commit or cancel). Set
    /// when the picker is opened *over* another overlay — e.g. the editor that
    /// ref-autocomplete inserts back into — so closing returns there instead of
    /// to the bare board.
    pub(crate) return_to: Option<Box<Overlay>>,
    /// Chars of the already-typed partial token (e.g. `yaks-`) to delete from the
    /// editor before inserting the chosen id, so autocomplete doesn't duplicate
    /// the prefix. Only meaningful for [`FuzzyAction::InsertRef`].
    pub(crate) replace_len: usize,
}

pub(crate) enum FuzzyAction {
    AddDep(String),
    Reparent(String),
    /// Insert the picked task's id at the cursor of the editor stashed in
    /// `FuzzyPick::return_to` (ref autocomplete while editing).
    InsertRef,
}

/// Inline incremental search box. Editing it updates `App.filter.search` on
/// every keystroke (live preview); Esc restores the pre-search query.
pub(crate) struct SearchBox {
    pub(crate) query: RefCell<EditorState>,
    pub(crate) handler: EditorEventHandler,
    /// The `filter.search` value before opening, restored on cancel.
    pub(crate) saved: Option<String>,
    /// Detail-find only: `(detail_scroll, detail_line)` captured when the find
    /// opened, restored on cancel so Esc returns to the pre-find position
    /// (vi-like). `None` for the list search box.
    pub(crate) detail_origin: Option<(u16, usize)>,
}

impl SearchBox {
    pub(crate) fn new(vim: bool, initial: Option<String>) -> Self {
        let seed = initial.clone().unwrap_or_default();
        let mut st = EditorState::new(Lines::from(seed.as_str()));
        st.set_single_line(true);
        st.mode = EditorMode::Insert;
        SearchBox {
            query: RefCell::new(st),
            handler: make_handler(vim),
            saved: initial,
            detail_origin: None,
        }
    }

    pub(crate) fn query_text(&self) -> String {
        self.query.borrow().lines.to_string()
    }
}

impl FuzzyPick {
    pub(crate) fn new(
        vim: bool,
        label: String,
        exclude: HashSet<String>,
        allow_none: bool,
        action: FuzzyAction,
    ) -> Self {
        let mut st = EditorState::new(Lines::from(""));
        st.set_single_line(true);
        st.mode = EditorMode::Insert;
        FuzzyPick {
            label,
            query: RefCell::new(st),
            handler: make_handler(vim),
            exclude,
            allow_none,
            sel: 0,
            action,
            return_to: None,
            replace_len: 0,
        }
    }

    pub(crate) fn query_text(&self) -> String {
        self.query.borrow().lines.to_string()
    }
}

/// Ranked substring matches over `all`, honoring the picker's exclude set and
/// query. Empty query lists everything (capped). Score: id-prefix < id-substr
/// < title-substr, then priority, then id.
pub(crate) fn fuzzy_candidates<'a>(all: &'a [Task], fp: &FuzzyPick) -> Vec<&'a Task> {
    let q = fp.query_text().to_lowercase();
    let mut scored: Vec<(u8, u8, &Task)> = Vec::new();
    for t in all {
        if fp.exclude.contains(&t.id) {
            continue;
        }
        let score = if q.is_empty() {
            0
        } else {
            let tid = t.id.to_lowercase();
            let title = t.title.to_lowercase();
            if tid.starts_with(&q) {
                0
            } else if tid.contains(&q) {
                1
            } else if title.contains(&q) {
                2
            } else {
                continue;
            }
        };
        scored.push((score, t.priority, t));
    }
    scored.sort_by(|a, b| {
        a.0.cmp(&b.0)
            .then(a.1.cmp(&b.1))
            .then_with(|| a.2.id.cmp(&b.2.id))
    });
    scored.into_iter().take(20).map(|(_, _, t)| t).collect()
}

/// Number of selectable rows (candidates plus the optional clear-parent row).
pub(crate) fn fuzzy_total(all: &[Task], fp: &FuzzyPick) -> usize {
    fuzzy_candidates(all, fp).len() + fp.allow_none as usize
}

pub(crate) fn render_fuzzy_results(app: &App, fp: &FuzzyPick, frame: &mut Frame, area: Rect) {
    let [head, body] = Layout::vertical([Constraint::Length(1), Constraint::Min(0)]).areas(area);
    let cands = fuzzy_candidates(&app.all, fp);
    frame.render_widget(
        Paragraph::new(Span::styled(
            format!("{}  ({} matches)", fp.label, cands.len()),
            Style::new().fg(Color::Cyan).add_modifier(Modifier::BOLD),
        )),
        head,
    );
    let mut items: Vec<ListItem> = Vec::new();
    if fp.allow_none {
        items.push(ListItem::new(Line::from(Span::styled(
            "(clear parent — make top-level)",
            Style::new().fg(Color::DarkGray),
        ))));
    }
    for t in &cands {
        // Status emoji (matching the list, tab bar and detail pane) rather than
        // a bracketed letter glyph.
        items.push(ListItem::new(Line::from(vec![
            Span::raw(format!("{}  ", t.status.emoji())),
            Span::styled(format!("{} ", t.id), Style::new().fg(Color::DarkGray)),
            Span::raw(t.title.clone()),
        ])));
    }
    let total = cands.len() + fp.allow_none as usize;
    let mut state = ListState::default();
    if total > 0 {
        state.select(Some(fp.sel.min(total - 1)));
    }
    frame.render_stateful_widget(
        List::new(items).highlight_style(Style::new().bg(Color::Indexed(237))),
        body,
        &mut state,
    );
}
