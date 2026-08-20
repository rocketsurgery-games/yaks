//! Interactive terminal UI (`yaks tui`) — a thin layer over the core.
//!
//! All drawing goes through the pure `render(&App, &mut Frame)`, so the same
//! painter can target a real terminal (crossterm) or an in-memory `TestBackend`
//! buffer (snapshot tests + the future demo-cast pipeline). Key handling only
//! mutates `App`; later slices route mutating keys through `herd::Herd`.

mod tree;

use std::collections::HashSet;
use std::io::{self, Stdout};

use anyhow::Result;
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
use ratatui::widgets::{Block, Borders, List, ListItem, ListState, Padding, Paragraph, Tabs, Wrap};
use ratatui::{Frame, Terminal};

use crate::model::{Status, Task};

#[derive(Clone, Copy, PartialEq)]
enum Focus {
    List,
    Detail,
}

/// TUI state. Holds the loaded task set; per-tab tree views are derived on
/// demand. (Read-only in slices 1-2; later slices add a `Herd` handle.)
pub struct App {
    all: Vec<Task>,
    tabs: Vec<Status>,
    tab: usize,
    cursor: usize,
    focus: Focus,
    detail_scroll: u16,
    collapsed: HashSet<String>,
    /// Approx. list viewport height, refreshed each loop for paging math.
    page: u16,
    quit: bool,
}

impl App {
    pub fn new(all: Vec<Task>) -> Self {
        App {
            all,
            tabs: vec![Status::Hairy, Status::Shaving, Status::Shorn],
            tab: 0,
            cursor: 0,
            focus: Focus::List,
            detail_scroll: 0,
            collapsed: HashSet::new(),
            page: 10,
            quit: false,
        }
    }

    fn tab_status(&self) -> Status {
        self.tabs[self.tab]
    }

    fn count(&self, s: Status) -> usize {
        self.all.iter().filter(|t| t.status == s).count()
    }

    /// Visible rows for the current tab (tree built + collapse applied).
    fn rows(&self) -> Vec<tree::Row<'_>> {
        let flat = tree::build(&self.all, self.tab_status());
        tree::apply_collapse(flat, &self.collapsed)
    }

    fn selected(&self) -> Option<&Task> {
        self.rows().into_iter().nth(self.cursor).map(|r| r.task)
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
            KeyCode::Char('l') | KeyCode::Right | KeyCode::Enter => {
                if app.selected().is_some() {
                    app.focus = Focus::Detail;
                    app.detail_scroll = 0;
                }
            }
            _ => {}
        },
        Focus::Detail => match k.code {
            KeyCode::Char('q') => app.quit = true,
            KeyCode::Char('h') | KeyCode::Left | KeyCode::Esc => app.focus = Focus::List,
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
    render_detail(app, frame, right);
    render_help(app, frame, bot);
}

fn status_name(s: Status) -> &'static str {
    match s {
        Status::Hairy => "Hairy",
        Status::Shaving => "Shaving",
        Status::Shorn => "Shorn",
        Status::Dead => "Dead",
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
    let text = match app.selected() {
        None => vec![Line::from(Span::styled(
            "(no task)",
            Style::new().fg(Color::DarkGray),
        ))],
        Some(t) => detail_lines(t),
    };
    let p = Paragraph::new(text)
        .block(block)
        .wrap(Wrap { trim: false })
        .scroll((app.detail_scroll, 0));
    frame.render_widget(p, area);
}

fn detail_lines(t: &Task) -> Vec<Line<'static>> {
    let mut out = vec![
        field("id", &t.id),
        field("title", &t.title),
        field("type", &t.kind),
        field("priority", &t.priority.to_string()),
    ];
    if let Some(p) = &t.parent {
        out.push(field("parent", p));
    }
    if !t.labels.is_empty() {
        out.push(field("labels", &t.labels.join(", ")));
    }
    if !t.depends_on.is_empty() {
        out.push(field("depends", &t.depends_on.join(", ")));
    }
    if let Some(s) = &t.source {
        out.push(field("source", s));
    }
    let body = t.body.trim();
    if !body.is_empty() {
        out.push(Line::from(""));
        for l in body.lines() {
            out.push(Line::from(l.to_string()));
        }
    }
    out
}

fn field(k: &str, v: &str) -> Line<'static> {
    Line::from(vec![
        Span::styled(format!("{k:<9}"), Style::new().fg(Color::DarkGray)),
        Span::raw(v.to_string()),
    ])
}

fn render_help(app: &App, frame: &mut Frame, area: Rect) {
    let hint = match app.focus {
        Focus::List => "j/k move · Space fold · Tab tab · l detail · q quit",
        Focus::Detail => "j/k scroll · h back · q quit",
    };
    frame.render_widget(
        Paragraph::new(Span::styled(hint, Style::new().fg(Color::DarkGray))),
        area,
    );
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
}
