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
    DEPS_CHOICES, DRAWER_LABEL_W, DRAWER_ROWS, PRI_CHOICES, STATUS_CHOICES, TYPE_CHOICES,
    clone_spec, make_handler, render_chip_row, render_text_row, status_word, text_field, toggle,
};
use crate::filter::FilterSpec;
use crate::model::Status;

/// The optional herd chip row is appended after the fixed [`DRAWER_ROWS`] rows
/// (0..=6), so its row index is `DRAWER_ROWS`. It only exists on multi-herd
/// farms; single-herd farms keep the original 7-row layout untouched.
const HERD_ROW: usize = DRAWER_ROWS;

/// The filter drawer: a small form of chip facets (status/type/priority/deps)
/// and text facets (labels/search/parent). Editing it previews live on the
/// list; Enter applies, Esc reverts to `saved`. Reproduces Python `_DrawerState`.
pub(crate) struct Drawer {
    pub(crate) saved: FilterSpec,
    pub(crate) statuses: Vec<Status>,
    pub(crate) types: Vec<String>,
    pub(crate) priorities: Vec<u8>,
    /// Selected herd scope (id prefixes), toggled via the herd chip row.
    pub(crate) herds: Vec<String>,
    /// The known herd set backing the herd chip row (from `App::herd_choices`).
    /// The row is shown only when this holds more than one herd.
    pub(crate) herd_choices: Vec<String>,
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
    pub(crate) fn from_filter(vim: bool, f: &FilterSpec, herd_choices: Vec<String>) -> Self {
        Drawer {
            saved: clone_spec(f),
            statuses: f.statuses.clone(),
            types: f.types.clone(),
            priorities: f.priorities.clone(),
            herds: f.herds.clone(),
            herd_choices,
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

    /// Whether the farm spans more than one herd — the gate for the herd chip
    /// row. Single-herd farms have no herd row (and no layout/snapshot change).
    pub(crate) fn multi_herd(&self) -> bool {
        self.herd_choices.len() > 1
    }

    /// Number of navigable rows: the fixed [`DRAWER_ROWS`], plus the herd row
    /// on multi-herd farms.
    pub(crate) fn row_count(&self) -> usize {
        DRAWER_ROWS + if self.multi_herd() { 1 } else { 0 }
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
            HERD_ROW => self.herd_choices.len(),
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
            HERD_ROW => toggle(&mut self.herds, self.herd_choices[self.chip_idx].clone()),
            _ => {}
        }
    }

    pub(crate) fn clear(&mut self) {
        self.statuses.clear();
        self.types.clear();
        self.priorities.clear();
        self.herds.clear();
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
        let labels = crate::model::normalize_labels([Self::text_of(&self.labels)]);
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
            // On multi-herd farms the herd chip row owns this scope; on
            // single-herd farms `herds` stays whatever the view carried in
            // (`from_filter` seeds it from the spec), so nothing is dropped.
            herds: self.herds.clone(),
        }
    }
}

pub(crate) fn render_drawer(d: &Drawer, frame: &mut Frame, area: Rect) {
    // header + the 7 fixed rows, plus the herd row on multi-herd farms, then a
    // flex filler. Row `n` renders into `rows[n + 1]`.
    let mut constraints = vec![
        Constraint::Length(1), // header
        Constraint::Length(1), // status chips
        Constraint::Length(1), // type chips
        Constraint::Length(1), // priority chips
        Constraint::Length(1), // labels
        Constraint::Length(1), // search
        Constraint::Length(1), // parent
        Constraint::Length(1), // deps chips
    ];
    if d.multi_herd() {
        constraints.push(Constraint::Length(1)); // herd chips
    }
    constraints.push(Constraint::Min(0));
    let rows = Layout::vertical(constraints).split(area);
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
    if d.multi_herd() {
        let herds: Vec<(String, bool)> = d
            .herd_choices
            .iter()
            .map(|h| (h.clone(), d.herds.contains(h)))
            .collect();
        render_chip_row(
            d.row == HERD_ROW,
            d.chip_idx,
            DRAWER_LABEL_W,
            "herd",
            &herds,
            frame,
            rows[HERD_ROW + 1],
        );
    }
}
