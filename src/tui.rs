//! Interactive terminal UI (`yaks tui`) — a thin layer over the core.
//!
//! All drawing goes through the pure `render(&App, &mut Frame)`, so the same
//! painter can target a real terminal (crossterm) or an in-memory `TestBackend`
//! buffer (snapshot tests + the future demo-cast pipeline). Key handling only
//! mutates `App`; mutating keys route through the `Herd` facade and then reload.

mod cache;
mod content;
mod detail;
mod headless;
mod markdown;
mod tree;
mod view;
mod views_store;

// `headless` holds the `toque::HeadlessApp` impl for `App`; nothing to re-export
// (the headless driver lives in the `toque` crate, invoked from `main`).

use std::cell::{Cell, RefCell};
use std::collections::{HashMap, HashSet};
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
use ratatui::widgets::{Block, Borders, List, ListItem, ListState, Padding, Paragraph, Wrap};
use ratatui::{Frame, Terminal};

use crate::filter::{self, FilterSpec};
use crate::herd::{
    AttachOutcome, CreateOutcome, DepOutcome, Herd, MoveOutcome, NewTask, Reparent, TaskEdit,
    UpdateOutcome,
};
use crate::model::{Status, Task};

#[derive(Clone, Copy, PartialEq, Debug)]
enum Focus {
    List,
    Detail,
}

/// A modal prompt painted on the bottom line: `pick()` (single keypress) and
/// `confirm()` (y/N) dialogs. Kept as plain
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
    /// Discard a dirty edit/create form or comment (stashed in `App.dirty_cancel`).
    DiscardEdit,
}

/// An embedded edtui editor plus what to do with its text on commit. The
/// `EditorState` is `RefCell`-wrapped because `EditorView` needs `&mut` at
/// render time, while our `render(&App, ..)` borrows the app immutably.
struct Editor {
    state: RefCell<EditorState>,
    handler: EditorEventHandler,
    single_line: bool,
    /// Whether this editor uses the vim keybinding profile. When true, `Esc`
    /// leaves Insert for Normal mode instead of closing the overlay, so the
    /// full normal-mode keymap is reachable even in single-line fields.
    vim: bool,
    /// Prompt label (bottom-line prefix for fields; header for the body panel).
    label: String,
    action: EditAction,
}

enum EditAction {
    Labels(String),
    Comment(String),
    Attach(String),
    SaveView,
    RenameView { index: usize },
}

/// What a context-sensitive `E` in the detail pane should edit, derived from
/// the line the cursor sits on.
#[derive(Clone, Copy)]
enum EditTarget {
    Title,
    Type,
    Priority,
    Labels,
    Status,
    /// A content block: `0` = description, `1..` = comments.
    Content(usize),
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

/// A multi-line edtui field (content zone for descriptions/comments). In vim,
/// a field seeded with existing content opens in Normal mode (the common vi
/// expectation when reviewing text); an empty one opens in Insert to type
/// straight away.
fn multiline_field(seed: &str, vim: bool) -> RefCell<EditorState> {
    let mut st = EditorState::new(Lines::from(seed));
    st.set_single_line(false);
    st.mode = if vim && !seed.is_empty() {
        EditorMode::Normal
    } else {
        EditorMode::Insert
    };
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

/// Max gap between two `Esc` presses for them to count as a double-tap cancel.
const DOUBLE_ESC_MS: u64 = 300;

// Task-form fixed rows: title, type, priority, labels. Content blocks (a
// description, plus one per comment when editing) follow as rows `HEADER_ROWS +
// block_index`, so the total row count is dynamic (`CreateForm::row_count`).
const HEADER_ROWS: usize = 4;

/// One editable content block in the form: the task's description, or a single
/// comment (carrying its original timestamp so it round-trips unchanged).
struct ContentBlock {
    kind: content::BlockKind,
    editor: RefCell<EditorState>,
}

/// The create/edit task form: a right-pane form modeled on `Drawer`. Two chip
/// rows (type/priority) are **single-select** — the cursor *is* the value —
/// plus single-line title/labels rows and a stack of multi-line **content**
/// blocks (description + comments). `Ctrl-N/P`/Tab walk every row; the focused
/// block expands to a live editor (accordion). `Ctrl-S` commits (create or
/// update), `Esc`/`Ctrl-C` cancels. Shared by `c`/`C` (create) and `E` (edit).
struct CreateForm {
    title: RefCell<EditorState>,
    labels: RefCell<EditorState>,
    /// `blocks[0]` is always the description; `blocks[1..]` are comments.
    blocks: Vec<ContentBlock>,
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
            blocks: vec![ContentBlock {
                kind: content::BlockKind::Description,
                editor: multiline_field("", vim),
            }],
            kind_idx: 0,           // task
            pri_idx: pri_index(3), // p3
            row: 0,
            parent,
            edit_id: None,
            handler: make_handler(vim),
        }
    }

    /// Seed the form from an existing task for editing: the body is split into a
    /// description block plus one block per comment.
    fn for_edit(vim: bool, task: &Task) -> Self {
        let blocks = content::parse(&task.body)
            .into_iter()
            .map(|b| ContentBlock {
                kind: b.kind,
                editor: multiline_field(&b.text, vim),
            })
            .collect();
        CreateForm {
            title: text_field(&task.title, vim),
            labels: text_field(&task.labels.join(", "), vim),
            blocks,
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

    fn row_count(&self) -> usize {
        HEADER_ROWS + self.blocks.len()
    }

    /// The content-block index for the current row, when the cursor is on one.
    fn content_index(&self) -> Option<usize> {
        self.row
            .checked_sub(HEADER_ROWS)
            .filter(|&i| i < self.blocks.len())
    }

    fn is_content_row(&self) -> bool {
        self.content_index().is_some()
    }

    /// Single-line text rows (title, labels); content blocks are multi-line and
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

    /// Reassemble the full body from the description + comment blocks (emptied
    /// comments are dropped, so saving an emptied comment deletes it).
    fn assembled_body(&self) -> String {
        let blocks: Vec<content::Block> = self
            .blocks
            .iter()
            .map(|b| content::Block {
                kind: b.kind.clone(),
                text: b.editor.borrow().lines.to_string(),
            })
            .collect();
        content::assemble(&blocks)
    }

    /// Body for a *create* (empty → no body). A create form only has the one
    /// description block.
    fn body_opt(&self) -> Option<String> {
        let b = self.assembled_body();
        if b.trim().is_empty() { None } else { Some(b) }
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
    /// The detail pane's per-line cursor (index into the built detail lines).
    /// j/k move it, Tab snaps it to link lines, Enter follows a link on it.
    detail_line: usize,
    /// Visual-selection anchor (Some when a v/Shift-arrow selection is active).
    detail_anchor: Option<usize>,
    /// Active detail-pane find query + which match is current (n/N cycle).
    detail_find: Option<String>,
    detail_match: usize,
    /// Browser-style navigation history of visited task ids (o = back, i =
    /// forward), driven by following detail links.
    nav_back: Vec<String>,
    nav_fwd: Vec<String>,
    collapsed: HashSet<String>,
    /// Per-view herd-scope overrides, keyed by `View::key` (persisted in the
    /// UI-state cache). A missing entry means the view inherits the global
    /// default ("auto"). See [`view::HerdScope`].
    herd_scope: HashMap<String, view::HerdScope>,
    /// The live view filter applied by the tree (re-colors + prunes).
    filter: FilterSpec,
    /// Approx. list viewport height, refreshed each loop for paging math.
    page: u16,
    /// Approx. detail viewport height (mid area = terminal height - 3),
    /// refreshed each loop; used to keep the active link scrolled into view.
    detail_page: u16,
    /// Detail-pane content width captured at render, used to soft-wrap the
    /// detail lines so the row-indexed model matches what's on screen.
    detail_width: Cell<u16>,
    overlay: Overlay,
    /// Transient one-line status message shown until the next mutation.
    notification: Option<String>,
    /// Editor keybinding profile (vim vs emacs), from herd config.
    editor_vim: bool,
    /// Timestamp of the last `Esc` in an editor overlay, for detecting a rapid
    /// double-`Esc` (a Ctrl-C-equivalent cancel gesture). See [`App::register_double_esc`].
    last_esc: Option<std::time::Instant>,
    /// A dirty edit/create/comment overlay stashed behind a "discard changes?"
    /// confirmation; restored if the user declines. See [`App::request_cancel`].
    dirty_cancel: Option<Overlay>,
    /// The in-progress `:` command line (vim `:w`/`:q`/...), layered over the
    /// active editor overlay while `Some`. See [`App::handle_cmdline_key`].
    cmdline: Option<String>,
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
            detail_line: 0,
            detail_anchor: None,
            detail_find: None,
            detail_match: 0,
            nav_back: Vec::new(),
            nav_fwd: Vec::new(),
            collapsed: HashSet::new(),
            herd_scope: HashMap::new(),
            filter,
            page: 10,
            detail_page: 10,
            detail_width: Cell::new(0),
            overlay: Overlay::None,
            notification: None,
            editor_vim: true,
            last_esc: None,
            dirty_cancel: None,
            cmdline: None,
            quit: false,
        }
    }

    /// Live constructor: loads the current herd view and keeps the handle so
    /// mutations can re-query after each change.
    pub fn with_herd(herd: Herd) -> Result<Self> {
        // Load every status incl. dead. The tree/flat views scope down to what
        // each shows, but keeping dead in the model lets the ancestor walk root
        // a live yak beneath a slaughtered parent, lets a Dead filter surface
        // slaughtered yaks, and treats a dep on a dead yak as resolved (fe00).
        let all = herd.list(FilterSpec::default(), true)?;
        let cfg = herd.config();
        let vim = cfg.vim_mode;
        let ui = cache::load(herd.root());
        let views = views_store::load_views(herd.root());
        let working_set = views_store::load_working_set(herd.root());
        let mut app = App::new(all);
        app.editor_vim = vim;
        app.collapsed = ui.collapsed;
        app.herd_scope = ui.herd;
        app.filter = clone_spec(&views[0].spec);
        app.views = views;
        app.working_set = working_set;
        app.herd = Some(herd);
        app.clamp_cursor();
        Ok(app)
    }

    /// Persist the (rebuildable) UI state — collapsed rows and herd-scope
    /// overrides — to the per-user cache.
    fn save_ui_state(&self) {
        if let Some(h) = &self.herd {
            cache::save(
                h.root(),
                &cache::UiState {
                    collapsed: self.collapsed.clone(),
                    herd: self.herd_scope.clone(),
                },
            );
        }
    }

    /// The herd scope in effect for `v`: its persisted override, else the global
    /// default. Only meaningful for tree views.
    fn resolved_herd_scope(&self, v: &view::View) -> view::HerdScope {
        self.herd_scope
            .get(&v.key)
            .copied()
            .unwrap_or(view::HerdScope::DEFAULT)
    }

    /// Cycle the active view's herd scope (the `h` key): auto -> lone ->
    /// remaining -> all -> auto. Flat/working-set views have no tree, so it's a
    /// no-op there with a hint.
    fn cycle_herd_scope(&mut self) {
        let v = self.active_view();
        if v.is_flat() || v.key == "working-set" {
            self.notification = Some("herd scope applies to tree views".into());
            return;
        }
        let key = v.key.clone();
        let next = view::HerdScope::cycle(self.herd_scope.get(&key).copied());
        match next {
            Some(s) => {
                self.herd_scope.insert(key, s);
            }
            None => {
                self.herd_scope.remove(&key);
            }
        }
        self.save_ui_state();
        self.notification = Some(match next {
            Some(s) => format!("herd: {}", s.as_str()),
            None => format!("herd: ~{}", view::HerdScope::DEFAULT.as_str()),
        });
    }

    /// Re-query the herd view after a mutation and keep the cursor in range.
    fn reload(&mut self) {
        if let Some(h) = &self.herd {
            if let Ok(all) = h.list(FilterSpec::default(), true) {
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
            if let Ok(all) = h.list(FilterSpec::default(), true) {
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
        tree::build(&self.all, &v.spec, self.resolved_herd_scope(v))
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
        // Flat views (Recent/custom) span all statuses, so exclude dead unless
        // the live filter explicitly asks for it (fe00): the model now carries
        // dead, but a working list shouldn't surface slaughtered yaks by default.
        let want_dead = self.filter.statuses.contains(&Status::Dead);
        let mut matched: Vec<&Task> = self
            .all
            .iter()
            .filter(|t| {
                (want_dead || t.status != Status::Dead) && self.filter.matches(t, &resolved)
            })
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
        let scope = self.resolved_herd_scope(v);
        let flat = tree::build(&self.all, &self.filter, scope);
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
                self.save_ui_state();
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
        self.open_edit_focus(None);
    }

    /// `E` from the detail pane: open the edit form focused on whatever the line
    /// cursor sits on — a header field, the description, or a specific comment.
    /// Status routes to its own picker (it isn't a form field).
    fn open_edit_at_cursor(&mut self) {
        match self.edit_target_at(self.detail_line) {
            Some(EditTarget::Status) => self.open_state_picker(),
            other => self.open_edit_focus(other),
        }
    }

    /// Open the edit form, optionally pre-focusing a row derived from `target`.
    fn open_edit_focus(&mut self, target: Option<EditTarget>) {
        let Some(id) = self.selected_id() else { return };
        let mut form = match self.task(&id) {
            Some(task) => CreateForm::for_edit(self.editor_vim, task),
            None => return,
        };
        if let Some(t) = target {
            let last_block = form.blocks.len().saturating_sub(1);
            form.row = match t {
                EditTarget::Title => 0,
                EditTarget::Type => 1,
                EditTarget::Priority => 2,
                EditTarget::Labels => 3,
                EditTarget::Status => 0, // handled by open_edit_at_cursor
                EditTarget::Content(i) => HEADER_ROWS + i.min(last_block),
            };
        }
        self.overlay = Overlay::Create(form);
    }

    /// Map a detail line to what `E` should edit there.
    fn edit_target_at(&self, line: usize) -> Option<EditTarget> {
        let lines = self.detail_dlines();
        let dl = lines.get(line)?;
        if dl.kind == detail::Kind::Field {
            let t = dl.text.as_str();
            for (label, target) in [
                ("Title:", EditTarget::Title),
                ("Type:", EditTarget::Type),
                ("Priority:", EditTarget::Priority),
                ("Labels:", EditTarget::Labels),
                ("Status:", EditTarget::Status),
            ] {
                if t.starts_with(label) {
                    return Some(target);
                }
            }
        }
        if dl.kind == detail::Kind::Body {
            if let Some(Some(b)) = detail::block_index_per_line(&lines).get(line) {
                return Some(EditTarget::Content(*b));
            }
        }
        None
    }

    /// The detail rows where each content block starts (description + comments),
    /// for `Ctrl-N/P` block navigation in the detail pane.
    fn block_starts(&self) -> Vec<usize> {
        let lines = self.detail_dlines();
        let mut starts = Vec::new();
        let mut last: Option<usize> = None;
        for (i, b) in detail::block_index_per_line(&lines).into_iter().enumerate() {
            if let Some(b) = b {
                if Some(b) != last {
                    starts.push(i);
                    last = Some(b);
                }
            }
        }
        starts
    }

    /// Move the line cursor to the next/prev content block start.
    fn jump_block(&mut self, delta: i32) {
        let starts = self.block_starts();
        if starts.is_empty() {
            return;
        }
        let cur = starts
            .iter()
            .rposition(|&s| s <= self.detail_line)
            .unwrap_or(0);
        let next = (cur as i32 + delta).clamp(0, starts.len() as i32 - 1) as usize;
        self.detail_line = starts[next];
        self.scroll_line_into_view(self.detail_line as u16);
    }

    fn handle_create_key(&mut self, k: KeyEvent, double_esc: bool) {
        let ctrl = k.modifiers.contains(KeyModifiers::CONTROL);
        let vim = self.editor_vim;
        let (is_content, is_line_text) = match &self.overlay {
            Overlay::Create(f) => (f.is_content_row(), f.is_line_text_row()),
            _ => return,
        };
        // Commit (Ctrl-S) / cancel (Ctrl-C, or Esc outside a content block —
        // inside one Esc belongs to the editor, e.g. vim normal mode).
        if ctrl && k.code == KeyCode::Char('s') {
            let has_title =
                matches!(&self.overlay, Overlay::Create(f) if !f.title_text().is_empty());
            if has_title {
                self.commit_form();
            }
            return;
        }
        // In vim every field is modal: a lone Esc drops the focused field to
        // Normal (or is a no-op on chip rows) and never cancels the form — use
        // Ctrl-C or a rapid double-Esc. Emacs keeps single-Esc-cancels on the
        // non-content rows.
        let esc_cancels = k.code == KeyCode::Esc && !vim && !is_content;
        if (ctrl && k.code == KeyCode::Char('c')) || double_esc || esc_cancels {
            let (editing, dirty) = match &self.overlay {
                Overlay::Create(f) => (f.is_editing(), self.form_is_dirty(f)),
                _ => (false, false),
            };
            let msg = if editing {
                "edit cancelled"
            } else {
                "create cancelled"
            };
            self.request_cancel(dirty, msg);
            return;
        }
        // Row navigation: Tab / Shift-Tab / Ctrl-N / Ctrl-P always move rows.
        // On chip rows j/k also navigate; on single-line text rows Up/Down do;
        // content blocks keep Up/Down for their own cursor.
        let is_chip = !is_content && !is_line_text;
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
                let n = f.row_count();
                f.row = if nav_down {
                    (f.row + 1) % n
                } else {
                    (f.row + n - 1) % n
                };
            }
            return;
        }
        // Enter on a single-line/chip row advances to the next row; in a content
        // block it inserts a newline (handled by the editor below).
        if k.code == KeyCode::Enter && !is_content {
            if let Overlay::Create(f) = &mut self.overlay {
                let n = f.row_count();
                f.row = (f.row + 1) % n;
            }
            return;
        }
        if is_content {
            // `:` in a content block's Normal mode opens the command line.
            let open_cmd = vim
                && k.code == KeyCode::Char(':')
                && matches!(&self.overlay, Overlay::Create(f)
                    if f.content_index().is_some_and(|i|
                        f.blocks[i].editor.borrow().mode == EditorMode::Normal));
            if open_cmd {
                self.cmdline = Some(String::new());
                return;
            }
            if let Overlay::Create(f) = &mut self.overlay {
                if let Some(i) = f.content_index() {
                    f.handler
                        .on_key_event(k, &mut f.blocks[i].editor.borrow_mut());
                }
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
                description: f.body_opt(),
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
        let body = f.assembled_body();
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
        if body != cur.body {
            edit.description = Some(body);
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

    /// A — attach an artifact (a file path, or the clipboard PNG when blank).
    fn open_attach(&mut self) {
        if let Some(id) = self.selected_id() {
            self.overlay = Overlay::Edit(Editor::new(
                self.editor_vim,
                true,
                "Attach path (empty = clipboard PNG): ".into(),
                "",
                EditAction::Attach(id),
            ));
        }
    }

    fn commit_attach(&mut self, id: String, path_input: String) {
        let path_input = path_input.trim();
        let (name, data) = if path_input.is_empty() {
            match crate::clipboard::read_png() {
                Some(bytes) => (
                    format!("paste-{}.png", chrono::Utc::now().format("%Y%m%d-%H%M%S")),
                    bytes,
                ),
                None => {
                    self.notification = Some("no PNG image on clipboard".into());
                    return;
                }
            }
        } else {
            let p = std::path::Path::new(path_input);
            let Ok(bytes) = std::fs::read(p) else {
                self.notification = Some(format!("not a file: {path_input}"));
                return;
            };
            let name = p
                .file_name()
                .and_then(|s| s.to_str())
                .unwrap_or("attachment")
                .to_string();
            (name, bytes)
        };
        let Some(h) = &self.herd else { return };
        match h.attach(&id, &name, &data) {
            Ok(AttachOutcome::Attached(n)) => {
                self.reload();
                self.notification = Some(format!("attached {n}"));
            }
            Ok(AttachOutcome::NotFound) => self.notification = Some(format!("{id} not found")),
            Err(e) => self.notification = Some(format!("attach failed: {e}")),
        }
    }

    /// Open a URL or artifact path in the OS default application (best-effort).
    fn open_external(&mut self, target: &str) {
        let arg = if target.starts_with("http") {
            target.to_string()
        } else {
            match &self.herd {
                Some(h) => h.root().join(target).display().to_string(),
                None => target.to_string(),
            }
        };
        let opener = if cfg!(target_os = "macos") {
            "open"
        } else {
            "xdg-open"
        };
        match std::process::Command::new(opener).arg(&arg).spawn() {
            Ok(_) => self.notification = Some(format!("opened {target}")),
            Err(e) => self.notification = Some(format!("open failed: {e}")),
        }
    }

    /// O — open the artifact/URL link on the current detail line externally.
    fn open_current_external(&mut self) {
        let jumps = self.detail_jumps();
        match jumps.into_iter().find(|j| j.line == self.detail_line) {
            Some(j) => match j.target {
                detail::Target::Artifact(p) => self.open_external(&p),
                detail::Target::Url(u) => self.open_external(&u),
                detail::Target::Task(_) => self.notification = Some("not an artifact/link".into()),
            },
            None => self.notification = Some("no link on this line".into()),
        }
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
            self.save_ui_state();
        }
    }

    /// The detail lines for the current selection, soft-wrapped to the width
    /// captured at the last render. This is the single source of truth for the
    /// row-indexed detail model (cursor, jumplist, find, scroll, render).
    fn detail_dlines(&self) -> Vec<detail::DLine> {
        match self.selected() {
            Some(t) => detail::wrap(
                detail::build(t, &self.all),
                self.detail_width.get() as usize,
            ),
            None => Vec::new(),
        }
    }

    /// Detail jumplist for the current selection (empty when nothing selected).
    fn detail_jumps(&self) -> Vec<detail::Jump> {
        detail::jumplist(&self.detail_dlines())
    }

    fn detail_line_count(&self) -> usize {
        self.detail_dlines().len()
    }

    /// Move the detail line cursor by `delta`, clamped, keeping it in view.
    fn move_detail_line(&mut self, delta: i32) {
        let n = self.detail_line_count();
        if n == 0 {
            return;
        }
        self.detail_line = (self.detail_line as i32 + delta).clamp(0, n as i32 - 1) as usize;
        self.scroll_line_into_view(self.detail_line as u16);
    }

    /// g / G — jump the line cursor to the top / bottom.
    fn detail_line_to(&mut self, end: bool) {
        let n = self.detail_line_count();
        self.detail_line = if end { n.saturating_sub(1) } else { 0 };
        self.scroll_line_into_view(self.detail_line as u16);
    }

    /// Tab / Shift-Tab — snap the line cursor to the next / prev line holding a
    /// link (wrapping), so Enter can follow it.
    fn jump_link(&mut self, delta: i32) {
        let jumps = self.detail_jumps();
        if jumps.is_empty() {
            return;
        }
        let mut lines: Vec<usize> = jumps.iter().map(|j| j.line).collect();
        lines.dedup();
        let target = if delta >= 0 {
            lines
                .iter()
                .find(|&&l| l > self.detail_line)
                .copied()
                .unwrap_or(lines[0])
        } else {
            lines
                .iter()
                .rev()
                .find(|&&l| l < self.detail_line)
                .copied()
                .unwrap_or_else(|| *lines.last().unwrap())
        };
        self.detail_line = target;
        self.scroll_line_into_view(self.detail_line as u16);
    }

    /// v — toggle visual selection anchored at the current line.
    fn toggle_visual(&mut self) {
        self.detail_anchor = if self.detail_anchor.is_some() {
            None
        } else {
            Some(self.detail_line)
        };
    }

    /// Shift-↑↓ — start (if needed) and extend the visual selection.
    fn extend_selection(&mut self, delta: i32) {
        if self.detail_anchor.is_none() {
            self.detail_anchor = Some(self.detail_line);
        }
        self.move_detail_line(delta);
    }

    /// The inclusive [lo, hi] line range currently selected, if any.
    fn selection_range(&self) -> Option<(usize, usize)> {
        self.detail_anchor
            .map(|a| (a.min(self.detail_line), a.max(self.detail_line)))
    }

    /// y / Enter (in visual mode) — copy the selected line block (dedented) to
    /// the clipboard and clear the selection.
    fn yank_selection(&mut self) {
        let Some((lo, hi)) = self.selection_range() else {
            return;
        };
        let text = match self.selected() {
            Some(_) => {
                let lines = self.detail_dlines();
                if lines.is_empty() {
                    return;
                }
                let hi = hi.min(lines.len() - 1);
                // Rejoin soft-wrapped continuations into their logical line so a
                // yanked paragraph doesn't carry hard breaks at wrap points.
                let mut rejoined: Vec<String> = Vec::new();
                for line in &lines[lo..=hi] {
                    if line.cont {
                        if let Some(last) = rejoined.last_mut() {
                            last.push(' ');
                            last.push_str(&line.text);
                            continue;
                        }
                    }
                    rejoined.push(line.text.clone());
                }
                dedent(&rejoined).join("\n")
            }
            None => return,
        };
        let ok = crate::clipboard::copy_text(&text);
        self.detail_anchor = None;
        let n = hi - lo + 1;
        self.notification = Some(if ok {
            format!("copied {n} line(s)")
        } else {
            "clipboard unavailable".into()
        });
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
        if self.selected().is_none() {
            return vec![];
        }
        detail_scan(&self.detail_dlines(), q)
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
        self.detail_line = 0;
        self.detail_anchor = None;
        self.detail_find = None;
        self.detail_match = 0;
    }

    fn follow_link(&mut self) {
        let jumps = self.detail_jumps();
        let Some(j) = jumps.into_iter().find(|j| j.line == self.detail_line) else {
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
            detail::Target::Url(u) => self.open_external(&u),
            detail::Target::Artifact(p) => self.open_external(&p),
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
            self.detail_line = 0;
            self.detail_anchor = None;
            self.detail_find = None;
            self.detail_match = 0;
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

    /// Record an `Esc` in an editor overlay and report whether it's a *rapid*
    /// second `Esc` (within [`DOUBLE_ESC_MS`] of the previous one) — the
    /// Ctrl-C-equivalent cancel gesture. A lone `Esc` keeps its usual meaning
    /// (leave Insert for Normal); only the fast second press cancels.
    fn register_double_esc(&mut self) -> bool {
        let now = std::time::Instant::now();
        let double = self.last_esc.is_some_and(|t| {
            now.duration_since(t) <= std::time::Duration::from_millis(DOUBLE_ESC_MS)
        });
        // Clear after a double so a third quick Esc doesn't also fire.
        self.last_esc = if double { None } else { Some(now) };
        double
    }

    /// Cancel the current edit overlay. When `dirty`, stash it behind a "discard
    /// changes?" confirmation (restored if declined) instead of dropping the work;
    /// otherwise cancel immediately with `msg`.
    fn request_cancel(&mut self, dirty: bool, msg: &str) {
        if dirty {
            let stashed = std::mem::replace(&mut self.overlay, Overlay::None);
            self.dirty_cancel = Some(stashed);
            self.overlay = Overlay::Confirm {
                prompt: "Discard changes? (y/N): ".into(),
                action: ConfirmAction::DiscardEdit,
            };
        } else {
            self.overlay = Overlay::None;
            self.notification = Some(msg.to_string());
        }
    }

    /// Whether the create/edit form differs from its starting point (the seeded
    /// task when editing, or the empty defaults when creating).
    fn form_is_dirty(&self, f: &CreateForm) -> bool {
        match &f.edit_id {
            Some(id) => match self.task(id) {
                Some(cur) => {
                    let mut a = f.labels_vec();
                    let mut b = cur.labels.clone();
                    a.sort();
                    b.sort();
                    f.title_text() != cur.title
                        || TYPE_CHOICES[f.kind_idx] != cur.kind
                        || PRI_CHOICES[f.pri_idx] != cur.priority
                        || a != b
                        || f.assembled_body() != cur.body
                }
                None => true,
            },
            None => {
                !f.title_text().is_empty()
                    || !f.assembled_body().is_empty()
                    || !f.labels_vec().is_empty()
                    || f.kind_idx != 0
                    || f.pri_idx != pri_index(3)
            }
        }
    }

    /// Edit the active `:` command line. Enter runs it, Esc dismisses it,
    /// Backspace deletes, and printable chars are appended.
    fn handle_cmdline_key(&mut self, k: KeyEvent) {
        match k.code {
            KeyCode::Esc => self.cmdline = None,
            KeyCode::Enter => {
                let cmd = self.cmdline.take().unwrap_or_default();
                self.run_command(&cmd);
            }
            KeyCode::Backspace => {
                if let Some(s) = &mut self.cmdline {
                    s.pop();
                }
            }
            KeyCode::Char(c) => {
                if let Some(s) = &mut self.cmdline {
                    s.push(c);
                }
            }
            _ => {}
        }
    }

    /// Run a `:` command against the active editor overlay. The verb set is
    /// deliberately small (`w`/`q`/`wq`/`x`/`q!`); the match's catch-all is the
    /// extension point for any future `:` commands.
    fn run_command(&mut self, cmd: &str) {
        match cmd.trim() {
            "" => {}
            "w" | "wq" | "x" => self.cmd_write(),
            "q" => self.cmd_quit(false),
            "q!" => self.cmd_quit(true),
            other => self.notification = Some(format!("unknown command: :{other}")),
        }
    }

    /// `:w` / `:wq` / `:x` — commit the active overlay (a modal editor saves and
    /// closes, so all three behave alike).
    fn cmd_write(&mut self) {
        match &self.overlay {
            Overlay::Create(f) => {
                if f.title_text().is_empty() {
                    self.notification = Some("need a title".into());
                } else {
                    self.commit_form();
                }
            }
            Overlay::Edit(_) => {
                if let Overlay::Edit(ed) = std::mem::replace(&mut self.overlay, Overlay::None) {
                    self.commit_edit(ed);
                }
            }
            _ => {}
        }
    }

    /// `:q` — cancel the active overlay (respecting the dirty-discard confirm);
    /// `:q!` force-cancels, discarding unsaved work outright.
    fn cmd_quit(&mut self, force: bool) {
        if force {
            self.overlay = Overlay::None;
            self.notification = Some("cancelled".into());
            return;
        }
        let (dirty, msg) = match &self.overlay {
            Overlay::Create(f) => (
                self.form_is_dirty(f),
                if f.is_editing() {
                    "edit cancelled"
                } else {
                    "create cancelled"
                },
            ),
            Overlay::Edit(ed) => (!ed.single_line && !ed.text().trim().is_empty(), "cancelled"),
            _ => (false, "cancelled"),
        };
        self.request_cancel(dirty, msg);
    }

    fn handle_overlay_key(&mut self, k: KeyEvent) {
        // An active `:` command line intercepts everything until it resolves.
        if self.cmdline.is_some() {
            self.handle_cmdline_key(k);
            return;
        }
        let ctrl = k.modifiers.contains(KeyModifiers::CONTROL);
        // A rapid double-Esc acts as Ctrl-C (cancel) in editor overlays, so Esc
        // can otherwise mean "leave Insert for Normal" without trapping the user.
        let double_esc = k.code == KeyCode::Esc && self.register_double_esc();
        if matches!(self.overlay, Overlay::Drawer(_)) {
            self.handle_drawer_key(k);
            return;
        }
        if matches!(self.overlay, Overlay::Create(_)) {
            self.handle_create_key(k, double_esc);
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
        if matches!(self.overlay, Overlay::Edit(_)) {
            // Decide within a short borrow, then act once it's released.
            let (do_commit, do_cancel, dirty, open_cmd) = {
                let Overlay::Edit(ed) = &mut self.overlay else {
                    unreachable!()
                };
                let commit = (ctrl && k.code == KeyCode::Char('s'))
                    || (ed.single_line && k.code == KeyCode::Enter);
                // `:` in a multiline editor's Normal mode opens the command line.
                let open_cmd = !ed.single_line
                    && ed.vim
                    && k.code == KeyCode::Char(':')
                    && ed.state.borrow().mode == EditorMode::Normal;
                // A single-line field is fully modal in vim: a lone Esc only
                // drops to Normal (handed to edtui) and never cancels, so the
                // whole normal-mode keymap (b/w/0/$/dd/x/yy/p/...) is reachable.
                // Cancel is Ctrl-C or a rapid double-Esc; emacs keeps plain
                // single-Esc-cancels. In a multiline editor a lone Esc always
                // just drops to Normal, so double-Esc is the only Esc-way out.
                let esc_cancels = ed.single_line && k.code == KeyCode::Esc && !ed.vim;
                let cancel = (ctrl && k.code == KeyCode::Char('c')) || esc_cancels || double_esc;
                if commit {
                    (true, false, false, false)
                } else if open_cmd {
                    (false, false, false, true)
                } else if cancel {
                    // Only the multiline comment editor guards unsaved work.
                    let dirty = !ed.single_line && !ed.text().trim().is_empty();
                    (false, true, dirty, false)
                } else {
                    ed.handler.on_key_event(k, &mut ed.state.borrow_mut());
                    (false, false, false, false)
                }
            };
            if do_commit {
                if let Overlay::Edit(ed) = std::mem::replace(&mut self.overlay, Overlay::None) {
                    self.commit_edit(ed);
                }
            } else if open_cmd {
                self.cmdline = Some(String::new());
            } else if do_cancel {
                self.request_cancel(dirty, "cancelled");
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
                    // Declining a discard-confirm returns to the stashed editor.
                    if matches!(action, ConfirmAction::DiscardEdit) {
                        if let Some(ov) = self.dirty_cancel.take() {
                            self.overlay = ov;
                        }
                    } else {
                        self.notification = Some("cancelled".into());
                    }
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
            EditAction::Attach(id) => self.commit_attach(id, text),
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
            ConfirmAction::DiscardEdit => {
                self.dirty_cancel = None; // drop the stashed editor
                self.notification = Some("changes discarded".into());
            }
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
            KeyCode::Char('A') => app.open_attach(),
            KeyCode::Char('v') => app.open_view_picker(),
            KeyCode::Char('V') => app.open_save_view(),
            KeyCode::Char('?') => app.open_help(),
            KeyCode::Esc => {
                if app.is_view_modified() {
                    app.revert_filter_to_view();
                    app.notification = Some("reverted to view".into());
                }
            }
            KeyCode::Char('h') => app.cycle_herd_scope(),
            KeyCode::Char('l') | KeyCode::Right | KeyCode::Enter => {
                if app.selected().is_some() {
                    app.focus = Focus::Detail;
                    app.detail_scroll = 0;
                    app.detail_line = 0;
                    app.detail_anchor = None;
                    app.detail_find = None;
                    app.detail_match = 0;
                }
            }
            _ => {}
        },
        Focus::Detail => {
            // Shift-↑↓ extend a visual selection (checked before the code match
            // so the modifier isn't lost).
            if k.modifiers.contains(KeyModifiers::SHIFT)
                && matches!(k.code, KeyCode::Up | KeyCode::Down)
            {
                app.extend_selection(if k.code == KeyCode::Down { 1 } else { -1 });
                return;
            }
            // Ctrl-N/P jump the line cursor between content blocks (description
            // and each comment), mirroring the edit form's field navigation.
            if ctrl && matches!(k.code, KeyCode::Char('n') | KeyCode::Char('p')) {
                app.jump_block(if k.code == KeyCode::Char('n') { 1 } else { -1 });
                return;
            }
            match k.code {
                KeyCode::Char('q') => app.quit = true,
                KeyCode::Char('h') | KeyCode::Left => app.focus = Focus::List,
                // Esc peels back: selection → find → back to list (Python layering).
                KeyCode::Esc => {
                    if app.detail_anchor.is_some() {
                        app.detail_anchor = None;
                    } else if app.detail_find.is_some() {
                        app.detail_find = None;
                    } else {
                        app.focus = Focus::List;
                    }
                }
                KeyCode::Tab | KeyCode::Char(']') => app.jump_link(1),
                KeyCode::BackTab | KeyCode::Char('[') => app.jump_link(-1),
                KeyCode::Char('o') => app.nav_back(),
                KeyCode::Char('i') => app.nav_forward(),
                KeyCode::Char('/') => app.open_detail_find(),
                KeyCode::Char('n') => app.detail_find_jump(1),
                KeyCode::Char('N') => app.detail_find_jump(-1),
                KeyCode::Char('?') => app.open_help(),
                KeyCode::Char('v') => app.toggle_visual(),
                // Enter yanks an active selection, else follows the link on the line.
                KeyCode::Enter => {
                    if app.detail_anchor.is_some() {
                        app.yank_selection();
                    } else {
                        app.follow_link();
                    }
                }
                // Mutating ops mirrored from the list pane (all act on selected()).
                KeyCode::Char('S') => app.open_state_picker(),
                KeyCode::Char('P') => app.open_priority_picker(),
                KeyCode::Char('T') => app.open_type_picker(),
                KeyCode::Char('L') => app.open_labels(),
                KeyCode::Char('X') => app.open_slaughter_confirm(),
                KeyCode::Char('E') => app.open_edit_at_cursor(),
                KeyCode::Char('D') => app.open_dep_picker(),
                KeyCode::Char('R') => app.open_reparent_picker(),
                KeyCode::Char('c') => app.open_create(false),
                KeyCode::Char('C') => app.open_create(true),
                KeyCode::Char('f') => app.open_drawer(),
                KeyCode::Char('*') => app.toggle_star(),
                // y yanks an active selection, else copies the task id.
                KeyCode::Char('y') => {
                    if app.detail_anchor.is_some() {
                        app.yank_selection();
                    } else {
                        app.copy_selected_id();
                    }
                }
                KeyCode::Char('M') => app.open_comment(),
                KeyCode::Char('A') => app.open_attach(),
                KeyCode::Char('O') => app.open_current_external(),
                // Move between tasks without leaving the detail pane.
                KeyCode::Char('J') => app.detail_next_task(1),
                KeyCode::Char('K') => app.detail_next_task(-1),
                // The line cursor (j/k/d/u/g/G); it auto-scrolls to stay in view.
                KeyCode::Char('j') | KeyCode::Down => app.move_detail_line(1),
                KeyCode::Char('k') | KeyCode::Up => app.move_detail_line(-1),
                KeyCode::Char('d') => app.move_detail_line(half),
                KeyCode::Char('u') => app.move_detail_line(-half),
                KeyCode::Char('g') => app.detail_line_to(false),
                KeyCode::Char('G') => app.detail_line_to(true),
                _ => {}
            }
        }
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
            // 📌 pinned, 🔒 builtin — emoji glyphs for the view manager.
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
        entry("Ctrl-N / Ctrl-P", "Next / prev content block"),
        entry("v , Shift-↑↓", "Visual select lines"),
        entry("y / Enter", "Copy selection / follow link"),
        entry("/ , n / N", "Find, next / prev match"),
        blank(),
        section("Edit"),
        entry("c / C", "New root / child yak"),
        entry("E", "Edit field/desc/comment at cursor"),
        entry("P / T / L / S", "Priority / type / labels / state"),
        entry("D / R", "Add dependency / reparent"),
        entry("M", "Add a comment (note)"),
        entry(
            ":w / :q / :wq",
            "Save / cancel / save+close (editor Normal)",
        ),
        entry("A / O", "Attach artifact / open it"),
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
    let [header_r, title_r, type_r, pri_r, labels_r, content_r] = Layout::vertical([
        Constraint::Length(1), // header
        Constraint::Length(1), // title
        Constraint::Length(1), // type chips
        Constraint::Length(1), // priority chips
        Constraint::Length(1), // labels
        Constraint::Min(0),    // content-block stack
    ])
    .areas(area);
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
        header_r,
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
        title_r,
    );
    render_chip_row(
        f.row == 1,
        f.kind_idx,
        CREATE_LABEL_W,
        "type",
        &types,
        frame,
        type_r,
    );
    render_chip_row(
        f.row == 2,
        f.pri_idx,
        CREATE_LABEL_W,
        "priority",
        &pris,
        frame,
        pri_r,
    );
    render_text_row(
        f.row == 3,
        CREATE_LABEL_W,
        "labels",
        &f.labels,
        "",
        frame,
        labels_r,
    );
    render_content_stack(f, frame, content_r);
}

/// Render the description + comment blocks as an accordion: one labeled
/// separator per block, with the "expanded" block's body beneath its separator.
/// The focused block (cursor on a content row) shows a live editor; otherwise
/// the description shows as a dimmed, wrapped preview so it's visible at a
/// glance while the cursor sits on a header field.
fn render_content_stack(f: &CreateForm, frame: &mut Frame, area: Rect) {
    let focused = f.content_index();
    let expand = focused.unwrap_or(0); // description is shown by default
    let mut constraints = Vec::new();
    for i in 0..f.blocks.len() {
        constraints.push(Constraint::Length(1)); // separator
        if i == expand {
            constraints.push(Constraint::Min(0)); // block body
        }
    }
    let rects = Layout::vertical(constraints).split(area);
    let mut ri = 0;
    for (i, block) in f.blocks.iter().enumerate() {
        let is_focused = focused == Some(i);
        render_block_separator(block, is_focused, frame, rects[ri]);
        ri += 1;
        if i != expand {
            continue;
        }
        let body = rects[ri];
        ri += 1;
        if is_focused {
            let mut st = block.editor.borrow_mut();
            let mode = st.mode;
            set_md_highlights(&mut st);
            frame.render_widget(EditorView::new(&mut st).theme(editor_theme(mode)), body);
        } else {
            let placeholder = match &block.kind {
                content::BlockKind::Description => "(no description)",
                content::BlockKind::Comment { .. } => "(empty)",
            };
            let text = block.editor.borrow().lines.to_string();
            let shown = if text.trim().is_empty() {
                placeholder.to_string()
            } else {
                text
            };
            frame.render_widget(
                Paragraph::new(shown)
                    .style(Style::new().fg(Color::DarkGray))
                    .wrap(Wrap { trim: false }),
                body,
            );
        }
    }
}

/// One accordion separator: `▸ label ───` (cyan marker when focused, dim
/// otherwise). Comment blocks label with their date.
fn render_block_separator(block: &ContentBlock, focused: bool, frame: &mut Frame, area: Rect) {
    let label = match &block.kind {
        content::BlockKind::Description => "description".to_string(),
        content::BlockKind::Comment { timestamp } => {
            let date = timestamp.get(..10).unwrap_or(timestamp);
            format!("comment · {date}")
        }
    };
    let marker = if focused { "▸ " } else { "  " };
    let head = format!("{marker}{label} ");
    let dashes = (area.width as usize).saturating_sub(disp_width(&head));
    let color = if focused {
        Color::Cyan
    } else {
        Color::DarkGray
    };
    frame.render_widget(
        Paragraph::new(Line::from(vec![
            Span::styled(head, Style::new().fg(color)),
            Span::styled("─".repeat(dashes), Style::new().fg(Color::DarkGray)),
        ])),
        area,
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
        let mode = st.mode;
        frame.render_widget(
            EditorView::new(&mut st)
                .theme(editor_theme(mode))
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

/// Short mode label shown next to a vim editor so the current mode is visible.
fn mode_tag(mode: EditorMode) -> &'static str {
    match mode {
        EditorMode::Normal => "NORMAL",
        EditorMode::Insert => "INSERT",
        EditorMode::Visual => "VISUAL",
        EditorMode::Search => "SEARCH",
    }
}

fn mode_style(mode: EditorMode) -> Style {
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
fn set_md_highlights(state: &mut EditorState) {
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
fn editor_theme(mode: EditorMode) -> EditorTheme<'static> {
    let cursor = match mode {
        EditorMode::Insert => Style::new().add_modifier(Modifier::UNDERLINED),
        _ => Style::new().bg(Color::White).fg(Color::Black),
    };
    EditorTheme::default()
        .hide_status_line()
        .block(Block::default())
        .cursor_style(cursor)
}

fn render_editor_panel(ed: &Editor, frame: &mut Frame, area: Rect) {
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
    let mode = st.mode;
    frame.render_widget(
        EditorView::new(&mut st)
            .theme(editor_theme(mode))
            .single_line(true),
        fld,
    );
}

fn render_line_editor(ed: &Editor, frame: &mut Frame, area: Rect) {
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
    let tabs_w = disp_width(&spans.iter().map(|s| s.content.as_ref()).collect::<String>());
    frame.render_widget(Paragraph::new(Line::from(spans)), area);
    // Right-aligned slot on the tab row: a transient notification takes it when
    // present (Python placement); otherwise the persistent herd-scope indicator
    // for tree views. `~` marks a view inheriting the global default (auto) vs
    // an explicit per-view override. The indicator yields rather than overwrite
    // tabs when the strip is too wide to fit it.
    let right: Option<(String, Style)> = if let Some(n) = &app.notification {
        Some((
            n.clone(),
            Style::new().fg(Color::Yellow).add_modifier(Modifier::BOLD),
        ))
    } else {
        let av = app.active_view();
        if av.is_flat() || av.key == "working-set" {
            None
        } else {
            let (marker, val) = match app.herd_scope.get(&av.key) {
                Some(s) => ("", s.as_str()),
                None => ("~", view::HerdScope::DEFAULT.as_str()),
            };
            let label = format!("herd: {marker}{val}");
            let fits = tabs_w + disp_width(&label) < area.width as usize;
            fits.then(|| (label, Style::new().fg(Color::DarkGray)))
        }
    };
    if let Some((text, style)) = right {
        let w = (disp_width(&text) as u16).min(area.width);
        if w > 0 {
            let rect = Rect {
                x: area.x + area.width - w,
                y: area.y,
                width: w,
                height: 1,
            };
            frame.render_widget(Paragraph::new(Span::styled(text, style)), rect);
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
    if app.selected().is_none() {
        let p = Paragraph::new(Span::styled("(no task)", Style::new().fg(Color::DarkGray)));
        frame.render_widget(p, area);
        return;
    };
    // Capture the content width so the row-indexed model (and event handlers)
    // wrap to exactly what's rendered here.
    app.detail_width.set(area.width);
    let lines = app.detail_dlines();
    let jumps = detail::jumplist(&lines);
    // The "current" link is whichever link sits on the line cursor.
    let cur = if focused {
        jumps.iter().find(|j| j.line == app.detail_line)
    } else {
        None
    };
    let sel = if focused { app.selection_range() } else { None };
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
            // Subtle background for the line cursor and any visual selection.
            let in_sel = sel.is_some_and(|(lo, hi)| i >= lo && i <= hi);
            let line_bg = if focused && (i == app.detail_line || in_sel) {
                Some(Color::Indexed(237))
            } else {
                None
            };
            render_dline(dl, cur, i, &lm, line_bg)
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
    line_bg: Option<Color>,
) -> Line<'a> {
    let chars: Vec<char> = dl.text.chars().collect();
    let n = chars.len();
    if n == 0 {
        // Still paint the cursor/selection background across an empty line.
        if let Some(bg) = line_bg {
            return Line::from(Span::styled(" ", Style::new().bg(bg)));
        }
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
            let base = if i < label_end {
                Style::new().fg(Color::DarkGray)
            } else if dl.kind == detail::Kind::Section {
                Style::new().fg(Color::Cyan).add_modifier(Modifier::BOLD)
            } else {
                Style::new()
            };
            match line_bg {
                Some(bg) => base.bg(bg),
                None => base,
            }
        })
        .collect();
    // Markdown highlight layer (body lines): sits beneath links/find so a link
    // inside emphasis still wins. Keep the line's cursor/selection background.
    for sp in &dl.md {
        let end = (sp.start + sp.len).min(n);
        let st = match line_bg {
            Some(bg) => sp.style.bg(bg),
            None => sp.style,
        };
        for s in styles.iter_mut().take(end).skip(sp.start) {
            *s = st;
        }
    }
    for (col, len, _) in &dl.links {
        let is_current = cur.is_some_and(|j| j.line == line_idx && j.col == *col);
        let mut st = if is_current {
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
        // Preserve the line's cursor/selection background on non-current links.
        if !is_current {
            if let Some(bg) = line_bg {
                st = st.bg(bg);
            }
        }
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

/// Strip the common leading-space prefix across non-empty lines (Python's
/// `dedent_block`), so a copied detail block isn't indented by the pane layout.
fn dedent(lines: &[String]) -> Vec<String> {
    let strip = lines
        .iter()
        .filter(|s| !s.trim().is_empty())
        .map(|s| s.len() - s.trim_start_matches(' ').len())
        .min()
        .unwrap_or(0);
    if strip == 0 {
        return lines.to_vec();
    }
    lines
        .iter()
        .map(|s| {
            if s.len() >= strip {
                s[strip..].to_string()
            } else {
                s.clone()
            }
        })
        .collect()
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
    // An active `:` command line owns the status line.
    if let Some(cmd) = &app.cmdline {
        frame.render_widget(
            Paragraph::new(Span::styled(
                format!(":{cmd}"),
                Style::new().fg(Color::Yellow).add_modifier(Modifier::BOLD),
            )),
            area,
        );
        return;
    }
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
        // In vim, Esc is modal (drops the field to Normal); cancel is a rapid
        // double-Esc or Ctrl-C. In emacs, Esc cancels directly.
        let cancel = if app.editor_vim {
            "EscEsc/Ctrl-C cancel"
        } else {
            "Esc cancel"
        };
        frame.render_widget(
            Paragraph::new(Span::styled(
                format!("Tab/↑↓ rows · ←→ chips · {commit} · {cancel}"),
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
            "Tab:view  j/k:move  l:detail  h:herd  v:views  c/C:new  E:edit  X:del  S:state  D:dep  {filter_hint}  ?:help"
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
        // Hairy tab under `all` scope: Root A + A1 (focus), A2 (ghost, shorn)
        // pulled in as family.
        let mut app = sample();
        app.herd_scope
            .insert("status:hairy".into(), view::HerdScope::All);
        insta::assert_snapshot!(draw(&app, 72, 14));
    }

    #[test]
    fn herd_indicator_on_tab_row() {
        // Wide enough for the right-aligned herd indicator to sit past the tabs.
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
        app.herd_scope
            .insert("status:hairy".into(), view::HerdScope::All);
        let rows = app.rows();
        let ids: Vec<&str> = rows.iter().map(|r| r.task.id.as_str()).collect();
        assert_eq!(ids, vec!["a0", "a1", "a2"]); // b0 (shaving) excluded from Hairy tab
        let a2 = rows.iter().find(|r| r.task.id == "a2").unwrap();
        assert!(a2.ghost, "shorn child should be a ghost in the Hairy tab");
        let a0 = rows.iter().find(|r| r.task.id == "a0").unwrap();
        assert!(a0.has_children && !a0.ghost);
    }

    #[test]
    fn herd_remaining_default_hides_completed_leaf() {
        // The default scope (remaining) drops the shorn leaf a2; open a1 stays.
        let app = sample();
        let ids: Vec<&str> = app.rows().iter().map(|r| r.task.id.as_str()).collect();
        assert_eq!(ids, vec!["a0", "a1"]);
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
        let b = crate::store::append_note("The description.", "2026-01-01T00:00:00Z", "first note");
        t.body = crate::store::append_note(&b, "2026-01-02T00:00:00Z", "second note");
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
            // Dead stays in the model (so deps/ancestors still resolve) but is
            // hidden from the default Hairy view.
            assert_eq!(app.task("t0").unwrap().status, Status::Dead);
            assert!(app.rows().iter().all(|r| r.task.id != "t0"));
            assert_eq!(app.notification.as_deref(), Some("slaughtered t0"));
        }

        #[test]
        fn dead_is_loaded_but_hidden_until_filtered() {
            let (_dir, herd) = temp_herd(&[
                task("t0", "alive", Status::Hairy, 3, None),
                task("t1", "slain", Status::Dead, 3, None),
            ]);
            let mut app = App::with_herd(herd).unwrap();
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
        fn attach_file_writes_artifact_and_links_body() {
            let (proj, herd) = temp_herd(&[task("t0", "solo", Status::Hairy, 3, None)]);
            let src = proj.join("shot.png");
            fs::write(&src, b"not-really-a-png").unwrap();
            let mut app = App::with_herd(herd).unwrap();
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
                "artifact copied into the herd"
            );
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
            let (_dir, herd) = temp_herd(&[task("t0", "solo", Status::Hairy, 3, None)]);
            let mut app = App::with_herd(herd).unwrap();
            press(&mut app, "M"); // add a comment
            press(&mut app, "original note");
            ctrl_s(&mut app);
            let before = app.task("t0").unwrap().body.clone();
            assert!(before.contains('\u{25b8}'));
            // Edit: walk title→type→priority→labels→description→comment. The
            // seeded comment opens in Normal (921a), so `i` to insert, then type.
            press(&mut app, "E");
            for _ in 0..5 {
                tab(&mut app);
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
            let (_dir, herd) = temp_herd(&[task("t0", "solo", Status::Hairy, 3, None)]);
            let mut app = App::with_herd(herd).unwrap();
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
            let (_dir, herd) = temp_herd(&[task("t0", "solo", Status::Hairy, 3, None)]);
            let mut app = App::with_herd(herd).unwrap();
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
            let (_dir, herd) = temp_herd(&[task("t0", "solo", Status::Hairy, 3, None)]);
            let mut app = App::with_herd(herd).unwrap();
            press(&mut app, "M");
            press(&mut app, "draft");
            esc(&mut app); // multiline: Esc drops to Normal, does NOT cancel
            assert!(matches!(app.overlay, Overlay::Edit(_)), "editor stays open");
        }

        #[test]
        fn double_esc_cancels_the_comment_editor() {
            let (_dir, herd) = temp_herd(&[task("t0", "solo", Status::Hairy, 3, None)]);
            let mut app = App::with_herd(herd).unwrap();
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
            let (_dir, herd) = temp_herd(&[task("t0", "solo", Status::Hairy, 3, None)]);
            let mut app = App::with_herd(herd).unwrap();
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
            let (_dir, herd) = temp_herd(&[task("t0", "solo", Status::Hairy, 3, None)]);
            let mut app = App::with_herd(herd).unwrap();
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
            let (_dir, herd) = temp_herd(&[task("t0", "solo", Status::Hairy, 3, None)]);
            let mut app = App::with_herd(herd).unwrap();
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
            let (_dir, herd) = temp_herd(&[task("t0", "solo", Status::Hairy, 3, None)]);
            let mut app = App::with_herd(herd).unwrap();
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
            let (_dir, herd) = temp_herd(&[task("t0", "solo", Status::Hairy, 3, None)]);
            let mut app = App::with_herd(herd).unwrap();
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
            let (_dir, herd) = temp_herd(&[task("t0", "solo", Status::Hairy, 3, None)]);
            let mut app = App::with_herd(herd).unwrap();
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
            let (_dir, herd) = temp_herd(&[task("t0", "solo", Status::Hairy, 3, None)]);
            let mut app = App::with_herd(herd).unwrap();
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
