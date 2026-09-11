//! The create/edit task form and its rendering. Split out of `tui.rs`
//! (yaks-b1cc / b1cc/3). Shared form helpers (`text_field`, `multiline_field`,
//! `kind_index`, `pri_index`, the `*_CHOICES` consts, `render_chip_row`/
//! `render_text_row`) stay in `tui.rs` and are reached via `super::`, as the
//! filter drawer uses them too; the editor render helpers live in `editor.rs`.

use std::cell::RefCell;

use edtui::{EditorEventHandler, EditorState, EditorView};
use ratatui::Frame;
use ratatui::layout::{Constraint, Layout, Rect};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Paragraph, Wrap};

use super::{
    CREATE_LABEL_W, HEADER_ROWS, PRI_CHOICES, TYPE_CHOICES, content, disp_width, editor_theme,
    kind_index, make_handler, multiline_field, pri_index, render_chip_row, render_text_row,
    set_md_highlights, text_field,
};
use crate::model::Task;

/// One editable content block in the form: the task's description, or a single
/// comment (carrying its original timestamp so it round-trips unchanged).
pub(crate) struct ContentBlock {
    pub(crate) kind: content::BlockKind,
    pub(crate) editor: RefCell<EditorState>,
}

/// The create/edit task form: a right-pane form modeled on `Drawer`. Two chip
/// rows (type/priority) are **single-select** — the cursor *is* the value —
/// plus single-line title/labels rows and a stack of multi-line **content**
/// blocks (description + comments). `Ctrl-N/P`/Tab walk every row; the focused
/// block expands to a live editor (accordion). `Ctrl-S` commits (create or
/// update), `Esc`/`Ctrl-C` cancels. Shared by `c`/`C` (create) and `E` (edit).
pub(crate) struct CreateForm {
    pub(crate) title: RefCell<EditorState>,
    pub(crate) labels: RefCell<EditorState>,
    /// `blocks[0]` is always the description; `blocks[1..]` are comments.
    pub(crate) blocks: Vec<ContentBlock>,
    /// Index into `TYPE_CHOICES` (single-select cursor==value).
    pub(crate) kind_idx: usize,
    /// Index into `PRI_CHOICES` (single-select cursor==value; default → p3).
    pub(crate) pri_idx: usize,
    pub(crate) row: usize,
    /// Create: the (optional) parent for the new task. Edit: unused (reparent
    /// is a separate action); shown in the header for context.
    pub(crate) parent: Option<String>,
    /// `Some(id)` when editing an existing task; `None` when creating.
    pub(crate) edit_id: Option<String>,
    pub(crate) handler: EditorEventHandler,
}

impl CreateForm {
    pub(crate) fn new(vim: bool, parent: Option<String>) -> Self {
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
    pub(crate) fn for_edit(vim: bool, task: &Task) -> Self {
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

    pub(crate) fn is_editing(&self) -> bool {
        self.edit_id.is_some()
    }

    pub(crate) fn row_count(&self) -> usize {
        HEADER_ROWS + self.blocks.len()
    }

    /// The content-block index for the current row, when the cursor is on one.
    pub(crate) fn content_index(&self) -> Option<usize> {
        self.row
            .checked_sub(HEADER_ROWS)
            .filter(|&i| i < self.blocks.len())
    }

    pub(crate) fn is_content_row(&self) -> bool {
        self.content_index().is_some()
    }

    /// Single-line text rows (title, labels); content blocks are multi-line and
    /// handled separately.
    pub(crate) fn is_line_text_row(&self) -> bool {
        matches!(self.row, 0 | 3)
    }

    /// The editor backing the current single-line text row (title/labels).
    pub(crate) fn line_editor(&self) -> Option<&RefCell<EditorState>> {
        match self.row {
            0 => Some(&self.title),
            3 => Some(&self.labels),
            _ => None,
        }
    }

    /// Move the single-select chip cursor on a chip row (wrapping).
    pub(crate) fn move_chip(&mut self, delta: i32) {
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

    pub(crate) fn title_text(&self) -> String {
        self.title.borrow().lines.to_string().trim().to_string()
    }

    pub(crate) fn labels_vec(&self) -> Vec<String> {
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
    pub(crate) fn assembled_body(&self) -> String {
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
    pub(crate) fn body_opt(&self) -> Option<String> {
        let b = self.assembled_body();
        if b.trim().is_empty() { None } else { Some(b) }
    }
}

pub(crate) fn render_create(f: &CreateForm, frame: &mut Frame, area: Rect) {
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
pub(crate) fn render_content_stack(f: &CreateForm, frame: &mut Frame, area: Rect) {
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
pub(crate) fn render_block_separator(
    block: &ContentBlock,
    focused: bool,
    frame: &mut Frame,
    area: Rect,
) {
    let label = match &block.kind {
        content::BlockKind::Description => "description".to_string(),
        content::BlockKind::Comment { timestamp, actor } => {
            let date = timestamp.get(..10).unwrap_or(timestamp);
            match actor {
                Some(a) => format!("comment · {date} · {a}"),
                None => format!("comment · {date}"),
            }
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
