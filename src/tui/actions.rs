//! `App`'s view actions and overlay openers: star/collapse/mark toggles, save
//! and pick views, cursor + `state_header`, and the `open_*` methods that raise
//! the create/edit form, pickers, drawer, help, attach, comment, and ask/answer
//! overlays. Split out of `tui.rs` (yaks-b1cc / b1cc/7) as an `impl App` block;
//! `use super::*` inherits `tui.rs`'s scope and the methods are `pub(crate)`.

use super::*;

impl App {
    pub(crate) fn toggle_star(&mut self) {
        let Some(id) = self.selected_id() else { return };
        let was = self.working_set.iter().any(|i| *i == id);
        self.working_set = views_store::toggle_working_set(&self.working_set, &id);
        if let Some(h) = &self.farm {
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

    pub(crate) fn is_starred(&self, id: &str) -> bool {
        self.working_set.iter().any(|i| i == id)
    }

    pub(crate) fn save_current_view(&mut self, name: String) {
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
        if let Some(h) = &self.farm {
            views_store::save_views(h.root(), &self.views);
        }
        self.set_view(self.views.len() - 1);
        self.notification = Some(format!("saved view: {name}"));
    }

    pub(crate) fn persist_views(&self) {
        if let Some(h) = &self.farm {
            views_store::save_views(h.root(), &self.views);
        }
    }

    pub(crate) fn open_view_picker(&mut self) {
        self.overlay = Overlay::ViewPicker(self.view.min(self.views.len().saturating_sub(1)));
    }

    pub(crate) fn handle_view_picker_key(&mut self, k: KeyEvent) {
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
    pub(crate) fn reorder_view(&mut self, sel: usize, dir: i32) {
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

    pub(crate) fn delete_view(&mut self, sel: usize) {
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

    pub(crate) fn move_cursor(&mut self, delta: i32) {
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
    pub(crate) fn state_header(&self) -> String {
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
        let base = format!(
            "focus={focus} · view={} · cursor={} · rows={} · sel={sel} · blocked={blocked} · collapsed={} · filter={filter} · overlay={}",
            self.active_view().name,
            self.cursor,
            self.rows().len(),
            self.collapsed.len(),
            overlay_name(&self.overlay),
        );
        // In the detail pane, expose the scroll offset and whether the sticky
        // header is showing (the frontmatter top has scrolled off).
        if self.focus == Focus::Detail {
            format!(
                "{base} · detail_scroll={} · pinned={}",
                self.detail_scroll,
                if self.detail_scroll > 0 { "yes" } else { "no" }
            )
        } else {
            base
        }
    }

    pub(crate) fn toggle_collapse(&mut self) {
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

    /// Toggle the cursor's yak in the multi-select set. `m` is the toggle key:
    /// the natural `Space` is already collapse/expand, and vi-style `v` collides
    /// with the `[v]iew` picker (that v-vs-view rebinding is deferred; yaks-de85).
    pub(crate) fn toggle_selected(&mut self) {
        let Some(id) = self.selected_id() else { return };
        if self.selected.remove(&id) {
            self.notification = Some(format!("unmarked {id} ({} marked)", self.selected.len()));
        } else {
            self.selected.insert(id.clone());
            self.notification = Some(format!("marked {id} ({} marked)", self.selected.len()));
        }
    }

    /// Open the h/s/n/x picker as a bulk action over the multi-select set. The
    /// resolved pick loops every marked id through `farm.transition` and then
    /// clears the set (see [`App::resolve_pick`]).
    pub(crate) fn open_bulk_state_picker(&mut self) {
        let ids: Vec<String> = self.selected.iter().cloned().collect();
        if ids.is_empty() {
            return;
        }
        let n = ids.len();
        self.overlay = Overlay::Pick {
            prompt: format!(
                "State for {n} marked: h=hairy s=shaving n=shorn x=slaughter  (Esc=cancel)"
            ),
            keys: "hsnx".into(),
            action: PickAction::BulkState(ids),
        };
    }

    pub(crate) fn open_state_picker(&mut self) {
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

    pub(crate) fn open_priority_picker(&mut self) {
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

    pub(crate) fn open_type_picker(&mut self) {
        if let Some(id) = self.selected_id() {
            self.overlay = Overlay::Pick {
                prompt: format!("Type for {id}: t=task b=bug f=feature i=idea  (Esc=cancel)"),
                keys: "tbfi".into(),
                action: PickAction::Type(id),
            };
        }
    }

    /// Count a yak's live (non-dead) children. Slaughter refuses to orphan
    /// children, so both the single path (`open_slaughter_confirm`) and the
    /// bulk path (`PickAction::BulkState` with `x`) gate on this (yaks-5c51).
    pub(crate) fn live_child_count(&self, id: &str) -> usize {
        self.all
            .iter()
            .filter(|c| c.parent.as_deref() == Some(id) && c.status != Status::Dead)
            .count()
    }

    pub(crate) fn open_slaughter_confirm(&mut self) {
        let Some(id) = self.selected_id() else { return };
        let kids = self.live_child_count(&id);
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

    pub(crate) fn open_labels(&mut self) {
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

    pub(crate) fn open_create(&mut self, child: bool) {
        let parent = if child { self.selected_id() } else { None };
        if child && parent.is_none() {
            return;
        }
        let herd = self.herd_for_new(parent.as_deref());
        self.overlay = Overlay::Create(CreateForm::new(self.editor_vim, parent, herd));
    }

    /// The target herd (id prefix) for a new yak, or `None` in a single-herd
    /// farm (where the config default applies). In a multi-herd farm, inherit
    /// the reference yak's herd — the parent for a child create, else the
    /// selected row — so `c` near a herd creates into it.
    fn herd_for_new(&self, reference: Option<&str>) -> Option<String> {
        let mut prefixes = std::collections::HashSet::new();
        for t in &self.all {
            if let Some(p) = t.id.split('-').next() {
                prefixes.insert(p);
            }
        }
        if prefixes.len() <= 1 {
            return None;
        }
        reference
            .map(String::from)
            .or_else(|| self.selected_id())
            .and_then(|id| id.split('-').next().map(String::from))
    }

    /// Open the shared form seeded from the selected task, for editing (E).
    pub(crate) fn open_edit(&mut self) {
        self.open_edit_focus(None);
    }

    /// `E` from the detail pane: open the edit form focused on whatever the line
    /// cursor sits on — a header field, the description, or a specific comment.
    /// Status routes to its own picker (it isn't a form field).
    pub(crate) fn open_edit_at_cursor(&mut self) {
        match self.edit_target_at(self.detail_line) {
            Some(EditTarget::Status) => self.open_state_picker(),
            other => self.open_edit_focus(other),
        }
    }

    /// Open the edit form, optionally pre-focusing a row derived from `target`.
    pub(crate) fn open_edit_focus(&mut self, target: Option<EditTarget>) {
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
    pub(crate) fn edit_target_at(&self, line: usize) -> Option<EditTarget> {
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
    pub(crate) fn block_starts(&self) -> Vec<usize> {
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
    pub(crate) fn jump_block(&mut self, delta: i32) {
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
}
