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
use std::path::PathBuf;
use std::sync::mpsc::{self, Receiver};
use std::time::Duration;

use anyhow::Result;
use edtui::{EditorEventHandler, EditorMode, EditorState, EditorTheme, EditorView, Lines};
use notify::event::{EventKind, ModifyKind};
use notify::{RecommendedWatcher, RecursiveMode, Watcher};
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
    /// The keyboard-help reference (`?`): carries the scroll offset.
    Help(u16),
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
    Comment(String),
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

/// A multi-line edtui field (content zone for descriptions, and later comments).
fn multiline_field(seed: &str, vim: bool) -> RefCell<EditorState> {
    let mut st = EditorState::new(Lines::from(seed));
    st.set_single_line(false);
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

// Task-form layout: title, type, priority, labels, description (a multi-line
// content zone). The last row index is the description.
const CREATE_ROWS: usize = 5;
const DESC_ROW: usize = 4;

/// The create/edit task form: a right-pane form modeled on `Drawer`. Two chip
/// rows (type/priority) are **single-select** — the cursor *is* the value —
/// plus single-line title/labels rows and a multi-line **description** content
/// zone. `Ctrl-S` commits (create or update), `Esc`/`Ctrl-C` cancels. Shared by
/// `c`/`C` (create) and `E` (edit); the reusable multi-line zone will also back
/// comment editing later.
struct CreateForm {
    title: RefCell<EditorState>,
    labels: RefCell<EditorState>,
    description: RefCell<EditorState>,
    /// Index into `TYPE_CHOICES` (single-select cursor==value).
    kind_idx: usize,
    /// Index into `PRI_CHOICES` (single-select cursor==value; default → p3).
    pri_idx: usize,
    row: usize,
    /// Create: the (optional) parent for the new task. Edit: unused (reparent
    /// is a separate action); shown in the header for context.
    parent: Option<String>,
    /// `Some(id)` when editing an existing task; `None` when creating.
    edit_id: Option<String>,
    handler: EditorEventHandler,
}

fn kind_index(kind: &str) -> usize {
    TYPE_CHOICES.iter().position(|&k| k == kind).unwrap_or(0)
}

fn pri_index(p: u8) -> usize {
    PRI_CHOICES.iter().position(|&x| x == p).unwrap_or(2)
}

impl CreateForm {
    fn new(vim: bool, parent: Option<String>) -> Self {
        CreateForm {
            title: text_field("", vim),
            labels: text_field("", vim),
            description: multiline_field("", vim),
            kind_idx: 0,           // task
            pri_idx: pri_index(3), // p3
            row: 0,
            parent,
            edit_id: None,
            handler: make_handler(vim),
        }
    }

    /// Seed the form from an existing task for editing.
    fn for_edit(vim: bool, task: &Task) -> Self {
        CreateForm {
            title: text_field(&task.title, vim),
            labels: text_field(&task.labels.join(", "), vim),
            description: multiline_field(&task.body, vim),
            kind_idx: kind_index(&task.kind),
            pri_idx: pri_index(task.priority),
            row: 0,
            parent: task.parent.clone(),
            edit_id: Some(task.id.clone()),
            handler: make_handler(vim),
        }
    }

    fn is_editing(&self) -> bool {
        self.edit_id.is_some()
    }

    fn is_description_row(&self) -> bool {
        self.row == DESC_ROW
    }

    /// Single-line text rows (title, labels); the description is multi-line and
    /// handled separately.
    fn is_line_text_row(&self) -> bool {
        matches!(self.row, 0 | 3)
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

    /// The raw description text (preserving newlines/formatting).
    fn description_text(&self) -> String {
        self.description.borrow().lines.to_string()
    }

    /// Description for a *create* (empty → no body).
    fn description_opt(&self) -> Option<String> {
        let d = self.description_text();
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
    /// Browser-style navigation history of visited task ids (o = back, i =
    /// forward), driven by following detail links.
    nav_back: Vec<String>,
    nav_fwd: Vec<String>,
    collapsed: HashSet<String>,
    /// The live view filter applied by the tree (re-colors + prunes).
    filter: FilterSpec,
    /// Approx. list viewport height, refreshed each loop for paging math.
    page: u16,
    /// Approx. detail viewport height (mid area = terminal height - 3),
    /// refreshed each loop; used to keep the active link scrolled into view.
    detail_page: u16,
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
            nav_back: Vec::new(),
            nav_fwd: Vec::new(),
            collapsed: HashSet::new(),
            filter,
            page: 10,
            detail_page: 10,
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

    /// Re-read the herd from disk (external change) while keeping the cursor on
    /// the same task by id. Silent: it must not clobber a mutation's own
    /// notification, and the event loop only calls it while idle (no overlay).
    fn reload_preserving_selection(&mut self) {
        let sel = self.selected_id();
        if let Some(h) = &self.herd {
            if let Ok(all) = h.list(FilterSpec::default(), false) {
                self.all = all;
            }
        }
        match sel.and_then(|id| self.rows().iter().position(|r| r.task.id == id)) {
            Some(pos) => self.cursor = pos,
            None => self.clamp_cursor(),
        }
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

    /// Open the shared form seeded from the selected task, for editing (E).
    fn open_edit(&mut self) {
        let Some(id) = self.selected_id() else { return };
        let Some(task) = self.task(&id) else { return };
        self.overlay = Overlay::Create(CreateForm::for_edit(self.editor_vim, task));
    }

    fn handle_create_key(&mut self, k: KeyEvent) {
        let ctrl = k.modifiers.contains(KeyModifiers::CONTROL);
        let (is_desc, is_line_text) = match &self.overlay {
            Overlay::Create(f) => (f.is_description_row(), f.is_line_text_row()),
            _ => return,
        };
        // Commit (Ctrl-S) / cancel (Ctrl-C, or Esc outside the description zone —
        // inside it Esc belongs to the editor, e.g. vim normal mode).
        if ctrl && k.code == KeyCode::Char('s') {
            let has_title =
                matches!(&self.overlay, Overlay::Create(f) if !f.title_text().is_empty());
            if has_title {
                self.commit_form();
            }
            return;
        }
        if (ctrl && k.code == KeyCode::Char('c')) || (k.code == KeyCode::Esc && !is_desc) {
            let editing = matches!(&self.overlay, Overlay::Create(f) if f.is_editing());
            self.overlay = Overlay::None;
            self.notification = Some(if editing {
                "edit cancelled".into()
            } else {
                "create cancelled".into()
            });
            return;
        }
        // Row navigation: Tab / Shift-Tab / Ctrl-N / Ctrl-P always move rows.
        // On chip rows j/k also navigate; on single-line text rows Up/Down do;
        // the description zone keeps Up/Down for its own cursor.
        let is_chip = !is_desc && !is_line_text;
        let nav_down = matches!(k.code, KeyCode::Tab)
            || (ctrl && k.code == KeyCode::Char('n'))
            || (is_line_text && k.code == KeyCode::Down)
            || (is_chip && matches!(k.code, KeyCode::Down | KeyCode::Char('j')));
        let nav_up = matches!(k.code, KeyCode::BackTab)
            || (ctrl && k.code == KeyCode::Char('p'))
            || (is_line_text && k.code == KeyCode::Up)
            || (is_chip && matches!(k.code, KeyCode::Up | KeyCode::Char('k')));
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
        // Enter on a single-line/chip row advances to the next row; in the
        // description zone it inserts a newline (handled by the editor below).
        if k.code == KeyCode::Enter && !is_desc {
            if let Overlay::Create(f) = &mut self.overlay {
                f.row = (f.row + 1) % CREATE_ROWS;
            }
            return;
        }
        if is_desc {
            if let Overlay::Create(f) = &mut self.overlay {
                f.handler.on_key_event(k, &mut f.description.borrow_mut());
            }
            return;
        }
        if is_line_text {
            if let Overlay::Create(f) = &mut self.overlay {
                match f.row {
                    0 => f.handler.on_key_event(k, &mut f.title.borrow_mut()),
                    3 => f.handler.on_key_event(k, &mut f.labels.borrow_mut()),
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

    fn commit_form(&mut self) {
        let editing = matches!(&self.overlay, Overlay::Create(f) if f.is_editing());
        if editing {
            self.commit_edit_form();
        } else {
            self.commit_create();
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

    /// Commit an edit: diff the form against the current task so unchanged
    /// fields don't rewrite the file or bump `updated`.
    fn commit_edit_form(&mut self) {
        let Overlay::Create(f) = std::mem::replace(&mut self.overlay, Overlay::None) else {
            return;
        };
        let Some(id) = f.edit_id.clone() else { return };
        let Some(cur) = self.task(&id).cloned() else {
            self.notification = Some(format!("{id} not found"));
            return;
        };
        let title = f.title_text();
        let kind = TYPE_CHOICES[f.kind_idx].to_string();
        let priority = PRI_CHOICES[f.pri_idx];
        let desc = f.description_text();
        let new_labels = f.labels_vec();
        let mut edit = TaskEdit::default();
        if title != cur.title {
            edit.title = Some(title);
        }
        if kind != cur.kind {
            edit.kind = Some(kind);
        }
        if priority != cur.priority {
            edit.priority = Some(priority);
        }
        if desc != cur.body {
            edit.description = Some(desc);
        }
        edit.add_labels = new_labels
            .iter()
            .filter(|l| !cur.labels.contains(l))
            .cloned()
            .collect();
        edit.remove_labels = cur
            .labels
            .iter()
            .filter(|l| !new_labels.contains(l))
            .cloned()
            .collect();
        self.apply_edit(&id, edit, format!("{id} updated"));
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

    fn open_help(&mut self) {
        self.overlay = Overlay::Help(0);
    }

    /// M — append a timestamped comment/note to the selected task (multi-line).
    fn open_comment(&mut self) {
        if let Some(id) = self.selected_id() {
            self.overlay = Overlay::Edit(Editor::new(
                self.editor_vim,
                false,
                format!("Comment on {id} — Ctrl-S save · Ctrl-C cancel"),
                "",
                EditAction::Comment(id),
            ));
        }
    }

    fn handle_help_key(&mut self, k: KeyEvent) {
        // Any of Esc/q/? dismisses the reference.
        if matches!(
            k.code,
            KeyCode::Esc | KeyCode::Char('q') | KeyCode::Char('?')
        ) {
            self.overlay = Overlay::None;
            return;
        }
        let vh = self.detail_page.max(1);
        let max_scroll = (help_content().len() as u16).saturating_sub(vh);
        let half = (vh / 2).max(1);
        if let Overlay::Help(scroll) = &mut self.overlay {
            match k.code {
                KeyCode::Char('j') | KeyCode::Down => *scroll = (*scroll + 1).min(max_scroll),
                KeyCode::Char('k') | KeyCode::Up => *scroll = scroll.saturating_sub(1),
                KeyCode::Char('d') | KeyCode::PageDown => {
                    *scroll = (*scroll + half).min(max_scroll)
                }
                KeyCode::Char('u') | KeyCode::PageUp => *scroll = scroll.saturating_sub(half),
                KeyCode::Char('g') => *scroll = 0,
                KeyCode::Char('G') => *scroll = max_scroll,
                _ => {}
            }
        }
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
        // Expand any collapsed ancestors so the target row is actually visible;
        // otherwise the cursor can't land on it and the jump silently no-ops.
        self.expand_ancestors(id);
        if let Some(pos) = self.rows().iter().position(|r| r.task.id == id) {
            self.cursor = pos;
        }
        self.focus = Focus::List;
    }

    /// Remove every ancestor of `id` from the collapsed set (walking the parent
    /// chain), persisting the change so a jumped-to task is never hidden.
    fn expand_ancestors(&mut self, id: &str) {
        let mut changed = false;
        let mut cur = self.task(id).and_then(|t| t.parent.clone());
        while let Some(pid) = cur {
            if self.collapsed.remove(&pid) {
                changed = true;
            }
            cur = self.task(&pid).and_then(|t| t.parent.clone());
        }
        if changed {
            self.save_collapsed();
        }
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
        // Bring the target line into view without disturbing the scroll when it
        // is already visible (so cycling links doesn't jump the viewport).
        let line = jumps[self.detail_link].line as u16;
        self.scroll_line_into_view(line);
    }

    /// Move `detail_scroll` the minimum needed so `line` sits inside the detail
    /// viewport; leave it untouched when the line is already visible.
    fn scroll_line_into_view(&mut self, line: u16) {
        let vh = self.detail_page.max(1);
        if line < self.detail_scroll {
            self.detail_scroll = line;
        } else if line >= self.detail_scroll.saturating_add(vh) {
            self.detail_scroll = line.saturating_sub(vh - 1);
        }
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

    /// Select `id` and show it in the detail pane, resetting the per-task detail
    /// view state (scroll/link/find/match). Shared by link-follow and i/o nav.
    fn open_task_in_detail(&mut self, id: &str) {
        self.select_task(id); // drops focus to the list; we re-raise it below
        self.focus = Focus::Detail;
        self.detail_scroll = 0;
        self.detail_link = 0;
        self.detail_find = None;
        self.detail_match = 0;
    }

    fn follow_link(&mut self) {
        let jumps = self.detail_jumps();
        let Some(j) = jumps.into_iter().nth(self.detail_link) else {
            return;
        };
        match j.target {
            detail::Target::Task(id) => {
                // Record the jump for o/i history (browser-style: a new follow
                // clears the forward stack).
                if let Some(cur) = self.selected_id() {
                    if cur != id {
                        self.nav_back.push(cur);
                        self.nav_fwd.clear();
                    }
                }
                self.open_task_in_detail(&id);
                self.notification = Some(format!("→ {id}"));
            }
            detail::Target::Url(u) => {
                self.notification = Some(format!("link: {u}"));
            }
        }
    }

    /// y — copy the selected task's id to the system clipboard (best-effort).
    fn copy_selected_id(&mut self) {
        let Some(id) = self.selected_id() else { return };
        self.notification = Some(if crate::clipboard::copy_text(&id) {
            format!("copied {id}")
        } else {
            "clipboard unavailable".into()
        });
    }

    /// J/K — move to the next/prev task in the list while staying in the detail
    /// pane, resetting the per-task detail view state.
    fn detail_next_task(&mut self, delta: i32) {
        let len = self.rows().len();
        if len == 0 {
            return;
        }
        let next = (self.cursor as i32 + delta).clamp(0, len as i32 - 1) as usize;
        if next != self.cursor {
            self.cursor = next;
            self.detail_scroll = 0;
            self.detail_link = 0;
            self.detail_find = None;
            self.detail_match = 0;
        }
    }

    /// G — scroll the detail pane so its last line rests at the viewport bottom.
    fn detail_scroll_bottom(&mut self) {
        if let Some(t) = self.selected() {
            let n = detail::build(t, &self.all).len() as u16;
            self.detail_scroll = n.saturating_sub(self.detail_page.max(1));
        }
    }

    /// o — jump back to the previously visited task (browser back).
    fn nav_back(&mut self) {
        let Some(prev) = self.nav_back.pop() else {
            self.notification = Some("no earlier yak".into());
            return;
        };
        if let Some(cur) = self.selected_id() {
            self.nav_fwd.push(cur);
        }
        self.open_task_in_detail(&prev);
        self.notification = Some(format!("← {prev}"));
    }

    /// i — jump forward again after going back (browser forward).
    fn nav_forward(&mut self) {
        let Some(next) = self.nav_fwd.pop() else {
            self.notification = Some("no later yak".into());
            return;
        };
        if let Some(cur) = self.selected_id() {
            self.nav_back.push(cur);
        }
        self.open_task_in_detail(&next);
        self.notification = Some(format!("→ {next}"));
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
        if matches!(self.overlay, Overlay::Help(_)) {
            self.handle_help_key(k);
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
            | Overlay::ViewPicker(_)
            | Overlay::Help(_) => {}
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
            EditAction::Comment(id) => {
                if text.trim().is_empty() {
                    self.notification = Some("comment cancelled".into());
                    return;
                }
                self.apply_edit(
                    &id,
                    TaskEdit {
                        note: Some(text),
                        ..Default::default()
                    },
                    format!("comment added to {id}"),
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
    // Best-effort filesystem watch on the herd's `.yaks/` tree so external edits
    // (including the user's own, from another process) refresh the UI. Kept
    // alive for the loop's duration; `_watcher` must not be dropped early.
    let watch_path = app.herd.as_ref().map(|h| h.root().to_path_buf());
    let (_watcher, rx) = setup_watcher(watch_path);
    let mut dirty = false;
    loop {
        // Apply external changes only while idle (no overlay), so we never yank
        // data out from under an open editor/picker mid-interaction.
        if dirty && matches!(app.overlay, Overlay::None) {
            app.reload_preserving_selection();
            dirty = false;
        }
        term.draw(|f| render(app, f))?;
        // Record the main-area height (minus tab + help lines) for paging.
        let h = term.size()?.height;
        app.page = h.saturating_sub(2).max(1);
        app.detail_page = h.saturating_sub(3).max(1);
        // Block for input, but wake periodically to service filesystem events.
        if event::poll(Duration::from_millis(250))? {
            if let Event::Key(k) = event::read()? {
                if k.kind == KeyEventKind::Press {
                    handle_key(app, k);
                }
            }
        }
        // Coalesce any pending fs notifications into one deferred refresh.
        if let Some(rx) = &rx {
            while rx.try_recv().is_ok() {
                dirty = true;
            }
        }
        if app.quit {
            break;
        }
    }
    Ok(())
}

/// Set up a recursive watcher on `path`, returning the watcher (which the caller
/// must keep alive) and a receiver that yields once per content-changing event.
/// Best-effort: any failure yields `(None, None)` and the TUI just won't
/// auto-refresh. Access/metadata-only events are filtered out so that our own
/// reads can't trigger a reload feedback loop.
fn setup_watcher(path: Option<PathBuf>) -> (Option<RecommendedWatcher>, Option<Receiver<()>>) {
    let Some(path) = path else {
        return (None, None);
    };
    let (tx, rx) = mpsc::channel();
    let handler = move |res: notify::Result<notify::Event>| {
        if let Ok(ev) = res {
            let interesting = match ev.kind {
                EventKind::Create(_) | EventKind::Remove(_) => true,
                EventKind::Modify(ModifyKind::Metadata(_)) => false,
                EventKind::Modify(_) => true,
                _ => false, // Access / Any / Other
            };
            if interesting {
                let _ = tx.send(());
            }
        }
    };
    let mut watcher = match notify::recommended_watcher(handler) {
        Ok(w) => w,
        Err(_) => return (None, None),
    };
    if watcher.watch(&path, RecursiveMode::Recursive).is_err() {
        return (None, None);
    }
    (Some(watcher), Some(rx))
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
            KeyCode::Char('E') => app.open_edit(),
            KeyCode::Char('D') => app.open_dep_picker(),
            KeyCode::Char('R') => app.open_reparent_picker(),
            KeyCode::Char('/') => app.open_search(),
            KeyCode::Char('f') => app.open_drawer(),
            KeyCode::Char('*') => app.toggle_star(),
            KeyCode::Char('y') => app.copy_selected_id(),
            KeyCode::Char('M') => app.open_comment(),
            KeyCode::Char('v') => app.open_view_picker(),
            KeyCode::Char('V') => app.open_save_view(),
            KeyCode::Char('?') => app.open_help(),
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
            KeyCode::Char('o') => app.nav_back(),
            KeyCode::Char('i') => app.nav_forward(),
            KeyCode::Char('/') => app.open_detail_find(),
            KeyCode::Char('n') => app.detail_find_jump(1),
            KeyCode::Char('N') => app.detail_find_jump(-1),
            KeyCode::Char('?') => app.open_help(),
            KeyCode::Enter => app.follow_link(),
            // Mutating ops mirrored from the list pane (all act on selected()).
            KeyCode::Char('S') => app.open_state_picker(),
            KeyCode::Char('P') => app.open_priority_picker(),
            KeyCode::Char('T') => app.open_type_picker(),
            KeyCode::Char('L') => app.open_labels(),
            KeyCode::Char('X') => app.open_slaughter_confirm(),
            KeyCode::Char('E') => app.open_edit(),
            KeyCode::Char('D') => app.open_dep_picker(),
            KeyCode::Char('R') => app.open_reparent_picker(),
            KeyCode::Char('c') => app.open_create(false),
            KeyCode::Char('C') => app.open_create(true),
            KeyCode::Char('f') => app.open_drawer(),
            KeyCode::Char('*') => app.toggle_star(),
            // Move between tasks without leaving the detail pane.
            KeyCode::Char('J') => app.detail_next_task(1),
            KeyCode::Char('K') => app.detail_next_task(-1),
            KeyCode::Char('y') => app.copy_selected_id(),
            KeyCode::Char('M') => app.open_comment(),
            KeyCode::Char('j') | KeyCode::Down => {
                app.detail_scroll = app.detail_scroll.saturating_add(1)
            }
            KeyCode::Char('k') | KeyCode::Up => {
                app.detail_scroll = app.detail_scroll.saturating_sub(1)
            }
            KeyCode::Char('d') => app.detail_scroll = app.detail_scroll.saturating_add(half as u16),
            KeyCode::Char('u') => app.detail_scroll = app.detail_scroll.saturating_sub(half as u16),
            KeyCode::Char('g') => app.detail_scroll = 0,
            KeyCode::Char('G') => app.detail_scroll_bottom(),
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
            Overlay::Fuzzy(_)
                | Overlay::Drawer(_)
                | Overlay::Create(_)
                | Overlay::ViewPicker(_)
                | Overlay::Help(_)
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
            Overlay::Help(scroll) => render_help(*scroll, frame, inner),
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
            // 📌 pinned, 🔒 builtin — emoji glyphs matching the Python view manager.
            let active = if i == app.view { "▸" } else { " " };
            let pin = if v.pinned { "\u{1f4cc}" } else { "  " };
            let lock = if v.builtin { "  \u{1f512}" } else { "" };
            ListItem::new(Line::from(vec![
                Span::styled(format!("{active} "), Style::new().fg(Color::Cyan)),
                Span::raw(format!("{pin} ")),
                Span::raw(v.name.clone()),
                Span::styled(
                    format!("  ({})", app.view_count(v)),
                    Style::new().fg(Color::DarkGray),
                ),
                Span::raw(lock.to_string()),
            ]))
        })
        .collect();
    let mut state = ListState::default();
    if !app.views.is_empty() {
        state.select(Some(sel.min(app.views.len() - 1)));
    }
    frame.render_stateful_widget(
        List::new(items).highlight_style(Style::new().bg(Color::Indexed(237))),
        body,
        &mut state,
    );
}

/// The keyboard reference shown by `?`, as styled lines: cyan-bold section
/// headers, a yellow key column, then the description. Reflects the actual Rust
/// bindings (not Python's). Rebuilt on demand; also used to clamp help scroll.
fn help_content() -> Vec<Line<'static>> {
    fn section(name: &'static str) -> Line<'static> {
        Line::from(Span::styled(
            name,
            Style::new().fg(Color::Cyan).add_modifier(Modifier::BOLD),
        ))
    }
    fn entry(key: &'static str, desc: &'static str) -> Line<'static> {
        Line::from(vec![
            Span::styled(format!("  {key:<14}"), Style::new().fg(Color::Yellow)),
            Span::raw(desc),
        ])
    }
    let blank = || Line::from(String::new());
    vec![
        section("Movement"),
        entry("j / k  ↓ / ↑", "Move cursor"),
        entry("d / u", "Half-page down / up"),
        entry("PgDn / PgUp", "Full-page down / up"),
        entry("g / G", "Top / bottom"),
        blank(),
        section("List pane"),
        entry("Tab / S-Tab", "Next / previous view"),
        entry("[ / ]", "Previous / next view"),
        entry("l / → / Enter", "Show detail pane"),
        entry("Space", "Collapse / expand subtree"),
        entry("v / V", "View picker / save filter as view"),
        entry("* ", "Star / unstar (Starred view)"),
        blank(),
        section("Detail pane"),
        entry("h / ← / Esc", "Back to list"),
        entry("j / k", "Scroll"),
        entry("Tab / [ / ]", "Cycle links"),
        entry("Enter", "Follow link"),
        entry("i / o", "Nav forward / back"),
        entry("J / K", "Next / prev task (stay in detail)"),
        entry("/ , n / N", "Find, next / prev match"),
        blank(),
        section("Edit"),
        entry("c / C", "New root / child yak"),
        entry("E", "Edit description"),
        entry("P / T / L / S", "Priority / type / labels / state"),
        entry("D / R", "Add dependency / reparent"),
        entry("M", "Add a comment (note)"),
        entry("X", "Slaughter (delete, confirm)"),
        blank(),
        section("Search & filter"),
        entry("/", "Inline search"),
        entry("f", "Filter drawer"),
        entry("Esc", "Revert filter to the active view"),
        blank(),
        section("General"),
        entry("y", "Copy yak id to clipboard"),
        entry("?", "Toggle this help"),
        entry("q / Ctrl-C", "Quit"),
    ]
}

fn render_help(scroll: u16, frame: &mut Frame, area: Rect) {
    let [head, body] = Layout::vertical([Constraint::Length(1), Constraint::Min(0)]).areas(area);
    frame.render_widget(
        Paragraph::new(Span::styled(
            "Help — keys",
            Style::new().fg(Color::Cyan).add_modifier(Modifier::BOLD),
        )),
        head,
    );
    frame.render_widget(Paragraph::new(help_content()).scroll((scroll, 0)), body);
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

/// The create/edit task form: header + title / type / priority / labels meta
/// rows, a `─ description ─` separator, then a multi-line description content
/// zone filling the rest. Laid out like `render_drawer`, inset by `right_divider`.
fn render_create(f: &CreateForm, frame: &mut Frame, area: Rect) {
    let rows = Layout::vertical([
        Constraint::Length(1), // header
        Constraint::Length(1), // title
        Constraint::Length(1), // type chips
        Constraint::Length(1), // priority chips
        Constraint::Length(1), // labels
        Constraint::Length(1), // description separator
        Constraint::Min(0),    // description content zone
    ])
    .split(area);
    let header = match (&f.edit_id, &f.parent) {
        (Some(id), _) => format!("Edit {id}"),
        (None, Some(p)) => format!("New task (child of {p})"),
        (None, None) => "New yak".to_string(),
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
    // Description separator: `▸ description ───` (marker cyan when focused).
    let desc_focused = f.row == DESC_ROW;
    let sep_w = rows[5].width as usize;
    let head = if desc_focused {
        "▸ description "
    } else {
        "  description "
    };
    let dashes = sep_w.saturating_sub(disp_width(head));
    frame.render_widget(
        Paragraph::new(Line::from(vec![
            Span::styled(
                head.to_string(),
                if desc_focused {
                    Style::new().fg(Color::Cyan)
                } else {
                    Style::new().fg(Color::DarkGray)
                },
            ),
            Span::styled("─".repeat(dashes), Style::new().fg(Color::DarkGray)),
        ])),
        rows[5],
    );
    if desc_focused {
        let mut st = f.description.borrow_mut();
        frame.render_widget(EditorView::new(&mut st).theme(editor_theme()), rows[6]);
    } else {
        let text = f.description.borrow().lines.to_string();
        let shown = if text.trim().is_empty() {
            "(no description)".to_string()
        } else {
            text
        };
        frame.render_widget(
            Paragraph::new(shown).style(Style::new().fg(Color::DarkGray)),
            rows[6],
        );
    }
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
            style = style.bg(Color::Indexed(237)).add_modifier(Modifier::BOLD);
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
        Overlay::Help(_) => "help",
        Overlay::DetailFind(_) => "detail-find",
        Overlay::ViewPicker(_) => "view-picker",
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

/// Per-priority colour, matching Python's `_PRIORITY_PAIRS`:
/// P1 urgent red+bold, P2 high magenta, P3 medium yellow, P4 low green,
/// P5 lowest blue+dim.
fn priority_style(p: u8) -> Style {
    match p {
        1 => Style::new().fg(Color::Red).add_modifier(Modifier::BOLD),
        2 => Style::new().fg(Color::Magenta),
        4 => Style::new().fg(Color::Green),
        5 => Style::new().fg(Color::Blue).add_modifier(Modifier::DIM),
        _ => Style::new().fg(Color::Yellow), // 3 (and any fallback)
    }
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
    // Subtle selection (Python's C_SELECTED): a dark-gray background with the
    // foreground reset to the terminal default, rather than an obtrusive
    // black-on-cyan reverse. Resetting fg keeps the row legible on the dark bg
    // (per-field colours like blue/blue-dim would be low-contrast on 237). The
    // unfocused list uses a slightly darker bg so focus is still clear.
    let hl = if focused {
        Style::new().fg(Color::Reset).bg(Color::Indexed(237))
    } else {
        Style::new().fg(Color::Reset).bg(Color::Indexed(236))
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
    // Right side, as separately-styled spans (labels magenta-dim like Python's
    // C_LABEL, star its own glyph, collapse badge dim).
    let mut right_spans: Vec<Span> = Vec::new();
    let mut right_plain = String::new();
    if !label_str.is_empty() {
        right_plain.push_str(&label_str);
        right_spans.push(Span::styled(
            label_str.clone(),
            dim(Style::new().fg(Color::Magenta).add_modifier(Modifier::DIM)),
        ));
    }
    if starred {
        if !right_plain.is_empty() {
            right_plain.push(' ');
            right_spans.push(Span::raw(" "));
        }
        right_plain.push('\u{2b50}');
        right_spans.push(Span::raw("\u{2b50}"));
    }
    if !badge.is_empty() {
        right_plain.push_str(&badge);
        right_spans.push(Span::styled(
            badge.clone(),
            dim(Style::new().fg(Color::DarkGray)),
        ));
    }
    let rw = disp_width(&right_plain);

    let left_fixed = 1 + disp_width(&body_padded) + disp_width(&pri_s) + disp_width(&type_s);
    let title_avail = width.saturating_sub(left_fixed + rw + 1);
    let title = truncate_disp(
        &format!("{} {}", r.task.status.emoji(), r.task.title),
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
        Span::styled(pri_s, dim(priority_style(r.task.priority))),
        Span::styled(type_s, dim(Style::new().fg(Color::Cyan))),
        Span::styled(title, dim(Style::new())),
    ];
    if pad > 0 {
        spans.push(Span::raw(" ".repeat(pad)));
    }
    spans.extend(right_spans);
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
            // Python C_LINK_SEL: blue on the subtle 237 background, emphasised.
            Style::new()
                .fg(Color::Blue)
                .bg(Color::Indexed(237))
                .add_modifier(Modifier::BOLD | Modifier::UNDERLINED)
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
    // Help reference hint.
    if matches!(&app.overlay, Overlay::Help(_)) {
        frame.render_widget(
            Paragraph::new(Span::styled(
                "j/k scroll · d/u page · g/G top/bottom · ?/q/Esc close",
                Style::new().fg(Color::DarkGray),
            )),
            area,
        );
        return;
    }
    // Create/edit-form help hint. The `(need title)` marker mirrors Python's guard.
    if let Overlay::Create(f) = &app.overlay {
        let commit = if f.title_text().is_empty() {
            "(need title)"
        } else if f.is_editing() {
            "Ctrl-S save"
        } else {
            "Ctrl-S create"
        };
        frame.render_widget(
            Paragraph::new(Span::styled(
                format!("Tab/↑↓ rows · ←→ chips · {commit} · Esc cancel"),
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
        enter_key(&mut app); // follow link 0 -> a1
        assert_eq!(app.focus, Focus::Detail);
        assert_eq!(app.selected_id().as_deref(), Some("a1"));
        assert!(!app.collapsed.contains("a0"), "ancestor expanded");
    }

    #[test]
    fn nav_history_back_and_forward() {
        // o/i retrace the link-follow chain (yaksrs-5d63).
        let mut app = linked();
        enter_key(&mut app); // detail on a0
        enter_key(&mut app); // follow link 0 -> a1
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
            let (_dir, herd) = temp_herd(&[]);
            let mut app = App::with_herd(herd).unwrap();
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
            let (_dir, herd) = temp_herd(&[
                task("t0", "first", Status::Hairy, 3, None),
                task("t1", "second", Status::Hairy, 3, None),
            ]);
            let mut app = App::with_herd(herd).unwrap();
            press(&mut app, "j"); // cursor -> t1
            assert_eq!(app.selected_id().as_deref(), Some("t1"));
            // Simulate an external write to the herd (as the file watcher would
            // observe), then refresh from disk.
            app.herd
                .as_ref()
                .unwrap()
                .create(NewTask {
                    title: "third".into(),
                    kind: Some("task".into()),
                    priority: Some(3),
                    parent: None,
                    labels: vec![],
                    depends_on: vec![],
                    source: None,
                    description: None,
                })
                .unwrap();
            app.reload_preserving_selection();
            assert!(app.all.iter().any(|t| t.title == "third"), "picked up add");
            assert_eq!(app.selected_id().as_deref(), Some("t1"), "cursor kept");
        }

        #[test]
        fn create_child_sets_parent() {
            let (_dir, herd) = temp_herd(&[task("p0", "parent", Status::Hairy, 3, None)]);
            let mut app = App::with_herd(herd).unwrap();
            press(&mut app, "C"); // create child of the selected task
            press(&mut app, "kid");
            ctrl_s(&mut app); // create (defaults: task, p3)
            let created = app.all.iter().find(|t| t.title == "kid").expect("child");
            assert_eq!(created.parent.as_deref(), Some("p0"));
            assert_eq!(created.kind, "task");
        }

        #[test]
        fn create_empty_title_ctrl_s_is_noop() {
            // Ctrl-S with an empty title keeps the form open (`(need title)`).
            let (_dir, herd) = temp_herd(&[]);
            let mut app = App::with_herd(herd).unwrap();
            press(&mut app, "c");
            ctrl_s(&mut app);
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
        fn edit_form_updates_description() {
            let (_dir, herd) = temp_herd(&[task("t0", "solo", Status::Hairy, 3, None)]);
            let mut app = App::with_herd(herd).unwrap();
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
            let (_dir, herd) = temp_herd(&[task("t0", "solo", Status::Hairy, 3, None)]);
            let mut app = App::with_herd(herd).unwrap();
            press(&mut app, "M"); // open the multi-line comment editor
            press(&mut app, "a helpful note");
            ctrl_s(&mut app);
            assert!(matches!(app.overlay, Overlay::None));
            let body = &app.task("t0").unwrap().body;
            assert!(body.contains("a helpful note"), "note text present");
            assert!(body.contains('\u{25b8}'), "timestamp sigil present");
        }

        #[test]
        fn edit_form_changes_type_and_priority() {
            let (_dir, herd) = temp_herd(&[task("t0", "solo", Status::Hairy, 3, None)]);
            let mut app = App::with_herd(herd).unwrap();
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
            let (_dir, herd) = temp_herd(&[task("t0", "solo", Status::Hairy, 3, None)]);
            let mut app = App::with_herd(herd).unwrap();
            press(&mut app, "E");
            press(&mut app, "zzz"); // edits the title field in place
            esc(&mut app); // cancel (title row, not description)
            assert!(matches!(app.overlay, Overlay::None));
            assert_eq!(app.task("t0").unwrap().title, "solo");
            assert_eq!(app.notification.as_deref(), Some("edit cancelled"));
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
