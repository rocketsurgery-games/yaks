//! The filter drawer: chip facets (status/type/priority/deps) + text facets
//! (labels/search/parent). Split out of `tui.rs` (yaks-b1cc / b1cc/3).
//!
//! Shared form helpers (`text_field`, `toggle`, the `*_CHOICES` consts,
//! `render_chip_row`/`render_text_row`) stay in `tui.rs` and are reached via
//! `super::`, since the create/edit form uses them too.

use std::cell::RefCell;

use edtui::{EditorEventHandler, EditorMode, EditorState, Lines};
use ratatui::Frame;
use ratatui::layout::{Constraint, Layout, Rect};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::Span;
use ratatui::widgets::Paragraph;

use super::{
    DEPS_CHOICES, DRAWER_LABEL_W, PRI_CHOICES, STATUS_CHOICES, TYPE_CHOICES, clone_spec,
    make_handler, render_chip_row, render_text_row, status_word, text_field, toggle,
};
use crate::filter::FilterSpec;
use crate::model::Status;

/// The filter drawer: a small form of chip facets (status/type/priority/deps)
/// and text facets (labels/search/parent). Editing it previews live on the
/// list; Enter applies, Esc reverts to `saved`. Reproduces Python `_DrawerState`.
pub(crate) struct Drawer {
    pub(crate) saved: FilterSpec,
    pub(crate) statuses: Vec<Status>,
    pub(crate) types: Vec<String>,
    pub(crate) priorities: Vec<u8>,
    pub(crate) ready: bool,
    pub(crate) tangled: bool,
    /// The inbox facet: keep only yaks carrying a `needs` block. Composes with
    /// the rest of the filter (replaces the old modal `i` toggle).
    pub(crate) needs: bool,
    pub(crate) labels: RefCell<EditorState>,
    pub(crate) search: RefCell<EditorState>,
    pub(crate) parent: RefCell<EditorState>,
    pub(crate) handler: EditorEventHandler,
    pub(crate) row: usize,
    pub(crate) chip_idx: usize,
}

impl Drawer {
    pub(crate) fn from_filter(vim: bool, f: &FilterSpec) -> Self {
        Drawer {
            saved: clone_spec(f),
            statuses: f.statuses.clone(),
            types: f.types.clone(),
            priorities: f.priorities.clone(),
            ready: f.ready_only,
            tangled: f.tangled_only,
            needs: f.needs_only,
            labels: text_field(&f.labels.join(", "), vim),
            search: text_field(f.search.as_deref().unwrap_or(""), vim),
            parent: text_field(f.parent.as_deref().unwrap_or(""), vim),
            handler: make_handler(vim),
            row: 0,
            chip_idx: 0,
        }
    }

    pub(crate) fn is_text_row(&self) -> bool {
        matches!(self.row, 3 | 4 | 5)
    }

    /// The editor backing the current text row (labels/search/parent), if on one.
    pub(crate) fn text_editor(&self) -> Option<&RefCell<EditorState>> {
        match self.row {
            3 => Some(&self.labels),
            4 => Some(&self.search),
            5 => Some(&self.parent),
            _ => None,
        }
    }

    pub(crate) fn chip_count(&self) -> usize {
        match self.row {
            0 => STATUS_CHOICES.len(),
            1 => TYPE_CHOICES.len(),
            2 => PRI_CHOICES.len(),
            6 => DEPS_CHOICES.len(),
            _ => 0,
        }
    }

    pub(crate) fn toggle_chip(&mut self) {
        match self.row {
            0 => toggle(&mut self.statuses, STATUS_CHOICES[self.chip_idx]),
            1 => toggle(&mut self.types, TYPE_CHOICES[self.chip_idx].to_string()),
            2 => toggle(&mut self.priorities, PRI_CHOICES[self.chip_idx]),
            6 => match self.chip_idx {
                0 => self.ready = !self.ready,
                1 => self.tangled = !self.tangled,
                _ => self.needs = !self.needs,
            },
            _ => {}
        }
    }

    pub(crate) fn clear(&mut self) {
        self.statuses.clear();
        self.types.clear();
        self.priorities.clear();
        self.ready = false;
        self.tangled = false;
        self.needs = false;
        *self.labels.borrow_mut() = {
            let mut s = EditorState::new(Lines::from(""));
            s.set_single_line(true);
            s.mode = EditorMode::Insert;
            s
        };
        *self.search.borrow_mut() = {
            let mut s = EditorState::new(Lines::from(""));
            s.set_single_line(true);
            s.mode = EditorMode::Insert;
            s
        };
        *self.parent.borrow_mut() = {
            let mut s = EditorState::new(Lines::from(""));
            s.set_single_line(true);
            s.mode = EditorMode::Insert;
            s
        };
    }

    fn text_of(cell: &RefCell<EditorState>) -> String {
        cell.borrow().lines.to_string()
    }

    pub(crate) fn build_spec(&self) -> FilterSpec {
        let labels: Vec<String> = Self::text_of(&self.labels)
            .split(',')
            .map(|s| s.trim())
            .filter(|s| !s.is_empty())
            .map(String::from)
            .collect();
        let non_empty = |cell: &RefCell<EditorState>| {
            let s = Self::text_of(cell).trim().to_string();
            if s.is_empty() { None } else { Some(s) }
        };
        FilterSpec {
            statuses: self.statuses.clone(),
            types: self.types.clone(),
            priorities: self.priorities.clone(),
            labels,
            search: non_empty(&self.search),
            ready_only: self.ready,
            tangled_only: self.tangled,
            needs_only: self.needs,
            parent: non_empty(&self.parent),
        }
    }
}

pub(crate) fn render_drawer(d: &Drawer, frame: &mut Frame, area: Rect) {
    let rows = Layout::vertical([
        Constraint::Length(1), // header
        Constraint::Length(1), // status chips
        Constraint::Length(1), // type chips
        Constraint::Length(1), // priority chips
        Constraint::Length(1), // labels
        Constraint::Length(1), // search
        Constraint::Length(1), // parent
        Constraint::Length(1), // deps chips
        Constraint::Min(0),
    ])
    .split(area);
    frame.render_widget(
        Paragraph::new(Span::styled(
            "Filter",
            Style::new().fg(Color::Cyan).add_modifier(Modifier::BOLD),
        )),
        rows[0],
    );
    let statuses: Vec<(String, bool)> = STATUS_CHOICES
        .iter()
        .map(|&s| (status_word(s).to_string(), d.statuses.contains(&s)))
        .collect();
    let types: Vec<(String, bool)> = TYPE_CHOICES
        .iter()
        .map(|&k| (k.to_string(), d.types.iter().any(|t| t == k)))
        .collect();
    let pris: Vec<(String, bool)> = PRI_CHOICES
        .iter()
        .map(|&p| (format!("p{p}"), d.priorities.contains(&p)))
        .collect();
    let deps: Vec<(String, bool)> = vec![
        ("ready".into(), d.ready),
        ("tangled".into(), d.tangled),
        ("inbox".into(), d.needs),
    ];
    render_chip_row(
        d.row == 0,
        d.chip_idx,
        DRAWER_LABEL_W,
        "status",
        &statuses,
        frame,
        rows[1],
    );
    render_chip_row(
        d.row == 1,
        d.chip_idx,
        DRAWER_LABEL_W,
        "type",
        &types,
        frame,
        rows[2],
    );
    render_chip_row(
        d.row == 2,
        d.chip_idx,
        DRAWER_LABEL_W,
        "priority",
        &pris,
        frame,
        rows[3],
    );
    render_text_row(
        d.row == 3,
        DRAWER_LABEL_W,
        "labels",
        &d.labels,
        "(any)",
        frame,
        rows[4],
    );
    render_text_row(
        d.row == 4,
        DRAWER_LABEL_W,
        "search",
        &d.search,
        "(any)",
        frame,
        rows[5],
    );
    render_text_row(
        d.row == 5,
        DRAWER_LABEL_W,
        "parent",
        &d.parent,
        "(any)",
        frame,
        rows[6],
    );
    render_chip_row(
        d.row == 6,
        d.chip_idx,
        DRAWER_LABEL_W,
        "deps",
        &deps,
        frame,
        rows[7],
    );
}
