//! Interactive terminal UI (`yaks tui`) — a thin layer over the core.
//!
//! All drawing goes through the pure `render(&App, &mut Frame)`, so the same
//! painter can target a real terminal (crossterm) or an in-memory `TestBackend`
//! buffer (snapshot tests + the future demo-cast pipeline). Key handling only
//! mutates `App`; later slices route mutating keys through `herd::Herd`.

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
use ratatui::widgets::{Block, List, ListItem, ListState, Paragraph, Tabs, Wrap};
use ratatui::{Frame, Terminal};

use crate::model::{Status, Task};

#[derive(Clone, Copy, PartialEq)]
enum Focus {
    List,
    Detail,
}

/// TUI state. Holds the loaded task set; per-tab views are derived on demand.
/// (Read-only in slice 1; later slices add a `Herd` handle for re-query.)
pub struct App {
    all: Vec<Task>,
    tabs: Vec<Status>,
    tab: usize,
    cursor: usize,
    focus: Focus,
    detail_scroll: u16,
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
            quit: false,
        }
    }

    fn tab_status(&self) -> Status {
        self.tabs[self.tab]
    }

    fn current(&self) -> Vec<&Task> {
        let s = self.tab_status();
        let mut v: Vec<&Task> = self.all.iter().filter(|t| t.status == s).collect();
        v.sort_by(|a, b| a.id.cmp(&b.id));
        v
    }

    fn count(&self, s: Status) -> usize {
        self.all.iter().filter(|t| t.status == s).count()
    }

    fn selected(&self) -> Option<&Task> {
        self.current().into_iter().nth(self.cursor)
    }

    fn switch_tab(&mut self, delta: i32) {
        let n = self.tabs.len() as i32;
        self.tab = (((self.tab as i32 + delta) % n + n) % n) as usize;
        self.cursor = 0;
        self.detail_scroll = 0;
        self.focus = Focus::List;
    }

    fn move_cursor(&mut self, delta: i32) {
        let len = self.current().len() as i32;
        if len == 0 {
            self.cursor = 0;
            return;
        }
        self.cursor = (self.cursor as i32 + delta).clamp(0, len - 1) as usize;
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
    if k.code == KeyCode::Char('c') && k.modifiers.contains(KeyModifiers::CONTROL) {
        app.quit = true;
        return;
    }
    match app.focus {
        Focus::List => match k.code {
            KeyCode::Char('q') => app.quit = true,
            KeyCode::Char('j') | KeyCode::Down => app.move_cursor(1),
            KeyCode::Char('k') | KeyCode::Up => app.move_cursor(-1),
            KeyCode::Char('g') => app.cursor = 0,
            KeyCode::Char('G') => {
                let n = app.current().len();
                app.cursor = n.saturating_sub(1);
            }
            KeyCode::Tab | KeyCode::Char(']') => app.switch_tab(1),
            KeyCode::BackTab | KeyCode::Char('[') => app.switch_tab(-1),
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
        Layout::horizontal([Constraint::Percentage(42), Constraint::Percentage(58)]).areas(mid);
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
        .highlight_style(
            Style::new()
                .fg(Color::Black)
                .bg(Color::Cyan)
                .add_modifier(Modifier::BOLD),
        )
        .divider("  ");
    frame.render_widget(tabs, area);
}

fn render_list(app: &App, frame: &mut Frame, area: Rect) {
    let focused = app.focus == Focus::List;
    let block = Block::bordered()
        .title(" yaks ")
        .border_style(border_style(focused));
    let rows = app.current();
    let items: Vec<ListItem> = rows
        .iter()
        .map(|t| {
            ListItem::new(Line::from(vec![
                Span::styled(
                    format!("p{} ", t.priority),
                    Style::new().fg(Color::DarkGray),
                ),
                Span::raw(t.title.clone()),
            ]))
        })
        .collect();
    let mut state = ListState::default();
    if !rows.is_empty() {
        state.select(Some(app.cursor.min(rows.len() - 1)));
    }
    let hl = if focused {
        Style::new().fg(Color::Black).bg(Color::Cyan)
    } else {
        Style::new().add_modifier(Modifier::REVERSED)
    };
    let list = List::new(items)
        .block(block)
        .highlight_style(hl)
        .highlight_symbol("› ");
    frame.render_stateful_widget(list, area, &mut state);
}

fn render_detail(app: &App, frame: &mut Frame, area: Rect) {
    let focused = app.focus == Focus::Detail;
    let block = Block::bordered()
        .title(" detail ")
        .border_style(border_style(focused));
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
        Focus::List => "j/k move · Tab tab · l detail · q quit",
        Focus::Detail => "j/k scroll · h back · q quit",
    };
    frame.render_widget(
        Paragraph::new(Span::styled(hint, Style::new().fg(Color::DarkGray))),
        area,
    );
}

fn border_style(focused: bool) -> Style {
    if focused {
        Style::new().fg(Color::Cyan)
    } else {
        Style::new().fg(Color::DarkGray)
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

    fn task(id: &str, title: &str, status: Status, priority: u8) -> Task {
        Task {
            id: id.into(),
            title: title.into(),
            kind: "task".into(),
            priority,
            status,
            created: None,
            updated: None,
            parent: None,
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
        App::new(vec![
            task("fix-0001", "Alpha task", Status::Hairy, 2),
            task("fix-0002", "Beta task", Status::Hairy, 1),
            task("fix-0003", "Gamma in progress", Status::Shaving, 3),
            task("fix-0004", "Delta done", Status::Shorn, 3),
        ])
    }

    #[test]
    fn board_list_focus() {
        insta::assert_snapshot!(draw(&sample(), 72, 14));
    }

    #[test]
    fn detail_focus_second_tab() {
        let mut app = sample();
        app.switch_tab(1); // Shaving
        app.focus = Focus::Detail;
        insta::assert_snapshot!(draw(&app, 72, 14));
    }
}
