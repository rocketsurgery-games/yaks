//! `App`'s detail-pane navigation: building the detail line model, cursor/scroll
//! movement, visual selection + yank, find, link-following, and browser-style
//! nav history. Split out of `tui.rs` (yaks-b1cc / b1cc/6) as an `impl App`
//! block. `use super::*` inherits `tui.rs`'s scope; methods are `pub(crate)`.

use super::*;

impl App {
    /// Select an existing task wherever it lives: switch to its status view (if
    /// one is pinned) and place the cursor on it, then focus the list.
    pub(crate) fn select_task(&mut self, id: &str) {
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
    pub(crate) fn expand_ancestors(&mut self, id: &str) {
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
    pub(crate) fn detail_dlines(&self) -> Vec<detail::DLine> {
        match self.selected() {
            Some(t) => detail::wrap(
                detail::build(t, &self.all),
                self.detail_width.get() as usize,
            ),
            None => Vec::new(),
        }
    }

    /// Detail jumplist for the current selection (empty when nothing selected).
    pub(crate) fn detail_jumps(&self) -> Vec<detail::Jump> {
        detail::jumplist(&self.detail_dlines())
    }

    pub(crate) fn detail_line_count(&self) -> usize {
        self.detail_dlines().len()
    }

    /// Move the detail line cursor by `delta`, clamped, keeping it in view.
    pub(crate) fn move_detail_line(&mut self, delta: i32) {
        let n = self.detail_line_count();
        if n == 0 {
            return;
        }
        self.detail_line = (self.detail_line as i32 + delta).clamp(0, n as i32 - 1) as usize;
        self.scroll_line_into_view(self.detail_line as u16);
    }

    /// g / G — jump the line cursor to the top / bottom.
    pub(crate) fn detail_line_to(&mut self, end: bool) {
        let n = self.detail_line_count();
        self.detail_line = if end { n.saturating_sub(1) } else { 0 };
        self.scroll_line_into_view(self.detail_line as u16);
    }

    /// Tab / Shift-Tab — snap the line cursor to the next / prev line holding a
    /// link (wrapping), so Enter can follow it.
    pub(crate) fn jump_link(&mut self, delta: i32) {
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
    pub(crate) fn toggle_visual(&mut self) {
        self.detail_anchor = if self.detail_anchor.is_some() {
            None
        } else {
            Some(self.detail_line)
        };
    }

    /// Shift-↑↓ — start (if needed) and extend the visual selection.
    pub(crate) fn extend_selection(&mut self, delta: i32) {
        if self.detail_anchor.is_none() {
            self.detail_anchor = Some(self.detail_line);
        }
        self.move_detail_line(delta);
    }

    /// The inclusive [lo, hi] line range currently selected, if any.
    pub(crate) fn selection_range(&self) -> Option<(usize, usize)> {
        self.detail_anchor
            .map(|a| (a.min(self.detail_line), a.max(self.detail_line)))
    }

    /// y / Enter (in visual mode) — copy the selected line block (dedented) to
    /// the clipboard and clear the selection.
    pub(crate) fn yank_selection(&mut self) {
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
    pub(crate) fn scroll_line_into_view(&mut self, line: u16) {
        let vh = self.detail_page.max(1);
        if line < self.detail_scroll {
            self.detail_scroll = line;
        } else if line >= self.detail_scroll.saturating_add(vh) {
            self.detail_scroll = line.saturating_sub(vh - 1);
        }
    }

    pub(crate) fn open_detail_find(&mut self) {
        let cur = self.detail_find.clone();
        let mut sb = SearchBox::new(self.editor_vim, cur);
        // Stash where the cursor/scroll sit now so Esc can restore them (vi-like).
        sb.detail_origin = Some((self.detail_scroll, self.detail_line));
        self.overlay = Overlay::DetailFind(sb);
    }

    /// (line, col, len) of every detail-find match for the current selection.
    pub(crate) fn detail_find_matches(&self) -> Vec<(usize, usize, usize)> {
        let Some(q) = self.detail_find.as_deref().filter(|s| !s.is_empty()) else {
            return vec![];
        };
        if self.selected().is_none() {
            return vec![];
        }
        detail_scan(&self.detail_dlines(), q)
    }

    pub(crate) fn detail_find_jump(&mut self, delta: i32) {
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
    pub(crate) fn open_task_in_detail(&mut self, id: &str) {
        self.select_task(id); // drops focus to the list; we re-raise it below
        self.focus = Focus::Detail;
        self.detail_scroll = 0;
        self.detail_line = 0;
        self.detail_anchor = None;
        self.detail_find = None;
        self.detail_match = 0;
    }

    pub(crate) fn follow_link(&mut self) {
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
    pub(crate) fn copy_selected_id(&mut self) {
        let Some(id) = self.selected_id() else { return };
        self.notification = Some(if crate::clipboard::copy_text(&id) {
            format!("copied {id}")
        } else {
            "clipboard unavailable".into()
        });
    }

    /// J/K — move to the next/prev task in the list while staying in the detail
    /// pane, resetting the per-task detail view state.
    pub(crate) fn detail_next_task(&mut self, delta: i32) {
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
    pub(crate) fn nav_back(&mut self) {
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
    pub(crate) fn nav_forward(&mut self) {
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
    pub(crate) fn select_id(&mut self, id: &str) {
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
}
