//! Interactive terminal UI (`yaks tui`) — a thin layer over the core.
//!
//! All drawing goes through the pure `render(&App, &mut Frame)`, so the same
//! painter can target a real terminal (crossterm) or an in-memory `TestBackend`
//! buffer (snapshot tests + the future demo-cast pipeline). Key handling only
//! mutates `App`; mutating keys route through the `Herd` facade and then reload.

mod cache;
mod detail;
mod headless;
mod tree;
mod view;
mod views_store;

pub use headless::{HeadlessOpts, StyleEncoding, run_headless};

use std::cell::RefCell;
use std::collections::HashSet;
use std::io::{self, Stdout};

use anyhow::Result;
use edtui::{EditorEventHandler, EditorMode, EditorState, EditorTheme, EditorView, Lines};
use ratatui::backend::CrosstermBackend;
use ratatui::crossterm::event::{
    self, Event, KeyCode, KeyEvent, KeyEventKind, KeyModifiers, KeyboardEnhancementFlags,
    PopKeyboardEnhancementFlags, PushKeyboardEnhancementFlags,
};
use ratatui::crossterm::execute;
use ratatui::crossterm::terminal::{
    EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode, enable_raw_mode,
    supports_keyboard_enhancement,
};
use ratatui::layout::{Constraint, Layout, Rect};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, List, ListItem, ListState, Padding, Paragraph};
use ratatui::{Frame, Terminal};

use crate::filter::{self, FilterSpec};
use crate::herd::{
    CreateOutcome, DepOutcome, Herd, MoveOutcome, NewTask, Reparent, TaskEdit, UpdateOutcome,
};
use crate::model::{Status, Task};

#[derive(Clone, Copy, PartialEq, Debug)]
enum Focus {
    List,
    Detail,
}

/// A modal prompt painted on the bottom line, reproducing the Python TUI's
/// `pick()` (single keypress) and `confirm()` (y/N) dialogs. Kept as plain
/// data so `render` stays pure — the action to perform on commit rides along.
enum Overlay {
    None,
    /// Single-key picker: any char in `keys` resolves `action`; Esc cancels.
    Pick {
        prompt: String,
        keys: String,
        action: PickAction,
    },
    /// y/N confirmation; Enter and Esc both default to "no".
    Confirm {
        prompt: String,
        action: ConfirmAction,
    },
    /// An in-frame edtui editor (single-line field or multi-line body).
    Edit(Editor),
    /// A filter-as-you-type task picker (dependencies, reparent).
    Fuzzy(FuzzyPick),
    /// Inline incremental search: edits `App.filter.search` live.
    Search(SearchBox),
    /// The multi-row filter drawer (chips + text facets).
    Drawer(Drawer),
    /// The create-task form (title/type/priority/labels/description).
    Create(CreateForm),
    /// Detail-pane find: edits `App.detail_find` live.
    DetailFind(SearchBox),
    /// The view manager (`v`): carries the picker's selection index.
    ViewPicker(usize),
}

/// What a resolved single-key pick should do (carries the target task id).
enum PickAction {
    State(String),
    Priority(String),
    Type(String),
}

/// What a confirmed y/N prompt should do.
enum ConfirmAction {
    Slaughter(String),
}

/// An embedded edtui editor plus what to do with its text on commit. The
/// `EditorState` is `RefCell`-wrapped because `EditorView` needs `&mut` at
/// render time, while our `render(&App, ..)` borrows the app immutably.
struct Editor {
    state: RefCell<EditorState>,
    handler: EditorEventHandler,
    single_line: bool,
    /// Prompt label (bottom-line prefix for fields; header for the body panel).
    label: String,
    action: EditAction,
}

enum EditAction {
    Labels(String),
    Body(String),
    SaveView,
    RenameView { index: usize },
}

fn make_handler(vim: bool) -> EditorEventHandler {
    if vim {
        EditorEventHandler::vim_mode()
    } else {
        EditorEventHandler::emacs_mode()
    }
}

impl Editor {
    fn new(vim: bool, single_line: bool, label: String, initial: &str, action: EditAction) -> Self {
        let mut state = EditorState::new(Lines::from(initial));
        state.set_single_line(single_line);
        // Start in Insert so the user can type immediately (vim `i` is implied).
        state.mode = EditorMode::Insert;
        Editor {
            state: RefCell::new(state),
            handler: make_handler(vim),
            single_line,
            label,
            action,
        }
    }

    fn text(&self) -> String {
        self.state.borrow().lines.to_string()
    }
}

/// A filter-as-you-type picker over the task set. The query is an edtui
/// single-line editor; candidates are ranked substring matches (Python's
/// `fuzzy_pick_task` semantics). `RefCell` for the same render-purity reason.
struct FuzzyPick {
    label: String,
    query: RefCell<EditorState>,
    handler: EditorEventHandler,
    /// Ids never offered (self, existing deps/parents, cycle- or loop-forming).
    exclude: HashSet<String>,
    /// When true, a synthetic top row clears the parent (reparent to root).
    allow_none: bool,
    sel: usize,
    action: FuzzyAction,
}

enum FuzzyAction {
    AddDep(String),
    Reparent(String),
}

/// Inline incremental search box. Editing it updates `App.filter.search` on
/// every keystroke (live preview); Esc restores the pre-search query.
struct SearchBox {
    query: RefCell<EditorState>,
    handler: EditorEventHandler,
    /// The `filter.search` value before opening, restored on cancel.
    saved: Option<String>,
}

// Filter-drawer layout: 7 rows, three of them free-text.
const DRAWER_ROWS: usize = 7;
const STATUS_CHOICES: [Status; 4] = [Status::Hairy, Status::Shaving, Status::Shorn, Status::Dead];
const TYPE_CHOICES: [&str; 4] = ["task", "bug", "feature", "idea"];
const PRI_CHOICES: [u8; 5] = [1, 2, 3, 4, 5];
const DEPS_CHOICES: [&str; 2] = ["ready", "tangled"];

/// The filter drawer: a small form of chip facets (status/type/priority/deps)
/// and text facets (labels/search/parent). Editing it previews live on the
/// list; Enter applies, Esc reverts to `saved`. Reproduces Python `_DrawerState`.
struct Drawer {
    saved: FilterSpec,
    statuses: Vec<Status>,
    types: Vec<String>,
    priorities: Vec<u8>,
    ready: bool,
    tangled: bool,
    labels: RefCell<EditorState>,
    search: RefCell<EditorState>,
    parent: RefCell<EditorState>,
    handler: EditorEventHandler,
    row: usize,
    chip_idx: usize,
}

fn text_field(seed: &str, vim: bool) -> RefCell<EditorState> {
    let mut st = EditorState::new(Lines::from(seed));
    st.set_single_line(true);
    st.mode = EditorMode::Insert;
    let _ = vim;
    RefCell::new(st)
}

fn toggle<T: PartialEq>(v: &mut Vec<T>, val: T) {
    if let Some(i) = v.iter().position(|x| *x == val) {
        v.remove(i);
    } else {
        v.push(val);
    }
}

impl Drawer {
    fn from_filter(vim: bool, f: &FilterSpec) -> Self {
        Drawer {
            saved: clone_spec(f),
            statuses: f.statuses.clone(),
            types: f.types.clone(),
            priorities: f.priorities.clone(),
            ready: f.ready_only,
            tangled: f.tangled_only,
            labels: text_field(&f.labels.join(", "), vim),
            search: text_field(f.search.as_deref().unwrap_or(""), vim),
            parent: text_field(f.parent.as_deref().unwrap_or(""), vim),
            handler: make_handler(vim),
            row: 0,
            chip_idx: 0,
        }
    }

    fn is_text_row(&self) -> bool {
        matches!(self.row, 3 | 4 | 5)
    }

    fn chip_count(&self) -> usize {
        match self.row {
            0 => STATUS_CHOICES.len(),
            1 => TYPE_CHOICES.len(),
            2 => PRI_CHOICES.len(),
            6 => DEPS_CHOICES.len(),
            _ => 0,
        }
    }

    fn toggle_chip(&mut self) {
        match self.row {
            0 => toggle(&mut self.statuses, STATUS_CHOICES[self.chip_idx]),
            1 => toggle(&mut self.types, TYPE_CHOICES[self.chip_idx].to_string()),
            2 => toggle(&mut self.priorities, PRI_CHOICES[self.chip_idx]),
            6 => {
                if self.chip_idx == 0 {
                    self.ready = !self.ready;
                } else {
                    self.tangled = !self.tangled;
                }
            }
            _ => {}
        }
    }

    fn clear(&mut self) {
        self.statuses.clear();
        self.types.clear();
        self.priorities.clear();
        self.ready = false;
        self.tangled = false;
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

    fn build_spec(&self) -> FilterSpec {
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
            parent: non_empty(&self.parent),
        }
    }
}

// Create-form layout: 5 rows (title, type, priority, labels, description).
const CREATE_ROWS: usize = 5;

/// The create-task form: a small right-pane form modeled on `Drawer`. Two chip
/// rows (type/priority) are **single-select** — the cursor *is* the value
/// (unlike the drawer's Space-toggle multi-select) — plus three text rows
/// (title/labels/description). `Enter` creates, `Esc` cancels. Reproduces the
/// Python `task_form` create flow as a thin form over `Herd::create`.
struct CreateForm {
    title: RefCell<EditorState>,
    labels: RefCell<EditorState>,
    description: RefCell<EditorState>,
    /// Index into `TYPE_CHOICES` (single-select cursor==value).
    kind_idx: usize,
    /// Index into `PRI_CHOICES` (single-select cursor==value; default → p3).
    pri_idx: usize,
    row: usize,
    parent: Option<String>,
    handler: EditorEventHandler,
}

impl CreateForm {
    fn new(vim: bool, parent: Option<String>) -> Self {
        CreateForm {
            title: text_field("", vim),
            labels: text_field("", vim),
            description: text_field("", vim),
            kind_idx: 0,                                                    // task
            pri_idx: PRI_CHOICES.iter().position(|&p| p == 3).unwrap_or(2), // p3
            row: 0,
            parent,
            handler: make_handler(vim),
        }
    }

    fn is_text_row(&self) -> bool {
        matches!(self.row, 0 | 3 | 4)
    }

    /// Move the single-select chip cursor on a chip row (wrapping).
    fn move_chip(&mut self, delta: i32) {
        match self.row {
            1 => {
                let n = TYPE_CHOICES.len() as i32;
                self.kind_idx = (self.kind_idx as i32 + delta).rem_euclid(n) as usize;
            }
            2 => {
                let n = PRI_CHOICES.len() as i32;
                self.pri_idx = (self.pri_idx as i32 + delta).rem_euclid(n) as usize;
            }
            _ => {}
        }
    }

    fn title_text(&self) -> String {
        self.title.borrow().lines.to_string().trim().to_string()
    }

    fn labels_vec(&self) -> Vec<String> {
        self.labels
            .borrow()
            .lines
            .to_string()
            .split(',')
            .map(|s| s.trim())
            .filter(|s| !s.is_empty())
            .map(String::from)
            .collect()
    }

    fn description_opt(&self) -> Option<String> {
        let d = self.description.borrow().lines.to_string();
        if d.trim().is_empty() { None } else { Some(d) }
    }
}

/// Sort key for a flat view field (ISO timestamps sort lexically; priority is
/// zero-padded so it orders numerically as a string).
fn sort_key(t: &Task, f: view::SortField) -> String {
    match f {
        view::SortField::Priority => format!("{:03}", t.priority),
        view::SortField::Title => t.title.to_lowercase(),
        view::SortField::Updated => t.updated.clone().unwrap_or_default(),
        view::SortField::Created => t.created.clone().unwrap_or_default(),
        view::SortField::Id => t.id.clone(),
    }
}

fn same_set<T: PartialEq>(a: &[T], b: &[T]) -> bool {
    a.len() == b.len() && a.iter().all(|x| b.contains(x)) && b.iter().all(|x| a.contains(x))
}

/// Order-insensitive FilterSpec equality (for the view-modified marker).
fn spec_eq(a: &FilterSpec, b: &FilterSpec) -> bool {
    same_set(&a.statuses, &b.statuses)
        && same_set(&a.types, &b.types)
        && same_set(&a.priorities, &b.priorities)
        && same_set(&a.labels, &b.labels)
        && a.search == b.search
        && a.ready_only == b.ready_only
        && a.tangled_only == b.tangled_only
        && a.parent == b.parent
}

/// Snapshot a spec (thin alias for `.clone()`; kept for call-site clarity).
fn clone_spec(f: &FilterSpec) -> FilterSpec {
    f.clone()
}

impl SearchBox {
    fn new(vim: bool, initial: Option<String>) -> Self {
        let seed = initial.clone().unwrap_or_default();
        let mut st = EditorState::new(Lines::from(seed.as_str()));
        st.set_single_line(true);
        st.mode = EditorMode::Insert;
        SearchBox {
            query: RefCell::new(st),
            handler: make_handler(vim),
            saved: initial,
        }
    }

    fn query_text(&self) -> String {
        self.query.borrow().lines.to_string()
    }
}

impl FuzzyPick {
    fn new(
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
        }
    }

    fn query_text(&self) -> String {
        self.query.borrow().lines.to_string()
    }
}

/// Ranked substring matches over `all`, honoring the picker's exclude set and
/// query. Empty query lists everything (capped). Score: id-prefix < id-substr
/// < title-substr, then priority, then id.
fn fuzzy_candidates<'a>(all: &'a [Task], fp: &FuzzyPick) -> Vec<&'a Task> {
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
fn fuzzy_total(all: &[Task], fp: &FuzzyPick) -> usize {
    fuzzy_candidates(all, fp).len() + fp.allow_none as usize
}

/// TUI state. Holds the loaded task set plus (in live use) a `Herd` handle so
/// mutations re-query through the core. Per-tab tree views are derived on demand.
pub struct App {
    /// `None` in read-only snapshot tests; `Some` in live use (`with_herd`).
    herd: Option<Herd>,
    all: Vec<Task>,
    /// Ordered views; pinned ones form the tab strip. Replaces fixed tabs.
    views: Vec<view::View>,
    /// Index into `views` of the active view.
    view: usize,
    /// Ordered starred ids backing the built-in Starred (working-set) view.
    working_set: Vec<String>,
    cursor: usize,
    focus: Focus,
    detail_scroll: u16,
    /// Index into the current detail's jumplist (Tab-cycled link targets).
    detail_link: usize,
    /// Active detail-pane find query + which match is current (n/N cycle).
    detail_find: Option<String>,
    detail_match: usize,
    collapsed: HashSet<String>,
    /// The live view filter applied by the tree (re-colors + prunes).
    filter: FilterSpec,
    /// Approx. list viewport height, refreshed each loop for paging math.
    page: u16,
    overlay: Overlay,
    /// Transient one-line status message shown until the next mutation.
    notification: Option<String>,
    /// Editor keybinding profile (vim vs emacs), from herd config.
    editor_vim: bool,
    quit: bool,
}

impl App {
    /// Read-only constructor: renders `all` with no herd behind it. Mutating
    /// keys become no-ops. Used by snapshot tests and any preview caller.
    pub fn new(all: Vec<Task>) -> Self {
        let views = view::default_views();
        let filter = clone_spec(&views[0].spec);
        App {
            herd: None,
            all,
            views,
            view: 0,
            working_set: Vec::new(),
            cursor: 0,
            focus: Focus::List,
            detail_scroll: 0,
            detail_link: 0,
            detail_find: None,
            detail_match: 0,
            collapsed: HashSet::new(),
            filter,
            page: 10,
            overlay: Overlay::None,
            notification: None,
            editor_vim: true,
            quit: false,
        }
    }

    /// Live constructor: loads the current herd view and keeps the handle so
    /// mutations can re-query after each change.
    pub fn with_herd(herd: Herd) -> Result<Self> {
        let all = herd.list(FilterSpec::default(), false)?;
        let vim = herd.config().vim_mode;
        let collapsed = cache::load_collapsed(herd.root());
        let views = views_store::load_views(herd.root());
        let working_set = views_store::load_working_set(herd.root());
        let mut app = App::new(all);
        app.editor_vim = vim;
        app.collapsed = collapsed;
        app.filter = clone_spec(&views[0].spec);
        app.views = views;
        app.working_set = working_set;
        app.herd = Some(herd);
        app.clamp_cursor();
        Ok(app)
    }

    /// Persist the (rebuildable) collapsed-tree state to the per-user cache.
    fn save_collapsed(&self) {
        if let Some(h) = &self.herd {
            cache::save_collapsed(h.root(), &self.collapsed);
        }
    }

    /// Re-query the herd view after a mutation and keep the cursor in range.
    fn reload(&mut self) {
        if let Some(h) = &self.herd {
            if let Ok(all) = h.list(FilterSpec::default(), false) {
                self.all = all;
            }
        }
        let len = self.rows().len();
        self.cursor = if len == 0 {
            0
        } else {
            self.cursor.min(len - 1)
        };
    }

    fn active_view(&self) -> &view::View {
        &self.views[self.view]
    }

    /// Count for a view: non-ghost tree rows of its own spec, or working-set
    /// membership. Independent of the live filter (a stable per-view size).
    fn view_count(&self, v: &view::View) -> usize {
        if v.key == "working-set" {
            return self
                .working_set
                .iter()
                .filter(|id| self.task(id).is_some())
                .count();
        }
        tree::build(&self.all, &v.spec)
            .iter()
            .filter(|r| !r.ghost)
            .count()
    }

    /// Hairy tasks with at least one unresolved dependency (the blocked set;
    /// Python marks these with a magenta `*`).
    fn blocked_ids(&self) -> HashSet<String> {
        let resolved = filter::resolved_ids(&self.all);
        self.all
            .iter()
            .filter(|t| {
                t.status == Status::Hairy
                    && t.depends_on.iter().any(|d| !resolved.contains(d.as_str()))
            })
            .map(|t| t.id.clone())
            .collect()
    }

    /// Indices of pinned views — exactly the tab strip, in order.
    fn pinned_indices(&self) -> Vec<usize> {
        (0..self.views.len())
            .filter(|&i| self.views[i].pinned)
            .collect()
    }

    /// Rows the starred working set resolves to, in star order (flat).
    fn working_set_rows(&self) -> Vec<tree::Row<'_>> {
        self.working_set
            .iter()
            .filter_map(|id| self.task(id))
            .map(tree::Row::leaf)
            .collect()
    }

    /// Flat, sorted rows for a sorted view (Recent / custom sorted).
    fn flat_rows(&self) -> Vec<tree::Row<'_>> {
        let v = self.active_view();
        let resolved = filter::resolved_ids(&self.all);
        let mut matched: Vec<&Task> = self
            .all
            .iter()
            .filter(|t| self.filter.matches(t, &resolved))
            .collect();
        let sort_by = v.sort_by.unwrap_or(view::SortField::Updated);
        matched.sort_by(|a, b| sort_key(a, sort_by).cmp(&sort_key(b, sort_by)));
        if v.sort_dir == view::SortDir::Desc {
            matched.reverse();
        }
        if let Some(lim) = v.limit {
            matched.truncate(lim);
        }
        matched.into_iter().map(tree::Row::leaf).collect()
    }

    /// Visible rows for the active view (dispatch: working-set / flat / tree).
    fn rows(&self) -> Vec<tree::Row<'_>> {
        let v = self.active_view();
        if v.key == "working-set" {
            return self.working_set_rows();
        }
        if v.is_flat() {
            return self.flat_rows();
        }
        let flat = tree::build(&self.all, &self.filter);
        tree::apply_collapse(flat, &self.collapsed)
    }

    /// Keep the cursor within the current row count (after a filter change).
    fn clamp_cursor(&mut self) {
        let len = self.rows().len();
        self.cursor = if len == 0 {
            0
        } else {
            self.cursor.min(len - 1)
        };
    }

    fn selected(&self) -> Option<&Task> {
        self.rows().into_iter().nth(self.cursor).map(|r| r.task)
    }

    fn selected_id(&self) -> Option<String> {
        self.selected().map(|t| t.id.clone())
    }

    fn task(&self, id: &str) -> Option<&Task> {
        self.all.iter().find(|t| t.id == id)
    }

    /// Activate view `i`: load its saved spec into the live filter and reset.
    fn set_view(&mut self, i: usize) {
        self.view = i;
        self.filter = clone_spec(&self.views[i].spec);
        self.cursor = 0;
        self.detail_scroll = 0;
        self.focus = Focus::List;
        self.clamp_cursor();
    }

    /// Cycle through the pinned views (the visible tabs) only.
    fn switch_tab(&mut self, delta: i32) {
        let pinned = self.pinned_indices();
        if pinned.is_empty() {
            return;
        }
        let cur = pinned.iter().position(|&i| i == self.view).unwrap_or(0);
        let n = pinned.len() as i32;
        let next = pinned[(((cur as i32 + delta) % n + n) % n) as usize];
        self.set_view(next);
    }

    /// True when the live filter has been edited away from the active view spec.
    fn is_view_modified(&self) -> bool {
        !spec_eq(&self.filter, &self.active_view().spec)
    }

    /// Esc: revert the live filter to the active view's saved spec.
    fn revert_filter_to_view(&mut self) {
        if self.is_view_modified() {
            let i = self.view;
            self.set_view(i);
        }
    }

    fn toggle_star(&mut self) {
        let Some(id) = self.selected_id() else { return };
        let was = self.working_set.iter().any(|i| *i == id);
        self.working_set = views_store::toggle_working_set(&self.working_set, &id);
        if let Some(h) = &self.herd {
            views_store::save_working_set(h.root(), &self.working_set);
        }
        if self.active_view().key == "working-set" {
            self.clamp_cursor();
        }
        self.notification = Some(if was {
            format!("unstarred {id}")
        } else {
            format!("starred {id}")
        });
    }

    fn is_starred(&self, id: &str) -> bool {
        self.working_set.iter().any(|i| i == id)
    }

    fn save_current_view(&mut self, name: String) {
        if name.trim().is_empty() {
            self.notification = Some("save view cancelled".into());
            return;
        }
        let active = self.active_view();
        let (sort_by, sort_dir, limit) = (active.sort_by, active.sort_dir, active.limit);
        let seed = views_store::custom_key_seed(&name, &self.views);
        let v = view::custom_view(
            name.clone(),
            clone_spec(&self.filter),
            sort_by,
            sort_dir,
            limit,
            &seed,
        );
        self.views.push(v);
        if let Some(h) = &self.herd {
            views_store::save_views(h.root(), &self.views);
        }
        self.set_view(self.views.len() - 1);
        self.notification = Some(format!("saved view: {name}"));
    }

    fn persist_views(&self) {
        if let Some(h) = &self.herd {
            views_store::save_views(h.root(), &self.views);
        }
    }

    fn open_view_picker(&mut self) {
        self.overlay = Overlay::ViewPicker(self.view.min(self.views.len().saturating_sub(1)));
    }

    fn handle_view_picker_key(&mut self, k: KeyEvent) {
        let ctrl = k.modifiers.contains(KeyModifiers::CONTROL);
        let sel = match &self.overlay {
            Overlay::ViewPicker(s) => *s,
            _ => return,
        };
        let n = self.views.len();
        match k.code {
            KeyCode::Esc | KeyCode::Char('q') => self.overlay = Overlay::None,
            KeyCode::Down | KeyCode::Char('j') => {
                self.overlay = Overlay::ViewPicker((sel + 1).min(n - 1))
            }
            KeyCode::Up | KeyCode::Char('k') => {
                self.overlay = Overlay::ViewPicker(sel.saturating_sub(1))
            }
            KeyCode::Enter => {
                self.set_view(sel);
                self.overlay = Overlay::None;
            }
            KeyCode::Char('p') | KeyCode::Char(' ') => {
                if self.views[sel].pinned && !views_store::can_unpin(&self.views, sel) {
                    self.notification = Some("can't unpin the last tab".into());
                } else {
                    self.views[sel].pinned = !self.views[sel].pinned;
                    self.persist_views();
                }
            }
            KeyCode::Char('J') => self.reorder_view(sel, 1),
            KeyCode::Char('K') => self.reorder_view(sel, -1),
            KeyCode::Char('r') => {
                let name = self.views[sel].name.clone();
                self.overlay = Overlay::Edit(Editor::new(
                    self.editor_vim,
                    true,
                    "Rename view: ".into(),
                    &name,
                    EditAction::RenameView { index: sel },
                ));
            }
            KeyCode::Char('d') => self.delete_view(sel),
            _ if ctrl && k.code == KeyCode::Char('n') => {
                self.overlay = Overlay::ViewPicker((sel + 1).min(n - 1))
            }
            _ if ctrl && k.code == KeyCode::Char('p') => {
                self.overlay = Overlay::ViewPicker(sel.saturating_sub(1))
            }
            _ => {}
        }
    }

    /// Move view `sel` by `dir`, keeping the active view pointing at its own
    /// entry and the picker selection following the moved row.
    fn reorder_view(&mut self, sel: usize, dir: i32) {
        let active_key = self.views[self.view].key.clone();
        let ns = views_store::move_view(&mut self.views, sel, dir);
        self.view = self
            .views
            .iter()
            .position(|v| v.key == active_key)
            .unwrap_or(self.view);
        self.persist_views();
        self.overlay = Overlay::ViewPicker(ns);
    }

    fn delete_view(&mut self, sel: usize) {
        if self.views[sel].builtin {
            self.notification = Some("can't delete a built-in view".into());
            return;
        }
        let active_key = self.views[self.view].key.clone();
        self.views.remove(sel);
        self.persist_views();
        // Keep the active view valid; if it was the deleted one, clamp.
        self.view = self
            .views
            .iter()
            .position(|v| v.key == active_key)
            .unwrap_or_else(|| self.view.min(self.views.len() - 1));
        let ns = sel.min(self.views.len() - 1);
        self.overlay = Overlay::ViewPicker(ns);
        self.clamp_cursor();
    }

    fn move_cursor(&mut self, delta: i32) {
        let len = self.rows().len() as i32;
        if len == 0 {
            self.cursor = 0;
            return;
        }
        self.cursor = (self.cursor as i32 + delta).clamp(0, len - 1) as usize;
    }

    /// One-line summary of internal state for the headless snapshot header.
    /// This is the optional, developer-fillable debug facility (Encoding "B"):
    /// it surfaces app-state facts that colour/layout alone can hide, so
    /// internal-state bugs show up directly in a snapshot. Keep it one line.
    fn state_header(&self) -> String {
        let focus = match self.focus {
            Focus::List => "list",
            Focus::Detail => "detail",
        };
        let sel = self.selected_id().unwrap_or_else(|| "-".into());
        let mut blocked: Vec<String> = self.blocked_ids().into_iter().collect();
        blocked.sort();
        let blocked = if blocked.is_empty() {
            "-".to_string()
        } else {
            format!("[{}]", blocked.join(","))
        };
        let filter = {
            let s = filter_summary(&self.filter);
            if s.is_empty() { "-".to_string() } else { s }
        };
        format!(
            "focus={focus} · view={} · cursor={} · rows={} · sel={sel} · blocked={blocked} · collapsed={} · filter={filter} · overlay={}",
            self.active_view().name,
            self.cursor,
            self.rows().len(),
            self.collapsed.len(),
            overlay_name(&self.overlay),
        )
    }

    fn toggle_collapse(&mut self) {
        let rows = self.rows();
        if let Some(row) = rows.get(self.cursor) {
            if row.has_children {
                let id = row.task.id.clone();
                if !self.collapsed.remove(&id) {
                    self.collapsed.insert(id);
                }
                self.save_collapsed();
            }
        }
    }

    // -- overlay openers --------------------------------------------------

    fn open_state_picker(&mut self) {
        if let Some(id) = self.selected_id() {
            self.overlay = Overlay::Pick {
                prompt: format!(
                    "State for {id}: h=hairy s=shaving n=shorn x=slaughter  (Esc=cancel)"
                ),
                keys: "hsnx".into(),
                action: PickAction::State(id),
            };
        }
    }

    fn open_priority_picker(&mut self) {
        if let Some(id) = self.selected_id() {
            self.overlay = Overlay::Pick {
                prompt: format!(
                    "Priority for {id}: 1=urgent 2=high 3=med 4=low 5=lowest  (Esc=cancel)"
                ),
                keys: "12345".into(),
                action: PickAction::Priority(id),
            };
        }
    }

    fn open_type_picker(&mut self) {
        if let Some(id) = self.selected_id() {
            self.overlay = Overlay::Pick {
                prompt: format!("Type for {id}: t=task b=bug f=feature i=idea  (Esc=cancel)"),
                keys: "tbfi".into(),
                action: PickAction::Type(id),
            };
        }
    }

    fn open_slaughter_confirm(&mut self) {
        let Some(id) = self.selected_id() else { return };
        let kids = self
            .all
            .iter()
            .filter(|c| c.parent.as_deref() == Some(id.as_str()) && c.status != Status::Dead)
            .count();
        if kids > 0 {
            let noun = if kids == 1 { "child" } else { "children" };
            self.notification = Some(format!("{id} has {kids} {noun}; slaughter them first"));
            return;
        }
        let title: String = self
            .task(&id)
            .map(|t| t.title.chars().take(40).collect())
            .unwrap_or_default();
        self.overlay = Overlay::Confirm {
            prompt: format!("Slaughter {id} ({title})? (y/N): "),
            action: ConfirmAction::Slaughter(id),
        };
    }

    fn open_labels(&mut self) {
        if let Some(id) = self.selected_id() {
            let initial = self
                .task(&id)
                .map(|t| t.labels.join(", "))
                .unwrap_or_default();
            self.overlay = Overlay::Edit(Editor::new(
                self.editor_vim,
                true,
                format!("Labels for {id}: "),
                &initial,
                EditAction::Labels(id),
            ));
        }
    }

    fn open_create(&mut self, child: bool) {
        let parent = if child { self.selected_id() } else { None };
        if child && parent.is_none() {
            return;
        }
        self.overlay = Overlay::Create(CreateForm::new(self.editor_vim, parent));
    }

    fn handle_create_key(&mut self, k: KeyEvent) {
        let ctrl = k.modifiers.contains(KeyModifiers::CONTROL);
        let (row, is_text) = match &self.overlay {
            Overlay::Create(f) => (f.row, f.is_text_row()),
            _ => return,
        };
        // Global commit / cancel. Enter creates (guarded on a non-empty title,
        // mirroring Python's `(need title)` — an empty title keeps the form open).
        match k.code {
            KeyCode::Enter => {
                let has_title =
                    matches!(&self.overlay, Overlay::Create(f) if !f.title_text().is_empty());
                if has_title {
                    self.commit_create();
                }
                return;
            }
            KeyCode::Esc => {
                self.overlay = Overlay::None;
                self.notification = Some("create cancelled".into());
                return;
            }
            _ => {}
        }
        // Row navigation. On text rows only Tab/arrows/Ctrl move rows (so j/k
        // stay typeable); on chip rows j/k also navigate.
        let nav_down = matches!(k.code, KeyCode::Down | KeyCode::Tab)
            || (ctrl && k.code == KeyCode::Char('n'))
            || (!is_text && k.code == KeyCode::Char('j'));
        let nav_up = matches!(k.code, KeyCode::Up | KeyCode::BackTab)
            || (ctrl && k.code == KeyCode::Char('p'))
            || (!is_text && k.code == KeyCode::Char('k'));
        if nav_down || nav_up {
            if let Overlay::Create(f) = &mut self.overlay {
                f.row = if nav_down {
                    (f.row + 1) % CREATE_ROWS
                } else {
                    (f.row + CREATE_ROWS - 1) % CREATE_ROWS
                };
            }
            return;
        }
        if is_text {
            if let Overlay::Create(f) = &mut self.overlay {
                match row {
                    0 => f.handler.on_key_event(k, &mut f.title.borrow_mut()),
                    3 => f.handler.on_key_event(k, &mut f.labels.borrow_mut()),
                    4 => f.handler.on_key_event(k, &mut f.description.borrow_mut()),
                    _ => {}
                }
            }
            return;
        }
        // Chip rows: left/right (h/l) move the single-select cursor = value.
        if let Overlay::Create(f) = &mut self.overlay {
            match k.code {
                KeyCode::Left | KeyCode::Char('h') => f.move_chip(-1),
                KeyCode::Right | KeyCode::Char('l') => f.move_chip(1),
                _ => {}
            }
        }
    }

    fn commit_create(&mut self) {
        let new = match std::mem::replace(&mut self.overlay, Overlay::None) {
            Overlay::Create(f) => NewTask {
                title: f.title_text(),
                kind: Some(TYPE_CHOICES[f.kind_idx].to_string()),
                priority: Some(PRI_CHOICES[f.pri_idx]),
                parent: f.parent.clone(),
                labels: f.labels_vec(),
                depends_on: vec![],
                source: None,
                description: f.description_opt(),
            },
            other => {
                self.overlay = other;
                return;
            }
        };
        let Some(h) = &self.herd else { return };
        match h.create(new) {
            Ok(CreateOutcome::Created(t)) => {
                let id = t.id.clone();
                self.reload();
                self.select_id(&id);
                self.notification = Some(format!("created {id}"));
            }
            Ok(CreateOutcome::ParentNotFound(p)) => {
                self.notification = Some(format!("parent {p} not found"))
            }
            Err(e) => self.notification = Some(format!("error: {e}")),
        }
    }

    fn open_body_edit(&mut self) {
        if let Some(id) = self.selected_id() {
            let initial = self.task(&id).map(|t| t.body.clone()).unwrap_or_default();
            self.overlay = Overlay::Edit(Editor::new(
                self.editor_vim,
                false,
                format!("Edit {id} — Ctrl-S save · Ctrl-C cancel"),
                &initial,
                EditAction::Body(id),
            ));
        }
    }

    fn open_dep_picker(&mut self) {
        let Some(id) = self.selected_id() else { return };
        let mut exclude: HashSet<String> = HashSet::new();
        exclude.insert(id.clone());
        if let Some(t) = self.task(&id) {
            for d in &t.depends_on {
                exclude.insert(d.clone());
            }
        }
        // Exclude any task that already reaches `id`, which would form a cycle.
        for t in &self.all {
            if t.id != id && filter::depends_on_transitively(&self.all, &t.id, &id) {
                exclude.insert(t.id.clone());
            }
        }
        self.overlay = Overlay::Fuzzy(FuzzyPick::new(
            self.editor_vim,
            format!("Add dependency to {id}"),
            exclude,
            false,
            FuzzyAction::AddDep(id),
        ));
    }

    fn open_reparent_picker(&mut self) {
        let Some(id) = self.selected_id() else { return };
        let mut exclude = filter::descendant_ids(&self.all, &id, true);
        exclude.insert(id.clone());
        let has_parent = self.task(&id).map(|t| t.parent.is_some()).unwrap_or(false);
        if let Some(cur) = self.task(&id).and_then(|t| t.parent.clone()) {
            exclude.insert(cur);
        }
        self.overlay = Overlay::Fuzzy(FuzzyPick::new(
            self.editor_vim,
            format!("Reparent {id}"),
            exclude,
            has_parent,
            FuzzyAction::Reparent(id),
        ));
    }

    fn open_search(&mut self) {
        let cur = self.filter.search.clone();
        self.overlay = Overlay::Search(SearchBox::new(self.editor_vim, cur));
    }

    fn open_drawer(&mut self) {
        self.overlay = Overlay::Drawer(Drawer::from_filter(self.editor_vim, &self.filter));
    }

    fn open_save_view(&mut self) {
        self.overlay = Overlay::Edit(Editor::new(
            self.editor_vim,
            true,
            "Save view as: ".into(),
            "",
            EditAction::SaveView,
        ));
    }

    fn drawer_live_preview(&mut self) {
        let spec = match &self.overlay {
            Overlay::Drawer(d) => Some(d.build_spec()),
            _ => None,
        };
        if let Some(s) = spec {
            self.filter = s;
            self.clamp_cursor();
        }
    }

    fn close_drawer(&mut self, commit: bool) {
        if let Overlay::Drawer(d) = std::mem::replace(&mut self.overlay, Overlay::None) {
            self.filter = if commit { d.build_spec() } else { d.saved };
            self.clamp_cursor();
            self.notification = Some(if commit {
                "filter applied".into()
            } else {
                "filter unchanged".into()
            });
        }
    }

    fn handle_drawer_key(&mut self, k: KeyEvent) {
        let ctrl = k.modifiers.contains(KeyModifiers::CONTROL);
        let (row, is_text) = match &self.overlay {
            Overlay::Drawer(d) => (d.row, d.is_text_row()),
            _ => return,
        };
        // Global apply / cancel / clear.
        match k.code {
            KeyCode::Enter => return self.close_drawer(true),
            KeyCode::Esc => return self.close_drawer(false),
            KeyCode::Char('C') => {
                if let Overlay::Drawer(d) = &mut self.overlay {
                    d.clear();
                }
                return self.drawer_live_preview();
            }
            _ => {}
        }
        // Row navigation. On text rows only Tab/arrows/Ctrl move rows (so j/k
        // remain typeable); on chip rows j/k also navigate.
        let nav_down = matches!(k.code, KeyCode::Down | KeyCode::Tab)
            || (ctrl && k.code == KeyCode::Char('n'))
            || (!is_text && k.code == KeyCode::Char('j'));
        let nav_up = matches!(k.code, KeyCode::Up | KeyCode::BackTab)
            || (ctrl && k.code == KeyCode::Char('p'))
            || (!is_text && k.code == KeyCode::Char('k'));
        if nav_down || nav_up {
            if let Overlay::Drawer(d) = &mut self.overlay {
                d.row = if nav_down {
                    (d.row + 1) % DRAWER_ROWS
                } else {
                    (d.row + DRAWER_ROWS - 1) % DRAWER_ROWS
                };
                d.chip_idx = 0;
            }
            return;
        }
        if is_text {
            if let Overlay::Drawer(d) = &mut self.overlay {
                match row {
                    3 => d.handler.on_key_event(k, &mut d.labels.borrow_mut()),
                    4 => d.handler.on_key_event(k, &mut d.search.borrow_mut()),
                    5 => d.handler.on_key_event(k, &mut d.parent.borrow_mut()),
                    _ => {}
                }
            }
            return self.drawer_live_preview();
        }
        // Chip rows: left/right move the cursor, Space toggles.
        let mut changed = false;
        if let Overlay::Drawer(d) = &mut self.overlay {
            let n = d.chip_count();
            match k.code {
                KeyCode::Left | KeyCode::Char('h') if n > 0 => {
                    d.chip_idx = (d.chip_idx + n - 1) % n
                }
                KeyCode::Right | KeyCode::Char('l') if n > 0 => d.chip_idx = (d.chip_idx + 1) % n,
                KeyCode::Char(' ') => {
                    d.toggle_chip();
                    changed = true;
                }
                _ => {}
            }
        }
        if changed {
            self.drawer_live_preview();
        }
    }

    /// Select an existing task wherever it lives: switch to its status view (if
    /// one is pinned) and place the cursor on it, then focus the list.
    fn select_task(&mut self, id: &str) {
        if let Some(st) = self.task(id).map(|t| t.status) {
            if let Some(i) = self.views.iter().position(|v| v.status == Some(st)) {
                self.set_view(i);
            }
        }
        if let Some(pos) = self.rows().iter().position(|r| r.task.id == id) {
            self.cursor = pos;
        }
        self.focus = Focus::List;
    }

    /// Detail jumplist for the current selection (empty when nothing selected).
    fn detail_jumps(&self) -> Vec<detail::Jump> {
        match self.selected() {
            Some(t) => detail::jumplist(&detail::build(t, &self.all)),
            None => Vec::new(),
        }
    }

    fn jump_link(&mut self, delta: i32) {
        let jumps = self.detail_jumps();
        if jumps.is_empty() {
            return;
        }
        let n = jumps.len() as i32;
        self.detail_link = (self.detail_link as i32 + delta).rem_euclid(n) as usize;
        // Bring the target line into view.
        self.detail_scroll = jumps[self.detail_link].line as u16;
    }

    fn open_detail_find(&mut self) {
        let cur = self.detail_find.clone();
        self.overlay = Overlay::DetailFind(SearchBox::new(self.editor_vim, cur));
    }

    /// (line, col, len) of every detail-find match for the current selection.
    fn detail_find_matches(&self) -> Vec<(usize, usize, usize)> {
        let Some(q) = self.detail_find.as_deref().filter(|s| !s.is_empty()) else {
            return vec![];
        };
        let Some(t) = self.selected() else {
            return vec![];
        };
        detail_scan(&detail::build(t, &self.all), q)
    }

    fn detail_find_jump(&mut self, delta: i32) {
        let m = self.detail_find_matches();
        if m.is_empty() {
            return;
        }
        let n = m.len() as i32;
        self.detail_match = (self.detail_match as i32 + delta).rem_euclid(n) as usize;
        self.detail_scroll = m[self.detail_match].0 as u16;
    }

    fn follow_link(&mut self) {
        let jumps = self.detail_jumps();
        let Some(j) = jumps.into_iter().nth(self.detail_link) else {
            return;
        };
        match j.target {
            detail::Target::Task(id) => {
                self.select_task(&id);
                self.notification = Some(format!("→ {id}"));
            }
            detail::Target::Url(u) => {
                self.notification = Some(format!("link: {u}"));
            }
        }
    }

    /// Jump to the Hairy view (where new tasks land) and select `id`.
    fn select_id(&mut self, id: &str) {
        if let Some(i) = self
            .views
            .iter()
            .position(|v| v.status == Some(Status::Hairy))
        {
            self.set_view(i);
        }
        if let Some(pos) = self.rows().iter().position(|r| r.task.id == id) {
            self.cursor = pos;
        }
    }

    // -- overlay resolution ----------------------------------------------

    fn handle_overlay_key(&mut self, k: KeyEvent) {
        let ctrl = k.modifiers.contains(KeyModifiers::CONTROL);
        if matches!(self.overlay, Overlay::Drawer(_)) {
            self.handle_drawer_key(k);
            return;
        }
        if matches!(self.overlay, Overlay::Create(_)) {
            self.handle_create_key(k);
            return;
        }
        if matches!(self.overlay, Overlay::ViewPicker(_)) {
            self.handle_view_picker_key(k);
            return;
        }
        // The fuzzy picker: nav/commit/cancel are intercepted; everything else
        // edits the query (and resets the selection to the top match).
        if matches!(self.overlay, Overlay::Fuzzy(_)) {
            let up = k.code == KeyCode::Up || (ctrl && k.code == KeyCode::Char('p'));
            let down = matches!(k.code, KeyCode::Down | KeyCode::Tab)
                || (ctrl && k.code == KeyCode::Char('n'));
            match k.code {
                KeyCode::Esc => {
                    self.overlay = Overlay::None;
                    self.notification = Some("cancelled".into());
                }
                KeyCode::Enter => {
                    if let Overlay::Fuzzy(fp) = std::mem::replace(&mut self.overlay, Overlay::None)
                    {
                        self.commit_fuzzy(fp);
                    }
                }
                _ if up => {
                    if let Overlay::Fuzzy(fp) = &mut self.overlay {
                        fp.sel = fp.sel.saturating_sub(1);
                    }
                }
                _ if down => {
                    let total = match &self.overlay {
                        Overlay::Fuzzy(fp) => fuzzy_total(&self.all, fp),
                        _ => 0,
                    };
                    if let Overlay::Fuzzy(fp) = &mut self.overlay {
                        if total > 0 {
                            fp.sel = (fp.sel + 1).min(total - 1);
                        }
                    }
                }
                _ => {
                    if let Overlay::Fuzzy(fp) = &mut self.overlay {
                        fp.handler.on_key_event(k, &mut fp.query.borrow_mut());
                        fp.sel = 0;
                    }
                }
            }
            return;
        }
        // Inline search: every keystroke edits the live filter for instant
        // preview; Enter keeps it, Esc restores the pre-search query.
        if matches!(self.overlay, Overlay::Search(_)) {
            match k.code {
                KeyCode::Esc => {
                    let saved = match &self.overlay {
                        Overlay::Search(sb) => sb.saved.clone(),
                        _ => None,
                    };
                    self.filter.search = saved;
                    self.overlay = Overlay::None;
                    self.clamp_cursor();
                    self.notification = Some("search cleared".into());
                }
                KeyCode::Enter => {
                    let q = match &self.overlay {
                        Overlay::Search(sb) => sb.query_text(),
                        _ => String::new(),
                    };
                    self.overlay = Overlay::None;
                    self.notification = Some(if q.is_empty() {
                        "search cleared".into()
                    } else {
                        format!("filter: {q}")
                    });
                }
                _ => {
                    if let Overlay::Search(sb) = &mut self.overlay {
                        sb.handler.on_key_event(k, &mut sb.query.borrow_mut());
                    }
                    let q = match &self.overlay {
                        Overlay::Search(sb) => sb.query_text(),
                        _ => String::new(),
                    };
                    self.filter.search = if q.is_empty() { None } else { Some(q) };
                    self.clamp_cursor();
                }
            }
            return;
        }
        // Detail-pane find: live-highlight matches; Enter keeps, Esc restores.
        if matches!(self.overlay, Overlay::DetailFind(_)) {
            match k.code {
                KeyCode::Esc => {
                    let saved = match &self.overlay {
                        Overlay::DetailFind(sb) => sb.saved.clone(),
                        _ => None,
                    };
                    self.detail_find = saved;
                    self.overlay = Overlay::None;
                }
                KeyCode::Enter => self.overlay = Overlay::None,
                _ => {
                    if let Overlay::DetailFind(sb) = &mut self.overlay {
                        sb.handler.on_key_event(k, &mut sb.query.borrow_mut());
                    }
                    let q = match &self.overlay {
                        Overlay::DetailFind(sb) => sb.query_text(),
                        _ => String::new(),
                    };
                    self.detail_find = if q.is_empty() { None } else { Some(q) };
                    self.detail_match = 0;
                    let m = self.detail_find_matches();
                    if let Some(first) = m.first() {
                        self.detail_scroll = first.0 as u16;
                    }
                }
            }
            return;
        }
        // Editors are handled in place (most keys flow to edtui); only the
        // Pick/Confirm variants move their action out on resolution.
        if let Overlay::Edit(ed) = &mut self.overlay {
            let commit = (ctrl && k.code == KeyCode::Char('s'))
                || (ed.single_line && k.code == KeyCode::Enter);
            let cancel = (ctrl && k.code == KeyCode::Char('c'))
                || (ed.single_line && k.code == KeyCode::Esc);
            if commit {
                if let Overlay::Edit(ed) = std::mem::replace(&mut self.overlay, Overlay::None) {
                    self.commit_edit(ed);
                }
            } else if cancel {
                self.overlay = Overlay::None;
                self.notification = Some("cancelled".into());
            } else {
                ed.handler.on_key_event(k, &mut ed.state.borrow_mut());
            }
            return;
        }
        match std::mem::replace(&mut self.overlay, Overlay::None) {
            Overlay::None
            | Overlay::Edit(_)
            | Overlay::Fuzzy(_)
            | Overlay::Search(_)
            | Overlay::Drawer(_)
            | Overlay::Create(_)
            | Overlay::DetailFind(_)
            | Overlay::ViewPicker(_) => {}
            Overlay::Pick {
                prompt,
                keys,
                action,
            } => match k.code {
                KeyCode::Esc => self.notification = Some("cancelled".into()),
                KeyCode::Char(c) if keys.contains(c) => self.resolve_pick(action, c),
                _ => {
                    self.overlay = Overlay::Pick {
                        prompt,
                        keys,
                        action,
                    }
                }
            },
            Overlay::Confirm { prompt, action } => match k.code {
                KeyCode::Char('y') | KeyCode::Char('Y') => self.resolve_confirm(action),
                KeyCode::Char('n') | KeyCode::Char('N') | KeyCode::Esc | KeyCode::Enter => {
                    self.notification = Some("cancelled".into())
                }
                _ => self.overlay = Overlay::Confirm { prompt, action },
            },
        }
    }

    fn commit_edit(&mut self, ed: Editor) {
        let text = ed.text();
        match ed.action {
            EditAction::Labels(id) => {
                let new: Vec<String> = text
                    .split(',')
                    .map(|s| s.trim())
                    .filter(|s| !s.is_empty())
                    .map(String::from)
                    .collect();
                let cur = self.task(&id).map(|t| t.labels.clone()).unwrap_or_default();
                if new == cur {
                    self.notification = Some(format!("{id} labels unchanged"));
                    return;
                }
                let add: Vec<String> = new.iter().filter(|l| !cur.contains(l)).cloned().collect();
                let remove: Vec<String> =
                    cur.iter().filter(|l| !new.contains(l)).cloned().collect();
                let shown = if new.is_empty() {
                    "(none)".to_string()
                } else {
                    new.join(", ")
                };
                self.apply_edit(
                    &id,
                    TaskEdit {
                        add_labels: add,
                        remove_labels: remove,
                        ..Default::default()
                    },
                    format!("{id} labels: {shown}"),
                );
            }
            EditAction::Body(id) => {
                self.apply_edit(
                    &id,
                    TaskEdit {
                        description: Some(text),
                        ..Default::default()
                    },
                    format!("{id} description updated"),
                );
            }
            EditAction::SaveView => self.save_current_view(text),
            EditAction::RenameView { index } => {
                let name = text.trim().to_string();
                if !name.is_empty() && index < self.views.len() {
                    self.views[index].name = name;
                    self.persist_views();
                }
                // Return to the picker on the renamed row.
                self.overlay = Overlay::ViewPicker(index.min(self.views.len().saturating_sub(1)));
            }
        }
    }

    fn commit_fuzzy(&mut self, fp: FuzzyPick) {
        let cands = fuzzy_candidates(&self.all, &fp);
        // Resolve the selection to a target: `None` = clear-parent row,
        // `Some(id)` = a task, or bail if the selection points at nothing.
        let target: Option<Option<String>> = if fp.allow_none {
            if fp.sel == 0 {
                Some(None)
            } else {
                cands.get(fp.sel - 1).map(|t| Some(t.id.clone()))
            }
        } else {
            cands.get(fp.sel).map(|t| Some(t.id.clone()))
        };
        let Some(target) = target else {
            self.notification = Some("nothing selected".into());
            return;
        };
        match fp.action {
            FuzzyAction::AddDep(id) => {
                let Some(dep) = target else { return };
                let Some(h) = &self.herd else { return };
                match h.dep_add(&id, &dep) {
                    Ok(DepOutcome::Added) => {
                        self.reload();
                        self.notification = Some(format!("{id} depends on {dep}"));
                    }
                    Ok(DepOutcome::AlreadyDep) => {
                        self.notification = Some(format!("{id} already depends on {dep}"))
                    }
                    Ok(_) => self.notification = Some("dependency not added".into()),
                    Err(e) => self.notification = Some(format!("error: {e}")),
                }
            }
            FuzzyAction::Reparent(id) => {
                let Some(h) = &self.herd else { return };
                match h.reparent(&id, target.clone()) {
                    Ok(Reparent::Done { new_parent }) => {
                        self.reload();
                        self.notification = Some(match new_parent {
                            Some(p) => format!("{id} reparented under {p}"),
                            None => format!("{id} moved to top level"),
                        });
                    }
                    Ok(Reparent::Error(m)) => self.notification = Some(m),
                    Err(e) => self.notification = Some(format!("error: {e}")),
                }
            }
        }
    }

    fn resolve_pick(&mut self, action: PickAction, c: char) {
        match action {
            PickAction::State(id) => {
                let dest = match c {
                    'h' => Status::Hairy,
                    's' => Status::Shaving,
                    'n' => Status::Shorn,
                    'x' => Status::Dead,
                    _ => return,
                };
                if self.task(&id).map(|t| t.status) == Some(dest) {
                    self.notification = Some(format!("{id} already {}", status_word(dest)));
                    return;
                }
                let Some(h) = &self.herd else { return };
                match h.transition(&id, dest) {
                    Ok(MoveOutcome::Moved) => {
                        self.reload();
                        self.notification = Some(format!("{id} → {}", status_word(dest)));
                    }
                    Ok(MoveOutcome::AlreadyThere) => {
                        self.notification = Some(format!("{id} already {}", status_word(dest)))
                    }
                    Ok(MoveOutcome::NotFound) => {
                        self.notification = Some(format!("{id} not found"))
                    }
                    Err(e) => self.notification = Some(format!("error: {e}")),
                }
            }
            PickAction::Priority(id) => {
                let Some(p) = c.to_digit(10).map(|d| d as u8) else {
                    return;
                };
                if self.task(&id).map(|t| t.priority) == Some(p) {
                    self.notification = Some(format!("{id} already p{p}"));
                    return;
                }
                self.apply_edit(
                    &id,
                    TaskEdit {
                        priority: Some(p),
                        ..Default::default()
                    },
                    format!("{id} → p{p}"),
                );
            }
            PickAction::Type(id) => {
                let kind = match c {
                    't' => "task",
                    'b' => "bug",
                    'f' => "feature",
                    'i' => "idea",
                    _ => return,
                };
                if self.task(&id).map(|t| t.kind.as_str()) == Some(kind) {
                    self.notification = Some(format!("{id} already {kind}"));
                    return;
                }
                self.apply_edit(
                    &id,
                    TaskEdit {
                        kind: Some(kind.to_string()),
                        ..Default::default()
                    },
                    format!("{id} → {kind}"),
                );
            }
        }
    }

    fn apply_edit(&mut self, id: &str, edit: TaskEdit, ok_msg: String) {
        let Some(h) = &self.herd else { return };
        match h.update(id, edit) {
            Ok(UpdateOutcome::Updated) => {
                self.reload();
                self.notification = Some(ok_msg);
            }
            Ok(UpdateOutcome::NoChanges) => self.notification = Some(format!("{id} unchanged")),
            Ok(UpdateOutcome::NotFound) => self.notification = Some(format!("{id} not found")),
            Err(e) => self.notification = Some(format!("error: {e}")),
        }
    }

    fn resolve_confirm(&mut self, action: ConfirmAction) {
        match action {
            ConfirmAction::Slaughter(id) => {
                let Some(h) = &self.herd else { return };
                match h.transition(&id, Status::Dead) {
                    Ok(MoveOutcome::Moved) => {
                        self.reload();
                        self.notification = Some(format!("slaughtered {id}"));
                    }
                    Ok(_) => self.notification = Some(format!("{id} not slaughtered")),
                    Err(e) => self.notification = Some(format!("error: {e}")),
                }
            }
        }
    }
}

pub fn run(mut app: App) -> Result<()> {
    let (mut term, kitty) = setup()?;
    let res = event_loop(&mut term, &mut app);
    let _ = restore(kitty);
    res
}

fn event_loop(term: &mut Terminal<CrosstermBackend<Stdout>>, app: &mut App) -> Result<()> {
    loop {
        term.draw(|f| render(app, f))?;
        // Record the main-area height (minus tab + help lines) for paging.
        app.page = term.size()?.height.saturating_sub(2).max(1);
        if let Event::Key(k) = event::read()? {
            if k.kind == KeyEventKind::Press {
                handle_key(app, k);
            }
        }
        if app.quit {
            break;
        }
    }
    Ok(())
}

fn handle_key(app: &mut App, k: KeyEvent) {
    let ctrl = k.modifiers.contains(KeyModifiers::CONTROL);
    // A modal prompt swallows all other input until resolved (including Ctrl-C,
    // which an editor treats as cancel rather than quitting the app).
    if !matches!(app.overlay, Overlay::None) {
        app.handle_overlay_key(k);
        return;
    }
    if ctrl && k.code == KeyCode::Char('c') {
        app.quit = true;
        return;
    }
    let half = (app.page / 2).max(1) as i32;
    let full = app.page.max(1) as i32;
    match app.focus {
        Focus::List => match k.code {
            KeyCode::Char('q') => app.quit = true,
            KeyCode::Char('j') | KeyCode::Down => app.move_cursor(1),
            KeyCode::Char('k') | KeyCode::Up => app.move_cursor(-1),
            KeyCode::Char('g') => app.cursor = 0,
            KeyCode::Char('G') => app.move_cursor(i32::MAX / 4),
            KeyCode::Char('d') => app.move_cursor(half),
            KeyCode::Char('u') => app.move_cursor(-half),
            KeyCode::PageDown => app.move_cursor(full),
            KeyCode::PageUp => app.move_cursor(-full),
            KeyCode::Tab | KeyCode::Char(']') => app.switch_tab(1),
            KeyCode::BackTab | KeyCode::Char('[') => app.switch_tab(-1),
            KeyCode::Char(' ') => app.toggle_collapse(),
            // Mutations (single-key pickers + confirm).
            KeyCode::Char('S') => app.open_state_picker(),
            KeyCode::Char('P') => app.open_priority_picker(),
            KeyCode::Char('T') => app.open_type_picker(),
            KeyCode::Char('X') => app.open_slaughter_confirm(),
            KeyCode::Char('L') => app.open_labels(),
            KeyCode::Char('c') => app.open_create(false),
            KeyCode::Char('C') => app.open_create(true),
            KeyCode::Char('E') => app.open_body_edit(),
            KeyCode::Char('D') => app.open_dep_picker(),
            KeyCode::Char('R') => app.open_reparent_picker(),
            KeyCode::Char('/') => app.open_search(),
            KeyCode::Char('f') => app.open_drawer(),
            KeyCode::Char('*') => app.toggle_star(),
            KeyCode::Char('v') => app.open_view_picker(),
            KeyCode::Char('V') => app.open_save_view(),
            KeyCode::Esc => {
                if app.is_view_modified() {
                    app.revert_filter_to_view();
                    app.notification = Some("reverted to view".into());
                }
            }
            KeyCode::Char('l') | KeyCode::Right | KeyCode::Enter => {
                if app.selected().is_some() {
                    app.focus = Focus::Detail;
                    app.detail_scroll = 0;
                    app.detail_link = 0;
                    app.detail_find = None;
                    app.detail_match = 0;
                }
            }
            _ => {}
        },
        Focus::Detail => match k.code {
            KeyCode::Char('q') => app.quit = true,
            KeyCode::Char('h') | KeyCode::Left | KeyCode::Esc => app.focus = Focus::List,
            KeyCode::Tab | KeyCode::Char(']') => app.jump_link(1),
            KeyCode::BackTab | KeyCode::Char('[') => app.jump_link(-1),
            KeyCode::Char('/') => app.open_detail_find(),
            KeyCode::Char('n') => app.detail_find_jump(1),
            KeyCode::Char('N') => app.detail_find_jump(-1),
            KeyCode::Enter => app.follow_link(),
            KeyCode::Char('j') | KeyCode::Down => {
                app.detail_scroll = app.detail_scroll.saturating_add(1)
            }
            KeyCode::Char('k') | KeyCode::Up => {
                app.detail_scroll = app.detail_scroll.saturating_sub(1)
            }
            KeyCode::Char('d') => app.detail_scroll = app.detail_scroll.saturating_add(half as u16),
            KeyCode::Char('u') => app.detail_scroll = app.detail_scroll.saturating_sub(half as u16),
            KeyCode::Char('g') => app.detail_scroll = 0,
            _ => {}
        },
    }
}

fn render(app: &App, frame: &mut Frame) {
    // Tab row, a blank gap, the main area, then the help bar (Python layout).
    let [top, _gap, mid, bot] = Layout::vertical([
        Constraint::Length(1),
        Constraint::Length(1),
        Constraint::Min(0),
        Constraint::Length(1),
    ])
    .areas(frame.area());
    render_tabs(app, frame, top);
    // The list is full-width until the detail pane or a right-pane overlay is
    // shown. We intentionally keep the filter drawer / fuzzy picker / view
    // picker on the RIGHT (diverging from Python's top drawer): in a wide, short
    // terminal a right-side drawer wastes no rows. Every right-pane surface gets
    // the same left divider (via right_divider) so it doesn't bleed into the list.
    let right_overlay = matches!(&app.overlay, Overlay::Edit(ed) if !ed.single_line)
        || matches!(
            app.overlay,
            Overlay::Fuzzy(_) | Overlay::Drawer(_) | Overlay::Create(_) | Overlay::ViewPicker(_)
        );
    if app.focus == Focus::Detail || right_overlay {
        let [left, right] =
            Layout::horizontal([Constraint::Percentage(34), Constraint::Percentage(66)]).areas(mid);
        render_list(app, frame, left);
        let inner = right_divider(frame, right, true);
        match &app.overlay {
            Overlay::Edit(ed) if !ed.single_line => render_editor_panel(ed, frame, inner),
            Overlay::Fuzzy(fp) => render_fuzzy_results(app, fp, frame, inner),
            Overlay::Drawer(d) => render_drawer(d, frame, inner),
            Overlay::Create(f) => render_create(f, frame, inner),
            Overlay::ViewPicker(sel) => render_view_picker(app, *sel, frame, inner),
            _ => render_detail(app, frame, inner),
        }
    } else {
        render_list(app, frame, mid);
    }
    render_status(app, frame, bot);
}

/// Draw the shared left-divider rule (as the detail pane has) on a right-pane
/// area and return the inset content region. Applied to detail AND every
/// right-pane overlay so drawers don't bleed into the list.
fn right_divider(frame: &mut Frame, area: Rect, focused: bool) -> Rect {
    let block = Block::new()
        .borders(Borders::LEFT)
        .border_style(if focused {
            Style::new().fg(Color::Cyan)
        } else {
            Style::new().fg(Color::DarkGray)
        })
        .padding(Padding::horizontal(1));
    let inner = block.inner(area);
    frame.render_widget(block, area);
    inner
}

fn render_view_picker(app: &App, sel: usize, frame: &mut Frame, area: Rect) {
    let [head, body] = Layout::vertical([Constraint::Length(1), Constraint::Min(0)]).areas(area);
    frame.render_widget(
        Paragraph::new(Span::styled(
            "Views",
            Style::new().fg(Color::Cyan).add_modifier(Modifier::BOLD),
        )),
        head,
    );
    let items: Vec<ListItem> = app
        .views
        .iter()
        .enumerate()
        .map(|(i, v)| {
            let active = if i == app.view { "▸" } else { " " };
            let pin = if v.pinned { "*" } else { " " };
            let lock = if v.builtin { "  (builtin)" } else { "" };
            ListItem::new(Line::from(vec![
                Span::styled(format!("{active} "), Style::new().fg(Color::Cyan)),
                Span::styled(format!("{pin} "), Style::new().fg(Color::Yellow)),
                Span::raw(v.name.clone()),
                Span::styled(
                    format!("  ({})", app.view_count(v)),
                    Style::new().fg(Color::DarkGray),
                ),
                Span::styled(lock.to_string(), Style::new().fg(Color::DarkGray)),
            ]))
        })
        .collect();
    let mut state = ListState::default();
    if !app.views.is_empty() {
        state.select(Some(sel.min(app.views.len() - 1)));
    }
    frame.render_stateful_widget(
        List::new(items).highlight_style(Style::new().fg(Color::Black).bg(Color::Cyan)),
        body,
        &mut state,
    );
}

fn render_drawer(d: &Drawer, frame: &mut Frame, area: Rect) {
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
    let deps: Vec<(String, bool)> = vec![("ready".into(), d.ready), ("tangled".into(), d.tangled)];
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

/// The create-task form: header + title / type / priority / labels /
/// description rows, laid out like `render_drawer` and inset by `right_divider`.
fn render_create(f: &CreateForm, frame: &mut Frame, area: Rect) {
    let rows = Layout::vertical([
        Constraint::Length(1), // header
        Constraint::Length(1), // title
        Constraint::Length(1), // type chips
        Constraint::Length(1), // priority chips
        Constraint::Length(1), // labels
        Constraint::Length(1), // description
        Constraint::Min(0),
    ])
    .split(area);
    let header = match &f.parent {
        Some(p) => format!("New task (child of {p})"),
        None => "New yak".to_string(),
    };
    frame.render_widget(
        Paragraph::new(Span::styled(
            header,
            Style::new().fg(Color::Cyan).add_modifier(Modifier::BOLD),
        )),
        rows[0],
    );
    let types: Vec<(String, bool)> = TYPE_CHOICES
        .iter()
        .enumerate()
        .map(|(i, &k)| (k.to_string(), i == f.kind_idx))
        .collect();
    let pris: Vec<(String, bool)> = PRI_CHOICES
        .iter()
        .enumerate()
        .map(|(i, &p)| (format!("p{p}"), i == f.pri_idx))
        .collect();
    render_text_row(
        f.row == 0,
        CREATE_LABEL_W,
        "title",
        &f.title,
        "",
        frame,
        rows[1],
    );
    render_chip_row(
        f.row == 1,
        f.kind_idx,
        CREATE_LABEL_W,
        "type",
        &types,
        frame,
        rows[2],
    );
    render_chip_row(
        f.row == 2,
        f.pri_idx,
        CREATE_LABEL_W,
        "priority",
        &pris,
        frame,
        rows[3],
    );
    render_text_row(
        f.row == 3,
        CREATE_LABEL_W,
        "labels",
        &f.labels,
        "",
        frame,
        rows[4],
    );
    render_text_row(
        f.row == 4,
        CREATE_LABEL_W,
        "description",
        &f.description,
        "",
        frame,
        rows[5],
    );
}

/// Label-column widths (gutter marker excluded): the drawer's longest label is
/// `priority` (8); the create form's is `description` (11). Each leaves one
/// trailing space before the chips/field.
const DRAWER_LABEL_W: usize = 9;
const CREATE_LABEL_W: usize = 12;

/// Render one chip row: a `▸` gutter, a padded label, then space-separated
/// chips. `current_row` highlights this row; `cursor_idx` is the chip the
/// cursor sits on (drawn REVERSED); each choice's bool marks it selected
/// (green bold). For single-select forms the cursor and the selection coincide.
fn render_chip_row(
    current_row: bool,
    cursor_idx: usize,
    label_w: usize,
    label: &str,
    choices: &[(String, bool)],
    frame: &mut Frame,
    area: Rect,
) {
    let mut spans = vec![
        Span::styled(
            if current_row { "▸ " } else { "  " },
            Style::new().fg(Color::Cyan),
        ),
        Span::styled(
            format!("{label:<label_w$}"),
            Style::new().fg(Color::DarkGray),
        ),
    ];
    for (j, (disp, sel)) in choices.iter().enumerate() {
        let mut style = if *sel {
            Style::new().fg(Color::Green).add_modifier(Modifier::BOLD)
        } else {
            Style::new().fg(Color::DarkGray)
        };
        if current_row && cursor_idx == j {
            style = style.add_modifier(Modifier::REVERSED);
        }
        spans.push(Span::raw(" "));
        spans.push(Span::styled(disp.clone(), style));
    }
    frame.render_widget(Paragraph::new(Line::from(spans)), area);
}

/// Render one text-facet row: a `▸` gutter, a padded label, then either a live
/// edtui field (when `current`) or the dimmed current value / `placeholder`
/// (shown when the field is empty and unfocused).
fn render_text_row(
    current: bool,
    label_w: usize,
    label: &str,
    cell: &RefCell<EditorState>,
    placeholder: &str,
    frame: &mut Frame,
    area: Rect,
) {
    let [g, lab, fld] = Layout::horizontal([
        Constraint::Length(2),
        Constraint::Length(label_w as u16),
        Constraint::Min(0),
    ])
    .areas(area);
    frame.render_widget(
        Paragraph::new(Span::styled(
            if current { "▸ " } else { "  " },
            Style::new().fg(Color::Cyan),
        )),
        g,
    );
    frame.render_widget(
        Paragraph::new(Span::styled(
            label.to_string(),
            Style::new().fg(Color::DarkGray),
        )),
        lab,
    );
    if current {
        let mut st = cell.borrow_mut();
        frame.render_widget(
            EditorView::new(&mut st)
                .theme(editor_theme())
                .single_line(true),
            fld,
        );
    } else {
        let text = cell.borrow().lines.to_string();
        let shown = if text.is_empty() {
            placeholder.to_string()
        } else {
            text
        };
        frame.render_widget(
            Paragraph::new(Span::styled(shown, Style::new().fg(Color::DarkGray))),
            fld,
        );
    }
}

fn editor_theme() -> EditorTheme<'static> {
    EditorTheme::default()
        .hide_status_line()
        .block(Block::default())
}

fn render_editor_panel(ed: &Editor, frame: &mut Frame, area: Rect) {
    let [head, body] = Layout::vertical([Constraint::Length(1), Constraint::Min(0)]).areas(area);
    frame.render_widget(
        Paragraph::new(Span::styled(
            ed.label.clone(),
            Style::new().fg(Color::Cyan).add_modifier(Modifier::BOLD),
        )),
        head,
    );
    let mut state = ed.state.borrow_mut();
    frame.render_widget(EditorView::new(&mut state).theme(editor_theme()), body);
}

/// Render `label` + a single-line edtui field across one row.
fn render_query_line(label: &str, state: &RefCell<EditorState>, frame: &mut Frame, area: Rect) {
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
    frame.render_widget(
        EditorView::new(&mut st)
            .theme(editor_theme())
            .single_line(true),
        fld,
    );
}

fn render_line_editor(ed: &Editor, frame: &mut Frame, area: Rect) {
    render_query_line(&ed.label, &ed.state, frame, area);
}

fn render_fuzzy_results(app: &App, fp: &FuzzyPick, frame: &mut Frame, area: Rect) {
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
        items.push(ListItem::new(Line::from(vec![
            Span::styled(
                format!("[{}] ", t.status.glyph()),
                Style::new().fg(Color::DarkGray),
            ),
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
        List::new(items).highlight_style(Style::new().fg(Color::Black).bg(Color::Cyan)),
        body,
        &mut state,
    );
}

fn overlay_name(o: &Overlay) -> &'static str {
    match o {
        Overlay::None => "none",
        Overlay::Pick { .. } => "pick",
        Overlay::Confirm { .. } => "confirm",
        Overlay::Edit(_) => "edit",
        Overlay::Fuzzy(_) => "fuzzy",
        Overlay::Search(_) => "search",
        Overlay::Drawer(_) => "drawer",
        Overlay::Create(_) => "create",
        Overlay::DetailFind(_) => "detail-find",
        Overlay::ViewPicker(_) => "view-picker",
    }
}

/// Emoji status glyph, matching the Python TUI (bison/razor/sheep/skull).
fn status_emoji(s: Status) -> &'static str {
    match s {
        Status::Hairy => "\u{1f9ac}",
        Status::Shaving => "\u{1fa92}",
        Status::Shorn => "\u{1f411}",
        Status::Dead => "\u{1f480}",
    }
}

/// Displayed count, capped like Python's format_count (unbounded views).
fn format_count(n: usize) -> String {
    if n <= 999 {
        n.to_string()
    } else {
        "999+".into()
    }
}

/// Approximate display width: emoji (and other astral glyphs) are width 2.
fn disp_width(s: &str) -> usize {
    s.chars()
        .map(|c| {
            let cp = c as u32;
            if cp >= 0x1_F000 || cp == 0x2b50 || cp == 0x2764 {
                2
            } else {
                1
            }
        })
        .sum()
}

fn status_word(s: Status) -> &'static str {
    match s {
        Status::Hairy => "hairy",
        Status::Shaving => "shaving",
        Status::Shorn => "shorn",
        Status::Dead => "dead",
    }
}

fn render_tabs(app: &App, frame: &mut Frame, area: Rect) {
    // ` {emoji name}{*} ({count}) ` per pinned view, active black-on-white bold,
    // others dim; a trailing filter indicator when the live filter is forked.
    let mut spans: Vec<Span> = Vec::new();
    for &i in &app.pinned_indices() {
        let v = &app.views[i];
        let mark = if i == app.view && app.is_view_modified() {
            "*"
        } else {
            ""
        };
        let text = format!(" {}{} ({}) ", v.name, mark, format_count(app.view_count(v)));
        let style = if i == app.view {
            Style::new()
                .fg(Color::Black)
                .bg(Color::White)
                .add_modifier(Modifier::BOLD)
        } else {
            Style::new().add_modifier(Modifier::DIM)
        };
        spans.push(Span::styled(text, style));
        spans.push(Span::raw(" "));
    }
    if app.is_view_modified() {
        spans.push(Span::styled(
            format!(" filter: {}", filter_summary(&app.filter)),
            Style::new().fg(Color::Yellow),
        ));
    }
    frame.render_widget(Paragraph::new(Line::from(spans)), area);
    // Notification, right-aligned on the tab row (Python placement).
    if let Some(n) = &app.notification {
        let w = (disp_width(n) as u16).min(area.width);
        if w > 0 {
            let rect = Rect {
                x: area.x + area.width - w,
                y: area.y,
                width: w,
                height: 1,
            };
            frame.render_widget(
                Paragraph::new(Span::styled(
                    n.clone(),
                    Style::new().fg(Color::Yellow).add_modifier(Modifier::BOLD),
                )),
                rect,
            );
        }
    }
}

fn render_list(app: &App, frame: &mut Frame, area: Rect) {
    let focused = app.focus == Focus::List;
    let rows = app.rows();
    if rows.is_empty() {
        frame.render_widget(
            Paragraph::new(Span::styled(
                "No tasks.",
                Style::new().add_modifier(Modifier::DIM),
            )),
            area,
        );
        return;
    }
    // Dynamic id column: widest `id + 2*depth` across visible rows (min 4).
    let max_id_len = rows
        .iter()
        .map(|r| r.task.id.chars().count() + 2 * r.depth as usize)
        .max()
        .unwrap_or(4)
        .max(4);
    let id_field_w = max_id_len + 2;
    let blocked = app.blocked_ids();
    let items: Vec<ListItem> = rows
        .iter()
        .map(|r| {
            list_item(
                r,
                id_field_w,
                blocked.contains(&r.task.id),
                app.is_starred(&r.task.id),
                area.width,
            )
        })
        .collect();
    let mut state = ListState::default();
    state.select(Some(app.cursor.min(rows.len() - 1)));
    let hl = if focused {
        Style::new().fg(Color::Black).bg(Color::Cyan)
    } else {
        Style::new().add_modifier(Modifier::REVERSED)
    };
    frame.render_stateful_widget(List::new(items).highlight_style(hl), area, &mut state);
}

/// Truncate `s` to a maximum display width (emoji counted as 2).
fn truncate_disp(s: &str, max: usize) -> String {
    if disp_width(s) <= max {
        return s.to_string();
    }
    let mut out = String::new();
    let mut w = 0;
    for c in s.chars() {
        let cw = if (c as u32) >= 0x1_F000 || c as u32 == 0x2b50 {
            2
        } else {
            1
        };
        if w + cw > max {
            break;
        }
        out.push(c);
        w += cw;
    }
    out
}

fn list_item<'a>(
    r: &tree::Row<'a>,
    id_field_w: usize,
    blocked: bool,
    starred: bool,
    width: u16,
) -> ListItem<'a> {
    let dim = |st: Style| {
        if r.ghost {
            st.add_modifier(Modifier::DIM)
        } else {
            st
        }
    };
    let width = width as usize;
    let indent = "  ".repeat(r.depth as usize);
    let lead = if blocked { "*" } else { " " };
    let body = format!("{indent}{}", r.task.id);
    let body_w = id_field_w.saturating_sub(1);
    let body_padded = format!("{body:<body_w$}");
    let pri_s = format!("p{} ", r.task.priority);
    let type_s = format!("{:8} ", r.task.kind);

    // Right side: right-aligned labels, a star, and a collapse badge.
    let max_lw = (width / 4).clamp(8, 30);
    let label_str = if r.task.labels.is_empty() {
        String::new()
    } else {
        truncate_disp(&format!("[{}]", r.task.labels.join(", ")), max_lw)
    };
    let badge = if r.collapsed && r.hidden > 0 {
        format!(" \u{25b6} {} ", r.hidden)
    } else {
        String::new()
    };
    let mut right = String::new();
    if !label_str.is_empty() {
        right.push_str(&label_str);
    }
    if starred {
        if !right.is_empty() {
            right.push(' ');
        }
        right.push('\u{2b50}');
    }
    right.push_str(&badge);
    let rw = disp_width(&right);

    let left_fixed = 1 + disp_width(&body_padded) + disp_width(&pri_s) + disp_width(&type_s);
    let title_avail = width.saturating_sub(left_fixed + rw + 1);
    let title = truncate_disp(
        &format!("{} {}", status_emoji(r.task.status), r.task.title),
        title_avail,
    );
    let used = left_fixed + disp_width(&title);
    let pad = width.saturating_sub(used + rw);

    let mut spans = vec![
        Span::styled(
            lead.to_string(),
            if blocked {
                Style::new().fg(Color::Magenta).add_modifier(Modifier::BOLD)
            } else {
                Style::new()
            },
        ),
        Span::styled(body_padded, dim(Style::new().fg(Color::Blue))),
        Span::styled(pri_s, dim(Style::new().fg(Color::DarkGray))),
        Span::styled(type_s, dim(Style::new().fg(Color::Rgb(181, 137, 0)))),
        Span::styled(title, dim(Style::new())),
    ];
    if pad > 0 {
        spans.push(Span::raw(" ".repeat(pad)));
    }
    if !right.is_empty() {
        spans.push(Span::styled(right, dim(Style::new().fg(Color::DarkGray))));
    }
    ListItem::new(Line::from(spans))
}

fn render_detail(app: &App, frame: &mut Frame, area: Rect) {
    let focused = app.focus == Focus::Detail;
    // The left divider is drawn by render() via right_divider(); we render the
    // content into the already-inset area.
    let Some(t) = app.selected() else {
        let p = Paragraph::new(Span::styled("(no task)", Style::new().fg(Color::DarkGray)));
        frame.render_widget(p, area);
        return;
    };
    let lines = detail::build(t, &app.all);
    let jumps = detail::jumplist(&lines);
    let cur = if focused {
        jumps.get(app.detail_link)
    } else {
        None
    };
    let matches = if focused {
        app.detail_find_matches()
    } else {
        vec![]
    };
    let rendered: Vec<Line> = lines
        .iter()
        .enumerate()
        .map(|(i, dl)| {
            let lm: Vec<(usize, usize, bool)> = matches
                .iter()
                .enumerate()
                .filter(|(_, (ln, _, _))| *ln == i)
                .map(|(mi, (_, col, len))| (*col, *len, mi == app.detail_match))
                .collect();
            render_dline(dl, cur, i, &lm)
        })
        .collect();
    // No wrap: link/match highlight columns must stay valid.
    let p = Paragraph::new(rendered).scroll((app.detail_scroll, 0));
    frame.render_widget(p, area);
}

/// Render one detail line by computing a per-char style (base -> link ->
/// find-match, each overriding the last) and coalescing equal runs into spans.
fn render_dline<'a>(
    dl: &'a detail::DLine,
    cur: Option<&detail::Jump>,
    line_idx: usize,
    matches: &[(usize, usize, bool)],
) -> Line<'a> {
    let chars: Vec<char> = dl.text.chars().collect();
    let n = chars.len();
    if n == 0 {
        return Line::from(String::new());
    }
    // Base per-char style: dim label prefix on plain fields; section headers cyan.
    let label_end = if dl.links.is_empty() && dl.kind == detail::Kind::Field && n > 13 {
        13
    } else {
        0
    };
    let mut styles: Vec<Style> = (0..n)
        .map(|i| {
            if i < label_end {
                Style::new().fg(Color::DarkGray)
            } else if dl.kind == detail::Kind::Section {
                Style::new().fg(Color::Cyan).add_modifier(Modifier::BOLD)
            } else {
                Style::new()
            }
        })
        .collect();
    for (col, len, _) in &dl.links {
        let is_current = cur.is_some_and(|j| j.line == line_idx && j.col == *col);
        let st = if is_current {
            Style::new().fg(Color::Black).bg(Color::Cyan)
        } else {
            Style::new()
                .fg(Color::Blue)
                .add_modifier(Modifier::UNDERLINED)
        };
        for s in styles.iter_mut().take((col + len).min(n)).skip(*col) {
            *s = st;
        }
    }
    for (col, len, is_current) in matches {
        let st = if *is_current {
            Style::new()
                .fg(Color::Black)
                .bg(Color::Green)
                .add_modifier(Modifier::BOLD)
        } else {
            Style::new().fg(Color::Black).bg(Color::Yellow)
        };
        for s in styles.iter_mut().take((col + len).min(n)).skip(*col) {
            *s = st;
        }
    }
    let mut spans: Vec<Span> = Vec::new();
    let mut i = 0;
    while i < n {
        let st = styles[i];
        let start = i;
        while i < n && styles[i] == st {
            i += 1;
        }
        spans.push(Span::styled(chars[start..i].iter().collect::<String>(), st));
    }
    Line::from(spans)
}

/// (line, col, len) of every case-insensitive occurrence of `q` in the lines.
fn detail_scan(lines: &[detail::DLine], q: &str) -> Vec<(usize, usize, usize)> {
    let ql = q.to_lowercase();
    if ql.is_empty() {
        return vec![];
    }
    let qlen = ql.chars().count();
    let mut out = Vec::new();
    for (i, l) in lines.iter().enumerate() {
        let text = l.text.to_lowercase();
        let mut from = 0usize;
        while let Some(pos) = text[from..].find(&ql) {
            let byte = from + pos;
            let col = text[..byte].chars().count();
            out.push((i, col, qlen));
            from = byte + ql.len();
        }
    }
    out
}

fn render_status(app: &App, frame: &mut Frame, area: Rect) {
    // A single-line editor field owns the status line while active.
    if let Overlay::Edit(ed) = &app.overlay {
        if ed.single_line {
            render_line_editor(ed, frame, area);
            return;
        }
    }
    // The fuzzy picker's query line owns the status line too.
    if let Overlay::Fuzzy(fp) = &app.overlay {
        render_query_line("search: ", &fp.query, frame, area);
        return;
    }
    // Inline search field.
    if let Overlay::Search(sb) = &app.overlay {
        render_query_line("/", &sb.query, frame, area);
        return;
    }
    // Detail-pane find field.
    if let Overlay::DetailFind(sb) = &app.overlay {
        render_query_line("find: ", &sb.query, frame, area);
        return;
    }
    // View manager help hint.
    if matches!(&app.overlay, Overlay::ViewPicker(_)) {
        frame.render_widget(
            Paragraph::new(Span::styled(
                "Enter open · p pin · J/K move · r rename · d delete · Esc close",
                Style::new().fg(Color::DarkGray),
            )),
            area,
        );
        return;
    }
    // Drawer help hint.
    if matches!(&app.overlay, Overlay::Drawer(_)) {
        frame.render_widget(
            Paragraph::new(Span::styled(
                "↑↓/Tab rows · ←→ chips · Space toggle · C clear · Enter apply · Esc cancel",
                Style::new().fg(Color::DarkGray),
            )),
            area,
        );
        return;
    }
    // Create-form help hint. The `(need title)` marker mirrors Python's guard.
    if let Overlay::Create(f) = &app.overlay {
        let commit = if f.title_text().is_empty() {
            "(need title)"
        } else {
            "Enter create"
        };
        frame.render_widget(
            Paragraph::new(Span::styled(
                format!("↑↓/Tab rows · ←→ chips · {commit} · Esc cancel"),
                Style::new().fg(Color::DarkGray),
            )),
            area,
        );
        return;
    }
    // Otherwise: an active modal prompt, else a transient notification, else the
    // context help hint. (A multi-line editor falls through to notification/help.)
    // Otherwise: a single-key modal prompt, else the context help bar.
    // (Notification + active-filter indicator now live on the tab row.)
    let (text, style) = match &app.overlay {
        Overlay::Pick { prompt, .. } | Overlay::Confirm { prompt, .. } => (
            prompt.clone(),
            Style::new()
                .fg(Color::Black)
                .bg(Color::Yellow)
                .add_modifier(Modifier::BOLD),
        ),
        _ => (help_hint(app), Style::new().fg(Color::DarkGray)),
    };
    frame.render_widget(Paragraph::new(Span::styled(text, style)), area);
}

/// One-line description of the active content facets (for the status line).
fn filter_summary(f: &FilterSpec) -> String {
    let mut parts: Vec<String> = Vec::new();
    if let Some(s) = f.search.as_deref().filter(|s| !s.is_empty()) {
        parts.push(format!("\u{201c}{s}\u{201d}"));
    }
    if !f.types.is_empty() {
        parts.push(f.types.join("|"));
    }
    if !f.priorities.is_empty() {
        parts.push(
            f.priorities
                .iter()
                .map(|p| format!("p{p}"))
                .collect::<Vec<_>>()
                .join("|"),
        );
    }
    if !f.labels.is_empty() {
        parts.push(f.labels.join(","));
    }
    if f.ready_only {
        parts.push("ready".into());
    }
    if f.tangled_only {
        parts.push("tangled".into());
    }
    if parts.is_empty() {
        "(all)".into()
    } else {
        parts.join(" · ")
    }
}

fn help_hint(app: &App) -> String {
    let filter_hint = if app.filter.content_active() {
        "f:filter  Esc:clear"
    } else {
        "f:filter  /:search"
    };
    match app.focus {
        Focus::List => format!(
            "Tab:view  j/k:move  l:detail  v:views  c/C:new  E:edit  X:del  S:state  D:dep  {filter_hint}  ?:help"
        ),
        Focus::Detail => format!(
            "h:list  j/k:move  Tab:link  Enter:follow  i/o:fwd/back  E:edit  D:dep  S:state  {filter_hint}  q:quit"
        ),
    }
}

// -- terminal lifecycle ---------------------------------------------------

fn setup() -> Result<(Terminal<CrosstermBackend<Stdout>>, bool)> {
    enable_raw_mode()?;
    let mut out = io::stdout();
    execute!(out, EnterAlternateScreen)?;
    let kitty = supports_keyboard_enhancement().unwrap_or(false);
    if kitty {
        let _ = execute!(
            out,
            PushKeyboardEnhancementFlags(KeyboardEnhancementFlags::DISAMBIGUATE_ESCAPE_CODES)
        );
    }
    let term = Terminal::new(CrosstermBackend::new(out))?;
    let prev = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |info| {
        let _ = restore(kitty);
        prev(info);
    }));
    Ok((term, kitty))
}

fn restore(kitty: bool) -> Result<()> {
    let mut out = io::stdout();
    if kitty {
        let _ = execute!(out, PopKeyboardEnhancementFlags);
    }
    execute!(out, LeaveAlternateScreen)?;
    disable_raw_mode()?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use ratatui::backend::TestBackend;

    fn task(id: &str, title: &str, status: Status, priority: u8, parent: Option<&str>) -> Task {
        Task {
            id: id.into(),
            title: title.into(),
            kind: "task".into(),
            priority,
            status,
            created: None,
            updated: None,
            parent: parent.map(String::from),
            labels: vec![],
            depends_on: vec![],
            source: None,
            body: String::new(),
        }
    }

    fn buffer_to_string(buf: &ratatui::buffer::Buffer) -> String {
        let area = buf.area;
        let mut s = String::new();
        for y in 0..area.height {
            let mut line = String::new();
            for x in 0..area.width {
                line.push_str(buf[(x, y)].symbol());
            }
            s.push_str(line.trim_end());
            s.push('\n');
        }
        s
    }

    fn draw(app: &App, w: u16, h: u16) -> String {
        let mut term = Terminal::new(TestBackend::new(w, h)).unwrap();
        term.draw(|f| render(app, f)).unwrap();
        buffer_to_string(term.backend().buffer())
    }

    fn key(c: char) -> KeyEvent {
        KeyEvent::new(KeyCode::Char(c), KeyModifiers::NONE)
    }

    fn sample() -> App {
        // root-a (hairy) with children a1 (hairy) + a2 (shorn, ghost in Hairy tab);
        // root-b (shaving, not in Hairy universe).
        App::new(vec![
            task("a0", "Root A", Status::Hairy, 2, None),
            task("a1", "Child A1", Status::Hairy, 3, Some("a0")),
            task("a2", "Child A2 done", Status::Shorn, 3, Some("a0")),
            task("b0", "Root B shaving", Status::Shaving, 1, None),
        ])
    }

    #[test]
    fn tree_with_ghost_family() {
        // Hairy tab: Root A + A1 (focus), A2 (ghost, shorn) pulled in as family.
        insta::assert_snapshot!(draw(&sample(), 72, 14));
    }

    #[test]
    fn collapsed_root_hides_children() {
        let mut app = sample();
        app.collapsed.insert("a0".into());
        insta::assert_snapshot!(draw(&app, 72, 14));
    }

    #[test]
    fn build_universe_pulls_ghost_descendants() {
        let app = sample();
        let rows = app.rows();
        let ids: Vec<&str> = rows.iter().map(|r| r.task.id.as_str()).collect();
        assert_eq!(ids, vec!["a0", "a1", "a2"]); // b0 (shaving) excluded from Hairy tab
        let a2 = rows.iter().find(|r| r.task.id == "a2").unwrap();
        assert!(a2.ghost, "shorn child should be a ghost in the Hairy tab");
        let a0 = rows.iter().find(|r| r.task.id == "a0").unwrap();
        assert!(a0.has_children && !a0.ghost);
    }

    // -- overlay rendering (herd-less; opening only needs `selected`) ------

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

    #[test]
    fn esc_cancels_picker() {
        let mut app = sample();
        handle_key(&mut app, key('P'));
        assert!(!matches!(app.overlay, Overlay::None));
        handle_key(&mut app, KeyEvent::new(KeyCode::Esc, KeyModifiers::NONE));
        assert!(matches!(app.overlay, Overlay::None));
        assert_eq!(app.notification.as_deref(), Some("cancelled"));
    }

    // -- editor overlay rendering (herd-less; open + render only) ----------

    fn editable() -> App {
        let mut t = task("e0", "Editable", Status::Hairy, 3, None);
        t.labels = vec!["rust".into(), "tui".into()];
        t.body = "First line.\nSecond line.".into();
        App::new(vec![t])
    }

    #[test]
    fn label_editor_field() {
        // L opens a single-line field on the status line, seeded with labels.
        let mut app = editable();
        handle_key(&mut app, key('L'));
        assert!(matches!(app.overlay, Overlay::Edit(_)));
        insta::assert_snapshot!(draw(&app, 72, 14));
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
    fn body_editor_panel() {
        // E takes over the detail pane with a header + multi-line body.
        let mut app = editable();
        handle_key(&mut app, key('E'));
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
    fn inline_search_esc_restores_previous() {
        let mut app = sample();
        handle_key(&mut app, key('/'));
        typ(&mut app, "zzz");
        assert_eq!(app.filter.search.as_deref(), Some("zzz"));
        handle_key(&mut app, KeyEvent::new(KeyCode::Esc, KeyModifiers::NONE));
        assert!(matches!(app.overlay, Overlay::None));
        assert!(app.filter.search.is_none());
    }

    fn enter_key(app: &mut App) {
        handle_key(app, KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE));
    }
    fn esc_key(app: &mut App) {
        handle_key(app, KeyEvent::new(KeyCode::Esc, KeyModifiers::NONE));
    }
    fn down_key(app: &mut App) {
        handle_key(app, KeyEvent::new(KeyCode::Down, KeyModifiers::NONE));
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

    fn linked() -> App {
        let mut a0 = task("a0", "Root A", Status::Hairy, 2, None);
        a0.body = "follow a1 then a2".into();
        App::new(vec![
            a0,
            task("a1", "Child A1", Status::Hairy, 3, Some("a0")),
            task("a2", "Child A2", Status::Hairy, 3, Some("a0")),
        ])
    }

    fn tab_key(app: &mut App) {
        handle_key(app, KeyEvent::new(KeyCode::Tab, KeyModifiers::NONE));
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
        enter_key(&mut app); // follow link 0 -> a1
        assert_eq!(app.focus, Focus::List);
        assert_eq!(app.selected_id().as_deref(), Some("a1"));
        assert_eq!(app.notification.as_deref(), Some("→ a1"));
    }

    #[test]
    fn detail_tab_cycles_links() {
        let mut app = linked();
        enter_key(&mut app);
        assert_eq!(app.detail_link, 0);
        tab_key(&mut app);
        assert_eq!(app.detail_link, 1);
        handle_key(
            &mut app,
            KeyEvent::new(KeyCode::BackTab, KeyModifiers::NONE),
        );
        assert_eq!(app.detail_link, 0);
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
        assert_eq!(app.views.len(), 5); // 3 status + Recent + Starred
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
        // Starred view (index 4) lists the starred task.
        app.set_view(4);
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
        assert_eq!(app.views.len(), 6);
        assert_eq!(app.view, 5);
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
        assert_eq!(app.views.len(), 6);
        // Delete it via the picker (active view is the custom one, index 5).
        handle_key(&mut app, key('v'));
        handle_key(&mut app, key('d'));
        assert_eq!(app.views.len(), 5);
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
    fn detail_find_esc_restores() {
        let mut app = linked();
        enter_key(&mut app);
        handle_key(&mut app, key('/'));
        typ(&mut app, "zzz");
        assert_eq!(app.detail_find.as_deref(), Some("zzz"));
        esc_key(&mut app);
        assert!(matches!(app.overlay, Overlay::None));
        assert!(app.detail_find.is_none());
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

    // -- live mutation through a temp herd --------------------------------

    mod live {
        use super::*;
        use crate::herd::Herd;
        use crate::store::{self, SCHEMA};
        use std::fs;
        use std::path::PathBuf;
        use std::sync::atomic::{AtomicU64, Ordering};

        static SEQ: AtomicU64 = AtomicU64::new(0);

        /// A temp project dir containing a `.yaks/` herd seeded with `tasks`.
        fn temp_herd(tasks: &[Task]) -> (PathBuf, Herd) {
            let mut proj = std::env::temp_dir();
            let n = SEQ.fetch_add(1, Ordering::Relaxed);
            proj.push(format!("yaksrs-tui-{}-{}", std::process::id(), n));
            let root = proj.join(".yaks");
            for st in [Status::Hairy, Status::Shaving, Status::Shorn, Status::Dead] {
                fs::create_dir_all(root.join(st.dir())).unwrap();
            }
            fs::write(root.join("schema"), SCHEMA.to_string()).unwrap();
            for t in tasks {
                store::write::save(&root, t).unwrap();
            }
            let herd = match Herd::open(&proj) {
                Ok(h) => h,
                Err(_) => panic!("failed to open temp herd"),
            };
            (proj, herd)
        }

        fn press(app: &mut App, chars: &str) {
            for c in chars.chars() {
                handle_key(app, key(c));
            }
        }

        fn enter(app: &mut App) {
            handle_key(app, KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE));
        }

        fn tab(app: &mut App) {
            handle_key(app, KeyEvent::new(KeyCode::Tab, KeyModifiers::NONE));
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

        fn esc(app: &mut App) {
            handle_key(app, KeyEvent::new(KeyCode::Esc, KeyModifiers::NONE));
        }

        #[test]
        fn state_pick_transitions_and_reloads() {
            let (_dir, herd) = temp_herd(&[task("t0", "solo", Status::Hairy, 3, None)]);
            let mut app = App::with_herd(herd).unwrap();
            // S then n → shorn. The task leaves the Hairy tab.
            press(&mut app, "Sn");
            assert!(matches!(app.overlay, Overlay::None));
            let t = app.task("t0").unwrap();
            assert_eq!(t.status, Status::Shorn);
            assert_eq!(app.notification.as_deref(), Some("t0 → shorn"));
        }

        #[test]
        fn priority_pick_updates_and_reloads() {
            let (_dir, herd) = temp_herd(&[task("t0", "solo", Status::Hairy, 3, None)]);
            let mut app = App::with_herd(herd).unwrap();
            press(&mut app, "P1");
            assert_eq!(app.task("t0").unwrap().priority, 1);
            assert_eq!(app.notification.as_deref(), Some("t0 → p1"));
        }

        #[test]
        fn type_pick_updates_and_reloads() {
            let (_dir, herd) = temp_herd(&[task("t0", "solo", Status::Hairy, 3, None)]);
            let mut app = App::with_herd(herd).unwrap();
            press(&mut app, "Tb");
            assert_eq!(app.task("t0").unwrap().kind, "bug");
            assert_eq!(app.notification.as_deref(), Some("t0 → bug"));
        }

        #[test]
        fn slaughter_confirm_moves_to_dead() {
            let (_dir, herd) = temp_herd(&[task("t0", "solo", Status::Hairy, 3, None)]);
            let mut app = App::with_herd(herd).unwrap();
            press(&mut app, "Xy");
            // Dead is excluded from the default view, so it drops out of `all`.
            assert!(app.task("t0").is_none());
            assert_eq!(app.notification.as_deref(), Some("slaughtered t0"));
        }

        #[test]
        fn slaughter_declined_keeps_task() {
            let (_dir, herd) = temp_herd(&[task("t0", "solo", Status::Hairy, 3, None)]);
            let mut app = App::with_herd(herd).unwrap();
            press(&mut app, "Xn");
            assert_eq!(app.task("t0").unwrap().status, Status::Hairy);
            assert_eq!(app.notification.as_deref(), Some("cancelled"));
        }

        #[test]
        fn state_pick_same_status_is_noop() {
            let (_dir, herd) = temp_herd(&[task("t0", "solo", Status::Hairy, 3, None)]);
            let mut app = App::with_herd(herd).unwrap();
            press(&mut app, "Sh");
            assert_eq!(app.task("t0").unwrap().status, Status::Hairy);
            assert_eq!(app.notification.as_deref(), Some("t0 already hairy"));
        }

        #[test]
        fn create_root_via_form() {
            let (_dir, herd) = temp_herd(&[]);
            let mut app = App::with_herd(herd).unwrap();
            press(&mut app, "c"); // open the create form (title row)
            press(&mut app, "foo"); // type the title
            tab(&mut app); // -> type row
            arrow_right(&mut app); // task -> bug (single-select cursor)
            enter(&mut app); // create
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
            let (_dir, herd) = temp_herd(&[]);
            let mut app = App::with_herd(herd).unwrap();
            press(&mut app, "c"); // title row
            press(&mut app, "tuned");
            tab(&mut app); // -> type
            tab(&mut app); // -> priority (default p3 == idx 2)
            arrow_left(&mut app); // p3 -> p2
            tab(&mut app); // -> labels
            press(&mut app, "rust, tui");
            enter(&mut app); // create
            let created = app
                .all
                .iter()
                .find(|t| t.title == "tuned")
                .expect("created");
            assert_eq!(created.priority, 2);
            assert_eq!(created.labels, vec!["rust".to_string(), "tui".to_string()]);
        }

        #[test]
        fn create_child_sets_parent() {
            let (_dir, herd) = temp_herd(&[task("p0", "parent", Status::Hairy, 3, None)]);
            let mut app = App::with_herd(herd).unwrap();
            press(&mut app, "C"); // create child of the selected task
            press(&mut app, "kid");
            enter(&mut app); // create (defaults: task, p3)
            let created = app.all.iter().find(|t| t.title == "kid").expect("child");
            assert_eq!(created.parent.as_deref(), Some("p0"));
            assert_eq!(created.kind, "task");
        }

        #[test]
        fn create_empty_title_enter_is_noop() {
            // Enter with an empty title keeps the form open (Python's `(need title)`).
            let (_dir, herd) = temp_herd(&[]);
            let mut app = App::with_herd(herd).unwrap();
            press(&mut app, "c");
            enter(&mut app);
            assert!(matches!(app.overlay, Overlay::Create(_)));
            assert!(app.all.is_empty());
        }

        #[test]
        fn create_cancelled_with_esc() {
            let (_dir, herd) = temp_herd(&[]);
            let mut app = App::with_herd(herd).unwrap();
            press(&mut app, "c");
            press(&mut app, "foo");
            esc(&mut app); // Esc cancels the form
            assert!(matches!(app.overlay, Overlay::None));
            assert!(app.all.is_empty());
            assert_eq!(app.notification.as_deref(), Some("create cancelled"));
        }

        #[test]
        fn labels_edit_commits() {
            let (_dir, herd) = temp_herd(&[task("t0", "solo", Status::Hairy, 3, None)]);
            let mut app = App::with_herd(herd).unwrap();
            press(&mut app, "L"); // empty labels field
            press(&mut app, "x, y");
            enter(&mut app);
            assert_eq!(
                app.task("t0").unwrap().labels,
                vec!["x".to_string(), "y".to_string()]
            );
        }

        #[test]
        fn body_edit_commits_with_ctrl_s() {
            let (_dir, herd) = temp_herd(&[task("t0", "solo", Status::Hairy, 3, None)]);
            let mut app = App::with_herd(herd).unwrap();
            press(&mut app, "E"); // multi-line body editor (empty body)
            press(&mut app, "hello");
            ctrl_s(&mut app);
            assert!(matches!(app.overlay, Overlay::None));
            assert_eq!(app.task("t0").unwrap().body, "hello");
        }

        fn with_deps(id: &str, status: Status, deps: &[&str]) -> Task {
            let mut t = task(id, id, status, 3, None);
            t.depends_on = deps.iter().map(|s| s.to_string()).collect();
            t
        }

        #[test]
        fn dep_add_via_picker() {
            let (_dir, herd) = temp_herd(&[
                task("t0", "t0", Status::Hairy, 3, None),
                task("t1", "t1", Status::Hairy, 3, None),
            ]);
            let mut app = App::with_herd(herd).unwrap();
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
            let (_dir, herd) = temp_herd(&[
                task("t0", "t0", Status::Hairy, 3, None),
                with_deps("t1", Status::Hairy, &["t0"]),
            ]);
            let mut app = App::with_herd(herd).unwrap();
            assert_eq!(app.selected_id().as_deref(), Some("t0"));
            press(&mut app, "D");
            enter(&mut app); // no candidates -> nothing selected
            assert!(app.task("t0").unwrap().depends_on.is_empty());
            assert_eq!(app.notification.as_deref(), Some("nothing selected"));
        }

        #[test]
        fn reparent_via_picker() {
            let (_dir, herd) = temp_herd(&[
                task("p0", "p0", Status::Hairy, 3, None),
                task("t0", "t0", Status::Hairy, 3, None),
            ]);
            let mut app = App::with_herd(herd).unwrap();
            press(&mut app, "j"); // move to t0
            assert_eq!(app.selected_id().as_deref(), Some("t0"));
            press(&mut app, "R");
            enter(&mut app); // pick first candidate (p0)
            assert_eq!(app.task("t0").unwrap().parent.as_deref(), Some("p0"));
            assert_eq!(app.notification.as_deref(), Some("t0 reparented under p0"));
        }

        #[test]
        fn reparent_clear_to_root() {
            let (_dir, herd) = temp_herd(&[
                task("p0", "p0", Status::Hairy, 3, None),
                task("c0", "c0", Status::Hairy, 3, Some("p0")),
            ]);
            let mut app = App::with_herd(herd).unwrap();
            press(&mut app, "j"); // move to c0 (child of p0)
            assert_eq!(app.selected_id().as_deref(), Some("c0"));
            press(&mut app, "R"); // p0 excluded (current parent); only clear-parent row
            enter(&mut app);
            assert!(app.task("c0").unwrap().parent.is_none());
            assert_eq!(app.notification.as_deref(), Some("c0 moved to top level"));
        }
    }
}
