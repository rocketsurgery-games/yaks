//! Interactive terminal UI (`yaks tui`) — a thin layer over the core.
//!
//! All drawing goes through the pure `render(&App, &mut Frame)`, so the same
//! painter can target a real terminal (crossterm) or an in-memory `TestBackend`
//! buffer (snapshot tests + the future demo-cast pipeline). Key handling only
//! mutates `App`; mutating keys route through the `Farm` facade and then reload.

mod actions;
mod cache;
mod content;
mod create;
mod detail;
mod detail_nav;
#[cfg(test)]
mod docshots;
mod drawer;
mod editor;
mod fuzzy;
mod handlers;
mod headless;
mod markdown;
mod render;
mod runtime;
#[cfg(test)]
mod test_support;
#[cfg(test)]
mod tests;
mod tree;
mod view;
mod viewmodel;
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

use edtui::{EditorMode, EditorState, Lines};
use notify::event::{EventKind, ModifyKind};
use notify::{RecommendedWatcher, RecursiveMode, Watcher};
use ratatui::Terminal;
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

use crate::farm::{
    AttachOutcome, CreateOutcome, DepOutcome, Farm, MoveOutcome, NewTask, RenameOutcome, Reparent,
    TaskEdit, UpdateOutcome,
};
use crate::filter::{self, FilterSpec};
use crate::model::{Status, Task};

use create::*;
use drawer::*;
use editor::*;
use fuzzy::*;
use handlers::handle_key;
use render::*;
pub use runtime::run;

/// One o/i nav-history stop: a task plus the detail cursor/scroll it was left
/// at, so back/forward return you to where you were (yaks-28b4). Deliberately
/// local to detail-link nav; a global cross-view history is yaks-158d.
#[derive(Clone, PartialEq, Debug)]
struct NavEntry {
    id: String,
    line: usize,
    scroll: u16,
}

#[derive(Clone, Copy, PartialEq, Debug)]
enum Focus {
    List,
    Detail,
}

/// A modal prompt painted on the bottom line: `pick()` (single keypress) and
/// `confirm()` (y/N) dialogs. Kept as plain
/// data so `render` stays pure — the action to perform on commit rides along.
pub(crate) enum Overlay {
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
pub(crate) enum PickAction {
    State(String),
    Priority(String),
    Type(String),
    /// Apply a state transition to every id in the multi-select set (yaks-de85).
    BulkState(Vec<String>),
    /// Move a yak to another herd (id prefix): `(id, candidate herds)`. The
    /// picked digit indexes into the candidates; the rename keeps the id tail
    /// (yaks-71d1).
    Herd(String, Vec<String>),
}

/// What a confirmed y/N prompt should do.
pub(crate) enum ConfirmAction {
    Slaughter(String),
    /// Discard a dirty edit/create form or comment (stashed in `App.dirty_cancel`).
    DiscardEdit,
}

// Filter-drawer layout: 7 rows, three of them free-text.
const DRAWER_ROWS: usize = 7;
const STATUS_CHOICES: [Status; 4] = [Status::Hairy, Status::Shaving, Status::Shorn, Status::Dead];
const TYPE_CHOICES: [&str; 4] = ["task", "bug", "feature", "idea"];
const PRI_CHOICES: [u8; 5] = [1, 2, 3, 4, 5];
const DEPS_CHOICES: [&str; 3] = ["ready", "tangled", "inbox"];

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

/// Max gap between two `Esc` presses for them to count as a double-tap cancel.
const DOUBLE_ESC_MS: u64 = 300;

// Task-form fixed rows: title, type, priority, labels. Content blocks (a
// description, plus one per comment when editing) follow as rows `HEADER_ROWS +
// block_index`, so the total row count is dynamic (`CreateForm::row_count`).
const HEADER_ROWS: usize = 4;

fn kind_index(kind: &str) -> usize {
    TYPE_CHOICES.iter().position(|&k| k == kind).unwrap_or(0)
}

fn pri_index(p: u8) -> usize {
    PRI_CHOICES.iter().position(|&x| x == p).unwrap_or(2)
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

/// TUI state. Holds the loaded task set plus (in live use) a `Farm` handle so
/// mutations re-query through the core. Per-tab tree views are derived on demand.
pub struct App {
    /// `None` in read-only snapshot tests; `Some` in live use (`with_farm`).
    farm: Option<Farm>,
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
    /// Browser-style navigation history of visited tasks (o = back, i =
    /// forward), driven by following detail links. Each entry remembers the
    /// detail cursor + scroll it was left at, restored on return (yaks-28b4).
    nav_back: Vec<NavEntry>,
    nav_fwd: Vec<NavEntry>,
    collapsed: HashSet<String>,
    /// Per-view family-scope overrides, keyed by `View::key` (persisted in the
    /// UI-state cache). A missing entry means the view inherits the global
    /// default ("auto"). See [`view::FamilyScope`].
    family_scope: HashMap<String, view::FamilyScope>,
    /// The live view filter applied by the tree (re-colors + prunes). The inbox
    /// (yaks awaiting a human) is just `FilterSpec::needs_only` on this filter
    /// — surfaced via the Inbox view and the drawer's `inbox` chip, not a mode.
    filter: FilterSpec,
    /// Approx. list viewport height, refreshed each loop for paging math.
    page: u16,
    /// Approx. detail viewport height (mid area = terminal height - 3),
    /// refreshed each loop; used to keep the active link scrolled into view.
    detail_page: u16,
    /// Detail-pane content width captured at render, used to soft-wrap the
    /// detail lines so the row-indexed model matches what's on screen.
    detail_width: Cell<u16>,
    /// The list/tree viewport's scroll offset (first visible row), carried
    /// across frames so moving the cursor scrolls only as far as needed to keep
    /// it visible, rather than re-deriving it from 0 each frame (which pinned
    /// the selection to the bottom edge when scrolling up — yaks-9009).
    list_offset: Cell<usize>,
    overlay: Overlay,
    /// Transient one-line status message shown until the next mutation.
    notification: Option<String>,
    /// Editor keybinding profile (vim vs emacs), from farm config.
    editor_vim: bool,
    /// The farm's configured id prefix (e.g. `yaks`), for autocomplete: typing
    /// `<ref_prefix>-` while editing opens the yak-ref picker.
    ref_prefix: String,
    /// The farm's declared herd set (`Config::known_herds`); unioned with the
    /// prefixes actually present to form the create-form herd picker's choices.
    config_herds: Vec<String>,
    /// Timestamp of the last `Esc` in an editor overlay, for detecting a rapid
    /// double-`Esc` (a Ctrl-C-equivalent cancel gesture). See [`App::register_double_esc`].
    last_esc: Option<std::time::Instant>,
    /// A dirty edit/create/comment overlay stashed behind a "discard changes?"
    /// confirmation; restored if the user declines. See [`App::request_cancel`].
    dirty_cancel: Option<Overlay>,
    /// The in-progress `:` command line (vim `:w`/`:q`/...), layered over the
    /// active editor overlay while `Some`. See [`App::handle_cmdline_key`].
    cmdline: Option<String>,
    /// Ids marked for multi-select bulk actions (`m` toggles the cursor's yak).
    /// A bulk state transition (`S` with a non-empty set) loops these ids
    /// through `farm.transition`, then clears the set. See [`App::toggle_selected`].
    selected: HashSet<String>,
    quit: bool,
}

impl App {
    /// Read-only constructor: renders `all` with no farm behind it. Mutating
    /// keys become no-ops. Used by snapshot tests and any preview caller.
    pub fn new(all: Vec<Task>) -> Self {
        let views = view::default_views();
        let filter = clone_spec(&views[0].spec);
        App {
            farm: None,
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
            family_scope: HashMap::new(),
            filter,
            page: 10,
            detail_page: 10,
            detail_width: Cell::new(0),
            list_offset: Cell::new(0),
            overlay: Overlay::None,
            notification: None,
            editor_vim: true,
            ref_prefix: "yak".to_string(),
            config_herds: Vec::new(),
            last_esc: None,
            dirty_cancel: None,
            cmdline: None,
            selected: HashSet::new(),
            quit: false,
        }
    }

    /// Live constructor: loads the current farm view and keeps the handle so
    /// mutations can re-query after each change.
    pub fn with_farm(farm: Farm) -> Result<Self> {
        // Load every status incl. dead. The tree/flat views scope down to what
        // each shows, but keeping dead in the model lets the ancestor walk root
        // a live yak beneath a slaughtered parent, lets a Dead filter surface
        // slaughtered yaks, and treats a dep on a dead yak as resolved (fe00).
        let all = farm.list(FilterSpec::default(), true)?;
        let cfg = farm.config();
        let vim = cfg.vim_mode;
        let ui = cache::load(farm.root());
        let views = views_store::load_views(farm.root());
        let working_set = views_store::load_working_set(farm.root());
        let mut app = App::new(all);
        app.editor_vim = vim;
        app.config_herds = cfg.known_herds();
        app.ref_prefix = cfg.prefix;
        app.collapsed = ui.collapsed;
        app.family_scope = ui.family;
        app.filter = clone_spec(&views[0].spec);
        app.views = views;
        app.working_set = working_set;
        app.farm = Some(farm);
        app.clamp_cursor();
        Ok(app)
    }
}
