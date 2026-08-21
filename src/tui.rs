//! Interactive terminal UI (`yaks tui`) — a thin layer over the core.
//!
//! All drawing goes through the pure `render(&App, &mut Frame)`, so the same
//! painter can target a real terminal (crossterm) or an in-memory `TestBackend`
//! buffer (snapshot tests + the future demo-cast pipeline). Key handling only
//! mutates `App`; mutating keys route through the `Herd` facade and then reload.

mod detail;
mod tree;

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
use ratatui::widgets::{Block, Borders, List, ListItem, ListState, Padding, Paragraph, Tabs};
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
}

/// What a resolved single-key pick should do (carries the target task id).
enum PickAction {
    State(String),
    Priority(String),
    Type(String),
    /// Second step of create: title already captured, now pick the type.
    CreateType {
        title: String,
        parent: Option<String>,
    },
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
    CreateTitle { parent: Option<String> },
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

/// FilterSpec isn't Clone; rebuild it field-by-field (used for the drawer's
/// revert snapshot).
fn clone_spec(f: &FilterSpec) -> FilterSpec {
    FilterSpec {
        statuses: f.statuses.clone(),
        types: f.types.clone(),
        priorities: f.priorities.clone(),
        labels: f.labels.clone(),
        search: f.search.clone(),
        ready_only: f.ready_only,
        tangled_only: f.tangled_only,
        parent: f.parent.clone(),
    }
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
    tabs: Vec<Status>,
    tab: usize,
    cursor: usize,
    focus: Focus,
    detail_scroll: u16,
    /// Index into the current detail's jumplist (Tab-cycled link targets).
    detail_link: usize,
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
        App {
            herd: None,
            all,
            tabs: vec![Status::Hairy, Status::Shaving, Status::Shorn],
            tab: 0,
            cursor: 0,
            focus: Focus::List,
            detail_scroll: 0,
            detail_link: 0,
            collapsed: HashSet::new(),
            filter: FilterSpec::default(),
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
        let mut app = App::new(all);
        app.editor_vim = vim;
        app.herd = Some(herd);
        Ok(app)
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

    fn tab_status(&self) -> Status {
        self.tabs[self.tab]
    }

    fn count(&self, s: Status) -> usize {
        self.all.iter().filter(|t| t.status == s).count()
    }

    /// Visible rows for the current tab (tree built + collapse applied).
    fn rows(&self) -> Vec<tree::Row<'_>> {
        let flat = tree::build(&self.all, self.tab_status(), &self.filter);
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

    fn switch_tab(&mut self, delta: i32) {
        let n = self.tabs.len() as i32;
        self.tab = (((self.tab as i32 + delta) % n + n) % n) as usize;
        self.cursor = 0;
        self.detail_scroll = 0;
        self.focus = Focus::List;
    }

    fn move_cursor(&mut self, delta: i32) {
        let len = self.rows().len() as i32;
        if len == 0 {
            self.cursor = 0;
            return;
        }
        self.cursor = (self.cursor as i32 + delta).clamp(0, len - 1) as usize;
    }

    fn toggle_collapse(&mut self) {
        let rows = self.rows();
        if let Some(row) = rows.get(self.cursor) {
            if row.has_children {
                let id = row.task.id.clone();
                if !self.collapsed.remove(&id) {
                    self.collapsed.insert(id);
                }
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
        let label = match &parent {
            Some(p) => format!("New child of {p} — title: "),
            None => "New yak — title: ".into(),
        };
        self.overlay = Overlay::Edit(Editor::new(
            self.editor_vim,
            true,
            label,
            "",
            EditAction::CreateTitle { parent },
        ));
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

    /// Select an existing task wherever it lives: switch to its status tab (if
    /// one is shown) and place the cursor on it, then focus the list.
    fn select_task(&mut self, id: &str) {
        if let Some(st) = self.task(id).map(|t| t.status) {
            if let Some(i) = self.tabs.iter().position(|&s| s == st) {
                self.tab = i;
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

    /// Jump to the Hairy tab (where new tasks land) and place the cursor on `id`.
    fn select_id(&mut self, id: &str) {
        if let Some(i) = self.tabs.iter().position(|&s| s == Status::Hairy) {
            self.tab = i;
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
            | Overlay::Drawer(_) => {}
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
            EditAction::CreateTitle { parent } => {
                let title = text.trim().to_string();
                if title.is_empty() {
                    self.notification = Some("create cancelled (empty title)".into());
                    return;
                }
                // Second step: pick the type, then actually create.
                self.overlay = Overlay::Pick {
                    prompt: "New yak type: t=task b=bug f=feature i=idea  (Esc=cancel)".into(),
                    keys: "tbfi".into(),
                    action: PickAction::CreateType { title, parent },
                };
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
            PickAction::CreateType { title, parent } => {
                let kind = match c {
                    't' => "task",
                    'b' => "bug",
                    'f' => "feature",
                    'i' => "idea",
                    _ => return,
                };
                let Some(h) = &self.herd else { return };
                let new = NewTask {
                    title,
                    kind: Some(kind.to_string()),
                    priority: None,
                    parent,
                    labels: vec![],
                    depends_on: vec![],
                    source: None,
                    description: None,
                };
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
            KeyCode::Esc => {
                if app.filter.content_active() {
                    app.filter = FilterSpec::default();
                    app.clamp_cursor();
                    app.notification = Some("filter cleared".into());
                }
            }
            KeyCode::Char('l') | KeyCode::Right | KeyCode::Enter => {
                if app.selected().is_some() {
                    app.focus = Focus::Detail;
                    app.detail_scroll = 0;
                    app.detail_link = 0;
                }
            }
            _ => {}
        },
        Focus::Detail => match k.code {
            KeyCode::Char('q') => app.quit = true,
            KeyCode::Char('h') | KeyCode::Left | KeyCode::Esc => app.focus = Focus::List,
            KeyCode::Tab | KeyCode::Char(']') => app.jump_link(1),
            KeyCode::BackTab | KeyCode::Char('[') => app.jump_link(-1),
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
    let [top, mid, bot] = Layout::vertical([
        Constraint::Length(1),
        Constraint::Min(0),
        Constraint::Length(1),
    ])
    .areas(frame.area());
    render_tabs(app, frame, top);
    let [left, right] =
        Layout::horizontal([Constraint::Percentage(45), Constraint::Percentage(55)]).areas(mid);
    render_list(app, frame, left);
    // The multi-line body editor takes over the detail pane; single-line fields
    // live on the status line, so the detail pane stays put for them.
    match &app.overlay {
        Overlay::Edit(ed) if !ed.single_line => render_editor_panel(ed, frame, right),
        Overlay::Fuzzy(fp) => render_fuzzy_results(app, fp, frame, right),
        Overlay::Drawer(d) => render_drawer(d, frame, right),
        _ => render_detail(app, frame, right),
    }
    render_status(app, frame, bot);
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
    render_chip_row(d, 0, "status", &statuses, frame, rows[1]);
    render_chip_row(d, 1, "type", &types, frame, rows[2]);
    render_chip_row(d, 2, "priority", &pris, frame, rows[3]);
    render_text_row(d, 3, "labels", &d.labels, frame, rows[4]);
    render_text_row(d, 4, "search", &d.search, frame, rows[5]);
    render_text_row(d, 5, "parent", &d.parent, frame, rows[6]);
    render_chip_row(d, 6, "deps", &deps, frame, rows[7]);
}

fn render_chip_row(
    d: &Drawer,
    row: usize,
    label: &str,
    choices: &[(String, bool)],
    frame: &mut Frame,
    area: Rect,
) {
    let current_row = d.row == row;
    let mut spans = vec![
        Span::styled(
            if current_row { "▸ " } else { "  " },
            Style::new().fg(Color::Cyan),
        ),
        Span::styled(format!("{label:<9}"), Style::new().fg(Color::DarkGray)),
    ];
    for (j, (disp, sel)) in choices.iter().enumerate() {
        let mut style = if *sel {
            Style::new().fg(Color::Green).add_modifier(Modifier::BOLD)
        } else {
            Style::new().fg(Color::DarkGray)
        };
        if current_row && d.chip_idx == j {
            style = style.add_modifier(Modifier::REVERSED);
        }
        spans.push(Span::raw(" "));
        spans.push(Span::styled(disp.clone(), style));
    }
    frame.render_widget(Paragraph::new(Line::from(spans)), area);
}

fn render_text_row(
    d: &Drawer,
    row: usize,
    label: &str,
    cell: &RefCell<EditorState>,
    frame: &mut Frame,
    area: Rect,
) {
    let current = d.row == row;
    let [g, lab, fld] = Layout::horizontal([
        Constraint::Length(2),
        Constraint::Length(9),
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
            "(any)".to_string()
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

fn status_name(s: Status) -> &'static str {
    match s {
        Status::Hairy => "Hairy",
        Status::Shaving => "Shaving",
        Status::Shorn => "Shorn",
        Status::Dead => "Dead",
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
    let titles: Vec<Line> = app
        .tabs
        .iter()
        .map(|&s| Line::from(format!("{} {}", status_name(s), app.count(s))))
        .collect();
    let tabs = Tabs::new(titles)
        .select(app.tab)
        .highlight_style(Style::new().fg(Color::Cyan).add_modifier(Modifier::BOLD))
        .divider(Span::styled("·", Style::new().fg(Color::DarkGray)));
    frame.render_widget(tabs, area);
}

fn render_list(app: &App, frame: &mut Frame, area: Rect) {
    let focused = app.focus == Focus::List;
    let rows = app.rows();
    let items: Vec<ListItem> = rows.iter().map(|r| list_item(r)).collect();
    let mut state = ListState::default();
    if !rows.is_empty() {
        state.select(Some(app.cursor.min(rows.len() - 1)));
    }
    let hl = if focused {
        Style::new().fg(Color::Black).bg(Color::Cyan)
    } else {
        Style::new().add_modifier(Modifier::REVERSED)
    };
    let list = List::new(items).highlight_style(hl);
    frame.render_stateful_widget(list, area, &mut state);
}

fn list_item<'a>(r: &tree::Row<'a>) -> ListItem<'a> {
    let indent = "  ".repeat(r.depth as usize);
    let chevron = if r.has_children {
        if r.collapsed { "▸ " } else { "▾ " }
    } else {
        "  "
    };
    let base = if r.ghost {
        Style::new().fg(Color::DarkGray)
    } else {
        Style::new()
    };
    let glyph = format!("[{}] ", r.task.status.glyph());
    let mut spans = vec![
        Span::styled(
            format!("{indent}{chevron}"),
            Style::new().fg(Color::DarkGray),
        ),
        Span::styled(
            glyph,
            if r.ghost {
                base
            } else {
                Style::new().fg(Color::DarkGray)
            },
        ),
        Span::styled(
            format!("p{} ", r.task.priority),
            Style::new().fg(Color::DarkGray),
        ),
        Span::styled(r.task.title.clone(), base),
    ];
    if r.collapsed && r.hidden > 0 {
        spans.push(Span::styled(
            format!("  (+{})", r.hidden),
            Style::new().fg(Color::DarkGray),
        ));
    }
    ListItem::new(Line::from(spans))
}

fn render_detail(app: &App, frame: &mut Frame, area: Rect) {
    let focused = app.focus == Focus::Detail;
    // Sparse chrome: a single left divider rule, no surrounding box.
    let block = Block::new()
        .borders(Borders::LEFT)
        .border_style(if focused {
            Style::new().fg(Color::Cyan)
        } else {
            Style::new().fg(Color::DarkGray)
        })
        .padding(Padding::horizontal(1));
    let Some(t) = app.selected() else {
        let p = Paragraph::new(Span::styled("(no task)", Style::new().fg(Color::DarkGray)))
            .block(block);
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
    let rendered: Vec<Line> = lines
        .iter()
        .enumerate()
        .map(|(i, dl)| render_dline(dl, cur, i))
        .collect();
    // No wrap: link highlight columns must stay valid.
    let p = Paragraph::new(rendered)
        .block(block)
        .scroll((app.detail_scroll, 0));
    frame.render_widget(p, area);
}

/// Render one detail line, styling link spans (and the focused jump target).
fn render_dline<'a>(
    dl: &'a detail::DLine,
    cur: Option<&detail::Jump>,
    line_idx: usize,
) -> Line<'a> {
    let base = match dl.kind {
        detail::Kind::Section => Style::new().fg(Color::Cyan).add_modifier(Modifier::BOLD),
        _ => Style::new(),
    };
    if dl.links.is_empty() {
        // Plain field: dim the fixed-width label, leave the value default.
        if dl.kind == detail::Kind::Field && dl.text.len() > 9 {
            let (label, value) = dl.text.split_at(9);
            return Line::from(vec![
                Span::styled(label.to_string(), Style::new().fg(Color::DarkGray)),
                Span::raw(value.to_string()),
            ]);
        }
        return Line::from(Span::styled(dl.text.clone(), base));
    }
    let chars: Vec<char> = dl.text.chars().collect();
    let mut links = dl.links.clone();
    links.sort_by_key(|(c, _, _)| *c);
    let mut spans: Vec<Span> = Vec::new();
    let mut pos = 0usize;
    for (col, len, _target) in links {
        let col = col.min(chars.len());
        let end = (col + len).min(chars.len());
        if col > pos {
            spans.push(Span::styled(
                chars[pos..col].iter().collect::<String>(),
                Style::new().fg(Color::DarkGray),
            ));
        }
        let is_current = cur.is_some_and(|j| j.line == line_idx && j.col == col);
        let style = if is_current {
            Style::new().fg(Color::Black).bg(Color::Cyan)
        } else {
            Style::new()
                .fg(Color::Blue)
                .add_modifier(Modifier::UNDERLINED)
        };
        spans.push(Span::styled(
            chars[col..end].iter().collect::<String>(),
            style,
        ));
        pos = end;
    }
    if pos < chars.len() {
        spans.push(Span::raw(chars[pos..].iter().collect::<String>()));
    }
    Line::from(spans)
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
    // Otherwise: an active modal prompt, else a transient notification, else the
    // context help hint. (A multi-line editor falls through to notification/help.)
    let (text, style) = match &app.overlay {
        Overlay::Pick { prompt, .. } | Overlay::Confirm { prompt, .. } => (
            prompt.clone(),
            Style::new()
                .fg(Color::Black)
                .bg(Color::Yellow)
                .add_modifier(Modifier::BOLD),
        ),
        _ => match &app.notification {
            Some(n) => (n.clone(), Style::new().fg(Color::Yellow)),
            None if app.filter.content_active() => (
                format!("filter: {}  (Esc clears)", filter_summary(&app.filter)),
                Style::new().fg(Color::Yellow),
            ),
            None => (help_hint(app).to_string(), Style::new().fg(Color::DarkGray)),
        },
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

fn help_hint(app: &App) -> &'static str {
    match app.focus {
        Focus::List => {
            "j/k move · c new · E edit · S/P/T/L/X · D/R link · / find · Tab tab · q quit"
        }
        Focus::Detail => "j/k scroll · h back · q quit",
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
    fn create_title_field() {
        let mut app = editable();
        handle_key(&mut app, key('c'));
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
        fn create_root_via_editor_then_type_pick() {
            let (_dir, herd) = temp_herd(&[]);
            let mut app = App::with_herd(herd).unwrap();
            press(&mut app, "c"); // open create title field
            press(&mut app, "foo"); // type the title
            enter(&mut app); // commit title -> type picker
            press(&mut app, "b"); // pick bug -> create
            assert!(matches!(app.overlay, Overlay::None));
            let created = app.all.iter().find(|t| t.title == "foo").expect("created");
            assert_eq!(created.kind, "bug");
            assert_eq!(created.status, Status::Hairy);
            // The cursor lands on the new task.
            assert_eq!(app.selected().map(|t| t.title.as_str()), Some("foo"));
        }

        #[test]
        fn create_child_sets_parent() {
            let (_dir, herd) = temp_herd(&[task("p0", "parent", Status::Hairy, 3, None)]);
            let mut app = App::with_herd(herd).unwrap();
            press(&mut app, "C"); // create child of the selected task
            press(&mut app, "kid");
            enter(&mut app);
            press(&mut app, "t");
            let created = app.all.iter().find(|t| t.title == "kid").expect("child");
            assert_eq!(created.parent.as_deref(), Some("p0"));
        }

        #[test]
        fn create_cancelled_with_esc() {
            let (_dir, herd) = temp_herd(&[]);
            let mut app = App::with_herd(herd).unwrap();
            press(&mut app, "c");
            press(&mut app, "foo");
            esc(&mut app); // single-line Esc cancels
            assert!(matches!(app.overlay, Overlay::None));
            assert!(app.all.is_empty());
            assert_eq!(app.notification.as_deref(), Some("cancelled"));
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
