//! The render layer: paints `App` state into ratatui widgets. Split out of
//! `tui.rs` (yaks-b1cc / b1cc/5). Overlay dispatch here calls into the
//! per-module renderers (drawer/create/fuzzy/editor); shared render helpers
//! (`disp_width`, `status_word`, `priority_style`, `render_chip_row`/
//! `render_text_row`) live here and are re-exported by `tui.rs`, so sibling
//! modules reach them via `super::`. The whole-frame snapshot/integration tests
//! stay central in `tui.rs` (yaks-2301 decision (a)).

use ratatui::Frame;
use ratatui::layout::{Constraint, Layout, Rect};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, List, ListItem, ListState, Padding, Paragraph};

use std::cell::RefCell;

use edtui::{EditorState, EditorView};

use super::{
    App, Focus, Overlay, completion_context, detail, editor_theme, render_create, render_drawer,
    render_editor_panel, render_fuzzy_results, render_line_editor, render_query_line, tree, view,
};
use crate::filter::FilterSpec;
use crate::model::{Status, Task};

pub(crate) fn render(app: &App, frame: &mut Frame) {
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
pub(crate) fn right_divider(frame: &mut Frame, area: Rect, focused: bool) -> Rect {
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

pub(crate) fn render_view_picker(app: &App, sel: usize, frame: &mut Frame, area: Rect) {
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
pub(crate) fn help_content() -> Vec<Line<'static>> {
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
        entry("m", "Mark / unmark row (multi-select)"),
        entry("S (marked)", "Bulk state over the marked set"),
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
        entry("a", "Ask / answer (raise / clear needs block)"),
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

pub(crate) fn render_help(scroll: u16, frame: &mut Frame, area: Rect) {
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

/// The create/edit task form: header + title / type / priority / labels meta
/// rows, a `─ description ─` separator, then a multi-line description content
/// zone filling the rest. Laid out like `render_drawer`, inset by `right_divider`.

/// Label-column widths (gutter marker excluded): the drawer's longest label is
/// `priority` (8); the create form's is `description` (11). Each leaves one
/// trailing space before the chips/field.
pub(crate) const DRAWER_LABEL_W: usize = 9;
pub(crate) const CREATE_LABEL_W: usize = 12;

/// Render one chip row: a `▸` gutter, a padded label, then space-separated
/// chips. `current_row` highlights this row; `cursor_idx` is the chip the
/// cursor sits on (drawn REVERSED); each choice's bool marks it selected
/// (green bold). For single-select forms the cursor and the selection coincide.
pub(crate) fn render_chip_row(
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
pub(crate) fn render_text_row(
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

pub(crate) fn overlay_name(o: &Overlay) -> &'static str {
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
pub(crate) fn format_count(n: usize) -> String {
    if n <= 999 {
        n.to_string()
    } else {
        "999+".into()
    }
}

/// Approximate display width: emoji (and other astral glyphs) are width 2.
pub(crate) fn disp_width(s: &str) -> usize {
    s.chars()
        .map(|c| {
            let cp = c as u32;
            if cp >= 0x1_F000 || cp == 0x2b50 || cp == 0x2764 || cp == 0x23f3 {
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
pub(crate) fn priority_style(p: u8) -> Style {
    match p {
        1 => Style::new().fg(Color::Red).add_modifier(Modifier::BOLD),
        2 => Style::new().fg(Color::Magenta),
        4 => Style::new().fg(Color::Green),
        5 => Style::new().fg(Color::Blue).add_modifier(Modifier::DIM),
        _ => Style::new().fg(Color::Yellow), // 3 (and any fallback)
    }
}

pub(crate) fn status_word(s: Status) -> &'static str {
    match s {
        Status::Hairy => "hairy",
        Status::Shaving => "shaving",
        Status::Shorn => "shorn",
        Status::Dead => "dead",
    }
}

pub(crate) fn render_tabs(app: &App, frame: &mut Frame, area: Rect) {
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
    // an explicit per-view override.
    if let Some(n) = &app.notification {
        // The ephemeral status message wants its usual right-aligned slot on the
        // tab row, but it must not scribble over the tabs. When it fits beside
        // them (the last tab already contributes a trailing space to `tabs_w`),
        // keep it inline; otherwise drop it to the blank gap row directly below
        // the tab strip (`area.y + 1`, guaranteed by the top-of-frame layout in
        // `render`), left-aligned on that otherwise-empty line. Pure render-time
        // geometry, so it re-evaluates on every resize.
        let style = Style::new().fg(Color::Yellow).add_modifier(Modifier::BOLD);
        let full = disp_width(n);
        let w = (full as u16).min(area.width);
        if w > 0 {
            let rect = if tabs_w + full <= area.width as usize {
                Rect {
                    x: area.x + area.width - w,
                    y: area.y,
                    width: w,
                    height: 1,
                }
            } else {
                Rect {
                    x: area.x,
                    y: area.y + 1,
                    width: w,
                    height: 1,
                }
            };
            frame.render_widget(Paragraph::new(Span::styled(n.clone(), style)), rect);
        }
    } else {
        let av = app.active_view();
        if !(av.is_flat() || av.key == "working-set") {
            let (marker, val) = match app.herd_scope.get(&av.key) {
                Some(s) => ("", s.as_str()),
                None => ("~", view::HerdScope::DEFAULT.as_str()),
            };
            let label = format!("herd: {marker}{val}");
            // The indicator yields rather than overwrite tabs when the strip is
            // too wide to fit it.
            if tabs_w + disp_width(&label) < area.width as usize {
                let w = (disp_width(&label) as u16).min(area.width);
                if w > 0 {
                    let rect = Rect {
                        x: area.x + area.width - w,
                        y: area.y,
                        width: w,
                        height: 1,
                    };
                    frame.render_widget(
                        Paragraph::new(Span::styled(label, Style::new().fg(Color::DarkGray))),
                        rect,
                    );
                }
            }
        }
    }
}

pub(crate) fn render_list(app: &App, frame: &mut Frame, area: Rect) {
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
                app.selected.contains(&r.task.id),
                r.task.needs.is_some(),
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
pub(crate) fn truncate_disp(s: &str, max: usize) -> String {
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

pub(crate) fn list_item<'a>(
    r: &tree::Row<'a>,
    id_field_w: usize,
    blocked: bool,
    selected: bool,
    needs_blocked: bool,
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
    // Multi-select gutter: a marked row shows a filled dot, taking precedence
    // over the blocked `*` (selection is the active, user-driven concern; the
    // `*` returns once the row is unmarked).
    let lead = if selected {
        "\u{25cf}"
    } else if blocked {
        "*"
    } else {
        " "
    };
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
    // Needs badge (⏳): this yak is awaiting a human (a `needs` block). A
    // distinct axis from the dependency-blocked `*` lead, so it can co-occur.
    if needs_blocked {
        if !right_plain.is_empty() {
            right_plain.push(' ');
            right_spans.push(Span::raw(" "));
        }
        right_plain.push('\u{23f3}');
        right_spans.push(Span::styled(
            "\u{23f3}",
            dim(Style::new().fg(Color::Yellow).add_modifier(Modifier::BOLD)),
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
            if selected {
                Style::new().fg(Color::Green).add_modifier(Modifier::BOLD)
            } else if blocked {
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

/// A one-line sticky summary of the task — status emoji, id, type, priority,
/// title, star — pinned above the detail content when it has scrolled past the
/// frontmatter, so you never lose track of which yak you're reading. Filled with
/// a faint background band so it reads as a header rather than a content row.
pub(crate) fn pinned_header_line(task: &Task, starred: bool, width: u16) -> Line<'static> {
    let bg = Color::Indexed(236);
    let w = width as usize;
    let emoji = task.status.emoji(); // display width 2
    let type_s = format!("{:8}", task.kind);
    let pri_s = format!("p{}", task.priority);
    let star = if starred { " \u{2b50}" } else { "" };
    let star_w = disp_width(star);

    // Left cluster: " 🪒 yaks-b517  feature  p2  "
    let left_plain = format!(" {emoji} {}  {type_s} {pri_s}  ", task.id);
    let left_w = disp_width(&left_plain);
    let title_avail = w.saturating_sub(left_w + star_w);
    let title = truncate_disp(&task.title, title_avail);
    let pad = w.saturating_sub(left_w + disp_width(&title) + star_w);

    let band = Style::new().bg(bg);
    let mut spans = vec![
        Span::styled(format!(" {emoji} "), band),
        Span::styled(task.id.clone(), band.fg(Color::Blue)),
        Span::styled("  ".to_string(), band),
        Span::styled(type_s, band.fg(Color::Cyan)),
        Span::styled(" ".to_string(), band),
        Span::styled(pri_s, priority_style(task.priority).bg(bg)),
        Span::styled("  ".to_string(), band),
        Span::styled(title, band.add_modifier(Modifier::BOLD)),
    ];
    if pad > 0 {
        spans.push(Span::styled(" ".repeat(pad), band));
    }
    if starred {
        spans.push(Span::styled(star.to_string(), band));
    }
    Line::from(spans)
}

pub(crate) fn render_detail(app: &App, frame: &mut Frame, area: Rect) {
    let focused = app.focus == Focus::Detail;
    // The left divider is drawn by render() via right_divider(); we render the
    // content into the already-inset area.
    let Some(task) = app.selected() else {
        let p = Paragraph::new(Span::styled("(no task)", Style::new().fg(Color::DarkGray)));
        frame.render_widget(p, area);
        return;
    };
    // Capture the content width so the row-indexed model (and event handlers)
    // wrap to exactly what's rendered here.
    app.detail_width.set(area.width);
    // When scrolled past the frontmatter, reserve the top row for a sticky
    // header and push the content down one row (nothing is hidden — the pin adds
    // a row rather than overlaying one).
    let pinned = app.detail_scroll > 0 && area.height > 1;
    let content_area = if pinned {
        Rect {
            x: area.x,
            y: area.y + 1,
            width: area.width,
            height: area.height - 1,
        }
    } else {
        area
    };
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
    frame.render_widget(p, content_area);
    if pinned {
        let header = pinned_header_line(task, app.is_starred(&task.id), area.width);
        frame.render_widget(Paragraph::new(header), Rect { height: 1, ..area });
    }
}

/// Render one detail line by computing a per-char style (base -> link ->
/// find-match, each overriding the last) and coalescing equal runs into spans.
pub(crate) fn render_dline<'a>(
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
            } else if dl.kind == detail::Kind::Warn {
                // Warning accent (the `needs` blocker): the whole line, label
                // included, reads yellow + bold so it stands out as a blocker.
                Style::new().fg(Color::Yellow).add_modifier(Modifier::BOLD)
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
pub(crate) fn dedent(lines: &[String]) -> Vec<String> {
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
pub(crate) fn detail_scan(lines: &[detail::DLine], q: &str) -> Vec<(usize, usize, usize)> {
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

pub(crate) fn render_status(app: &App, frame: &mut Frame, area: Rect) {
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
    // While editing, a `<prefix>-…` ref context surfaces a Tab-to-complete hint
    // on the help bar (opt-in: nothing pops up until you press Tab).
    let ref_context = match &app.overlay {
        Overlay::Edit(ed) if !ed.single_line => {
            completion_context(&ed.state, &app.ref_prefix).is_some()
        }
        Overlay::Create(f) => matches!(f.content_index(),
            Some(i) if completion_context(&f.blocks[i].editor, &app.ref_prefix).is_some()),
        _ => false,
    };
    if ref_context {
        frame.render_widget(
            Paragraph::new(Span::styled(
                "Tab → complete yak · Esc dismiss",
                Style::new().fg(Color::Cyan).add_modifier(Modifier::BOLD),
            )),
            area,
        );
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
    // Drawer help hint. In vim a lone Esc on a text row drops it to Normal, so
    // cancel is a double-Esc or Ctrl-C; emacs cancels on a plain Esc.
    if matches!(&app.overlay, Overlay::Drawer(_)) {
        let cancel = if app.editor_vim {
            "EscEsc/Ctrl-C cancel"
        } else {
            "Esc cancel"
        };
        frame.render_widget(
            Paragraph::new(Span::styled(
                format!("↑↓/Tab rows · ←→ chips · Space toggle · C clear · Enter apply · {cancel}"),
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
pub(crate) fn filter_summary(f: &FilterSpec) -> String {
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
    if f.needs_only {
        parts.push("inbox".into());
    }
    if parts.is_empty() {
        "(all)".into()
    } else {
        parts.join(" · ")
    }
}

pub(crate) fn help_hint(app: &App) -> String {
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
