//! `App`'s key handling, form commits, and overlay dispatch: `handle_create_key`,
//! `handle_drawer_key`, `handle_help_key`, `handle_cmdline_key`, the big
//! `handle_overlay_key` router, the `:`-command runners, `field_cancel`/
//! double-Esc, and the commit/resolve paths (`commit_*`, `apply_edit`,
//! `resolve_*`) — plus the top-level free `handle_key` dispatcher. Split out of
//! `tui.rs` (yaks-b1cc / b1cc/7). `use super::*` inherits `tui.rs`'s scope.

use super::*;

impl App {
    pub(crate) fn handle_create_key(&mut self, k: KeyEvent, double_esc: bool) {
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
        // In Normal mode a single-line field's j/k are free (no vertical motion
        // within one line), so they move between rows like Tab does on chips.
        let line_normal = is_line_text
            && matches!(&self.overlay, Overlay::Create(f)
                if f.line_editor().is_some_and(|e| e.borrow().mode == EditorMode::Normal));
        // Tab/BackTab move fields on chip + single-line rows, but inside a
        // content block they're normal editor keys (Ctrl-N/P still navigate).
        let nav_down = (matches!(k.code, KeyCode::Tab) && !is_content)
            || (ctrl && k.code == KeyCode::Char('n'))
            || (is_line_text && k.code == KeyCode::Down)
            || (line_normal && k.code == KeyCode::Char('j'))
            || (is_chip && matches!(k.code, KeyCode::Down | KeyCode::Char('j')));
        // BackTab (shift-tab) focuses the previous field on *every* row —
        // including content blocks, where forwarding it to edtui would panic
        // (`KeyCode::from` is `unimplemented!` for BackTab). It mirrors Tab's
        // focus-next, so it's field navigation even inside a content block.
        let nav_up = matches!(k.code, KeyCode::BackTab)
            || (ctrl && k.code == KeyCode::Char('p'))
            || (is_line_text && k.code == KeyCode::Up)
            || (line_normal && k.code == KeyCode::Char('k'))
            || (is_chip && matches!(k.code, KeyCode::Up | KeyCode::Char('k')));
        if nav_down || nav_up {
            // Carry Normal across a j/k field-nav so it keeps moving instead of
            // dropping back to Insert (typing) on the next single-line row.
            let carry = line_normal && matches!(k.code, KeyCode::Char('j') | KeyCode::Char('k'));
            if let Overlay::Create(f) = &mut self.overlay {
                let n = f.row_count();
                f.row = if nav_down {
                    (f.row + 1) % n
                } else {
                    (f.row + n - 1) % n
                };
                if carry {
                    if let Some(e) = f.line_editor() {
                        e.borrow_mut().mode = EditorMode::Normal;
                    }
                }
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
            // Tab opens the ref completer when the cursor is in a `<prefix>-…`
            // context; otherwise Tab is a normal editor key here.
            let complete = if k.code == KeyCode::Tab {
                match &self.overlay {
                    Overlay::Create(f) => f
                        .content_index()
                        .and_then(|i| completion_context(&f.blocks[i].editor, &self.ref_prefix)),
                    _ => None,
                }
            } else {
                None
            };
            if let Some((replace_len, seed)) = complete {
                self.open_ref_picker(replace_len, &seed);
                return;
            }
            // Never hand edtui a key it can't convert (it would panic).
            if edtui_can_handle(k.code) {
                if let Overlay::Create(f) = &mut self.overlay {
                    if let Some(i) = f.content_index() {
                        f.handler
                            .on_key_event(k, &mut f.blocks[i].editor.borrow_mut());
                    }
                }
            }
            return;
        }
        if is_line_text {
            // Same guard: drop keys edtui can't handle rather than panic.
            if edtui_can_handle(k.code) {
                if let Overlay::Create(f) = &mut self.overlay {
                    match f.row {
                        0 => f.handler.on_key_event(k, &mut f.title.borrow_mut()),
                        3 => f.handler.on_key_event(k, &mut f.labels.borrow_mut()),
                        _ => {}
                    }
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

    pub(crate) fn commit_form(&mut self) {
        let editing = matches!(&self.overlay, Overlay::Create(f) if f.is_editing());
        if editing {
            self.commit_edit_form();
        } else {
            self.commit_create();
        }
    }

    pub(crate) fn commit_create(&mut self) {
        let new = match std::mem::replace(&mut self.overlay, Overlay::None) {
            Overlay::Create(f) => NewTask {
                title: f.title_text(),
                prefix: f.target_herd(),
                kind: Some(TYPE_CHOICES[f.kind_idx].to_string()),
                priority: Some(PRI_CHOICES[f.pri_idx]),
                parent: f.parent.clone(),
                labels: f.labels_vec(),
                depends_on: vec![],
                source: None,
                description: f.body_opt(),
                verify: None,
            },
            other => {
                self.overlay = other;
                return;
            }
        };
        let Some(h) = &self.farm else { return };
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
            // The create form has no prefix field yet, so this is unreachable in
            // practice; surface it as a notification rather than panicking.
            Ok(CreateOutcome::InvalidPrefix(p)) => {
                self.notification = Some(format!("invalid prefix {p:?}"))
            }
            Err(e) => self.notification = Some(format!("error: {e}")),
        }
    }

    /// Commit an edit: diff the form against the current task so unchanged
    /// fields don't rewrite the file or bump `updated`.
    pub(crate) fn commit_edit_form(&mut self) {
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

    pub(crate) fn open_dep_picker(&mut self) {
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

    pub(crate) fn open_reparent_picker(&mut self) {
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

    /// Open the yak-ref picker over the active editor so the chosen id is
    /// inserted at the editor's cursor (ref autocomplete, yaks-5656). The editor
    /// is stashed in the picker's `return_to`, so both commit and cancel come
    /// back to it with its content intact.
    /// Open the yak-ref picker over whatever editor overlay is active (the
    /// comment `Overlay::Edit` or the create/edit form `Overlay::Create`),
    /// stashing it so commit/cancel return to it. `replace_len` is the typed
    /// `<prefix>-` to drop before inserting the chosen id.
    pub(crate) fn open_ref_picker(&mut self, replace_len: usize, seed: &str) {
        let back = std::mem::replace(&mut self.overlay, Overlay::None);
        let mut fp = FuzzyPick::new(
            self.editor_vim,
            "Insert yak reference".into(),
            HashSet::new(),
            false,
            FuzzyAction::InsertRef,
        );
        fp.return_to = Some(Box::new(back));
        fp.replace_len = replace_len;
        // Pre-filter by whatever tail was already typed after `<prefix>-`.
        if !seed.is_empty() {
            insert_into(&mut fp.handler, &fp.query, seed);
        }
        self.overlay = Overlay::Fuzzy(fp);
    }

    pub(crate) fn open_search(&mut self) {
        let cur = self.filter.search.clone();
        self.overlay = Overlay::Search(SearchBox::new(self.editor_vim, cur));
    }

    pub(crate) fn open_drawer(&mut self) {
        let choices = self.herd_choices();
        self.overlay = Overlay::Drawer(Drawer::from_filter(self.editor_vim, &self.filter, choices));
    }

    pub(crate) fn open_help(&mut self) {
        self.overlay = Overlay::Help(0);
    }

    /// A — attach an artifact (a file path, or the clipboard PNG when blank).
    pub(crate) fn open_attach(&mut self) {
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

    pub(crate) fn commit_attach(&mut self, id: String, path_input: String) {
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
        let Some(h) = &self.farm else { return };
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
    pub(crate) fn open_external(&mut self, target: &str) {
        let arg = if target.starts_with("http") {
            target.to_string()
        } else {
            match &self.farm {
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
    pub(crate) fn open_current_external(&mut self) {
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
    pub(crate) fn open_comment(&mut self) {
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

    /// Context-sensitive needs affordance: answer the selected yak if it already
    /// carries a `needs` block, otherwise ask (raise one). One key, two verbs —
    /// the prompt label states which.
    pub(crate) fn open_ask_or_answer(&mut self) {
        let blocked = self.selected().is_some_and(|t| t.needs.is_some());
        if blocked {
            self.open_answer();
        } else {
            self.open_ask();
        }
    }

    pub(crate) fn open_ask(&mut self) {
        if let Some(id) = self.selected_id() {
            self.overlay = Overlay::Edit(Editor::new(
                self.editor_vim,
                true,
                format!("Ask {id} (blocks on a human) — Enter save · Ctrl-C cancel"),
                "",
                EditAction::Ask(id),
            ));
        }
    }

    pub(crate) fn open_answer(&mut self) {
        if let Some(id) = self.selected_id() {
            self.overlay = Overlay::Edit(Editor::new(
                self.editor_vim,
                true,
                format!("Answer {id} (clears the block) — Enter save · Ctrl-C cancel"),
                "",
                EditAction::Answer(id),
            ));
        }
    }

    /// Set or clear a `needs` block via the farm, appending the typed text as an
    /// attributed note when non-empty. Backs the `Ask`/`Answer` edit actions.
    pub(crate) fn set_needs_edit(
        &mut self,
        id: &str,
        needs: Option<String>,
        note: Option<&str>,
        ok: String,
    ) {
        let actor = crate::actor::resolve(None);
        let Some(h) = &self.farm else { return };
        match h.set_needs(id, needs, actor.as_deref(), note) {
            Ok(Some(_)) => {
                self.reload();
                self.notification = Some(ok);
            }
            Ok(None) => self.notification = Some(format!("{id} not found")),
            Err(e) => self.notification = Some(format!("error: {e}")),
        }
    }

    pub(crate) fn handle_help_key(&mut self, k: KeyEvent) {
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

    pub(crate) fn open_save_view(&mut self) {
        self.overlay = Overlay::Edit(Editor::new(
            self.editor_vim,
            true,
            "Save view as: ".into(),
            "",
            EditAction::SaveView,
        ));
    }

    pub(crate) fn drawer_live_preview(&mut self) {
        let spec = match &self.overlay {
            Overlay::Drawer(d) => Some(d.build_spec()),
            _ => None,
        };
        if let Some(s) = spec {
            self.filter = s;
            self.clamp_cursor();
        }
    }

    pub(crate) fn close_drawer(&mut self, commit: bool) {
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

    pub(crate) fn handle_drawer_key(&mut self, k: KeyEvent, double_esc: bool) {
        let ctrl = k.modifiers.contains(KeyModifiers::CONTROL);
        let (row, is_text) = match &self.overlay {
            Overlay::Drawer(d) => (d.row, d.is_text_row()),
            _ => return,
        };
        // Global apply / cancel / clear. Esc closes the drawer, except on a text
        // row in vim, where a lone Esc drops the field to Normal (motions); a
        // Ctrl-C or a rapid double-Esc always closes.
        let close = (ctrl && k.code == KeyCode::Char('c'))
            || double_esc
            || (k.code == KeyCode::Esc && !(self.editor_vim && is_text));
        if k.code == KeyCode::Enter {
            return self.close_drawer(true);
        }
        if close {
            return self.close_drawer(false);
        }
        if k.code == KeyCode::Char('C') {
            if let Overlay::Drawer(d) = &mut self.overlay {
                d.clear();
            }
            return self.drawer_live_preview();
        }
        // Row navigation. On text rows only Tab/arrows/Ctrl move rows (so j/k
        // remain typeable); on chip rows j/k also navigate.
        // On a text row in Normal mode, j/k are free, so they move rows too.
        let text_normal = is_text
            && matches!(&self.overlay, Overlay::Drawer(d)
                if d.text_editor().is_some_and(|e| e.borrow().mode == EditorMode::Normal));
        let nav_down = matches!(k.code, KeyCode::Down | KeyCode::Tab)
            || (ctrl && k.code == KeyCode::Char('n'))
            || (!is_text && k.code == KeyCode::Char('j'))
            || (text_normal && k.code == KeyCode::Char('j'));
        let nav_up = matches!(k.code, KeyCode::Up | KeyCode::BackTab)
            || (ctrl && k.code == KeyCode::Char('p'))
            || (!is_text && k.code == KeyCode::Char('k'))
            || (text_normal && k.code == KeyCode::Char('k'));
        if nav_down || nav_up {
            let carry = text_normal && matches!(k.code, KeyCode::Char('j') | KeyCode::Char('k'));
            if let Overlay::Drawer(d) = &mut self.overlay {
                let n = d.row_count();
                d.row = if nav_down {
                    (d.row + 1) % n
                } else {
                    (d.row + n - 1) % n
                };
                d.chip_idx = 0;
                if carry {
                    if let Some(e) = d.text_editor() {
                        e.borrow_mut().mode = EditorMode::Normal;
                    }
                }
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

    /// Record an `Esc` in an editor overlay and report whether it's a *rapid*
    /// second `Esc` (within [`DOUBLE_ESC_MS`] of the previous one) — the
    /// Ctrl-C-equivalent cancel gesture. A lone `Esc` keeps its usual meaning
    /// (leave Insert for Normal); only the fast second press cancels.
    pub(crate) fn register_double_esc(&mut self) -> bool {
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
    pub(crate) fn request_cancel(&mut self, dirty: bool, msg: &str) {
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
    pub(crate) fn form_is_dirty(&self, f: &CreateForm) -> bool {
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
    pub(crate) fn handle_cmdline_key(&mut self, k: KeyEvent) {
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
    pub(crate) fn run_command(&mut self, cmd: &str) {
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
    pub(crate) fn cmd_write(&mut self) {
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
    pub(crate) fn cmd_quit(&mut self, force: bool) {
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

    /// Whether a key should trigger a single-line field overlay's cancel/close
    /// path. In vim a lone Esc instead drops the field to Normal (so the
    /// normal-mode motions are reachable) and only Ctrl-C or a rapid double-Esc
    /// cancels; emacs keeps the familiar single-Esc-cancels.
    pub(crate) fn field_cancel(&self, k: KeyEvent, double_esc: bool) -> bool {
        (k.modifiers.contains(KeyModifiers::CONTROL) && k.code == KeyCode::Char('c'))
            || double_esc
            || (k.code == KeyCode::Esc && !self.editor_vim)
    }

    pub(crate) fn handle_overlay_key(&mut self, k: KeyEvent) {
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
            self.handle_drawer_key(k, double_esc);
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
            // In Normal mode j/k move the result selection (the query's own j/k
            // have no single-line meaning).
            let q_normal = matches!(&self.overlay, Overlay::Fuzzy(fp)
                if fp.query.borrow().mode == EditorMode::Normal);
            let up = k.code == KeyCode::Up
                || (ctrl && k.code == KeyCode::Char('p'))
                || (q_normal && k.code == KeyCode::Char('k'));
            let down = matches!(k.code, KeyCode::Down | KeyCode::Tab)
                || (ctrl && k.code == KeyCode::Char('n'))
                || (q_normal && k.code == KeyCode::Char('j'));
            // Once the query is in Normal mode (after the first Esc dropped it
            // there), a single further Esc closes the picker — no rapid
            // double-Esc needed. Ctrl-C still cancels from anywhere.
            let esc_from_normal = q_normal && k.code == KeyCode::Esc;
            if self.field_cancel(k, double_esc) || esc_from_normal {
                // A picker opened over an editor (ref autocomplete) returns there;
                // a top-level picker just closes.
                let back = if let Overlay::Fuzzy(fp) = &mut self.overlay {
                    fp.return_to.take()
                } else {
                    None
                };
                match back {
                    Some(b) => self.overlay = *b,
                    None => {
                        self.overlay = Overlay::None;
                        self.notification = Some("cancelled".into());
                    }
                }
            } else if k.code == KeyCode::Enter {
                if let Overlay::Fuzzy(fp) = std::mem::replace(&mut self.overlay, Overlay::None) {
                    self.commit_fuzzy(fp);
                }
            } else if up {
                if let Overlay::Fuzzy(fp) = &mut self.overlay {
                    fp.sel = fp.sel.saturating_sub(1);
                }
            } else if down {
                let total = match &self.overlay {
                    Overlay::Fuzzy(fp) => fuzzy_total(&self.all, fp),
                    _ => 0,
                };
                if let Overlay::Fuzzy(fp) = &mut self.overlay {
                    if total > 0 {
                        fp.sel = (fp.sel + 1).min(total - 1);
                    }
                }
            } else if let Overlay::Fuzzy(fp) = &mut self.overlay {
                fp.handler.on_key_event(k, &mut fp.query.borrow_mut());
                fp.sel = 0;
            }
            return;
        }
        // Inline search: every keystroke edits the live filter for instant
        // preview; Enter keeps it, Esc restores the pre-search query.
        if matches!(self.overlay, Overlay::Search(_)) {
            if self.field_cancel(k, double_esc) {
                let saved = match &self.overlay {
                    Overlay::Search(sb) => sb.saved.clone(),
                    _ => None,
                };
                self.filter.search = saved;
                self.overlay = Overlay::None;
                self.clamp_cursor();
                self.notification = Some("search cleared".into());
            } else if k.code == KeyCode::Enter {
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
            } else {
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
            return;
        }
        // Detail-pane find: live-highlight matches; Enter keeps, Esc restores.
        if matches!(self.overlay, Overlay::DetailFind(_)) {
            if self.field_cancel(k, double_esc) {
                // Esc/Ctrl-C: drop the find and snap back to the pre-find
                // scroll+cursor (vi restores the origin on cancel).
                let (saved, origin) = match &self.overlay {
                    Overlay::DetailFind(sb) => (sb.saved.clone(), sb.detail_origin),
                    _ => (None, None),
                };
                self.detail_find = saved;
                if let Some((scroll, line)) = origin {
                    self.detail_scroll = scroll;
                    self.detail_line = line;
                }
                self.overlay = Overlay::None;
            } else if k.code == KeyCode::Enter {
                // Commit: leave the detail cursor ON the current match, keeping
                // the scroll where the incremental find placed it (vi-like).
                if let Some(&(line, ..)) = self.detail_find_matches().get(self.detail_match) {
                    self.detail_line = line;
                }
                self.overlay = Overlay::None;
            } else {
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
            return;
        }
        // Editors are handled in place (most keys flow to edtui); only the
        // Pick/Confirm variants move their action out on resolution.
        if matches!(self.overlay, Overlay::Edit(_)) {
            // Decide within a short borrow, then act once it's released.
            let ref_prefix = self.ref_prefix.clone();
            let (do_commit, do_cancel, dirty, open_cmd, open_ref) = {
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
                // Tab opens the ref completer when the cursor is in a `<prefix>-…`
                // context; otherwise it falls through as a normal editor key.
                let complete = if k.code == KeyCode::Tab {
                    completion_context(&ed.state, &ref_prefix)
                } else {
                    None
                };
                if commit {
                    (true, false, false, false, None)
                } else if complete.is_some() {
                    (false, false, false, false, complete)
                } else if open_cmd {
                    (false, false, false, true, None)
                } else if cancel {
                    // Only the multiline comment editor guards unsaved work.
                    let dirty = !ed.single_line && !ed.text().trim().is_empty();
                    (false, true, dirty, false, None)
                } else {
                    ed.handler.on_key_event(k, &mut ed.state.borrow_mut());
                    (false, false, false, false, None)
                }
            };
            if do_commit {
                if let Overlay::Edit(ed) = std::mem::replace(&mut self.overlay, Overlay::None) {
                    self.commit_edit(ed);
                }
            } else if let Some((replace_len, seed)) = open_ref {
                self.open_ref_picker(replace_len, &seed);
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

    pub(crate) fn commit_edit(&mut self, ed: Editor) {
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
                // Attribute TUI comments like CLI ones: $YAKS_ACTOR, else git
                // user (no --as in the TUI). Resolved lazily here so the
                // read-only/snapshot constructor stays pure (no git subprocess).
                self.apply_edit(
                    &id,
                    TaskEdit {
                        note: Some(text),
                        actor: crate::actor::resolve(None),
                        ..Default::default()
                    },
                    format!("comment added to {id}"),
                );
            }
            EditAction::Attach(id) => self.commit_attach(id, text),
            EditAction::Ask(id) => {
                let note = text.trim();
                let note = (!note.is_empty()).then_some(note);
                self.set_needs_edit(
                    &id,
                    Some("human".into()),
                    note,
                    format!("asked {id}: needs human (out of next until answered)"),
                );
            }
            EditAction::Answer(id) => {
                let note = text.trim();
                let note = (!note.is_empty()).then_some(note);
                self.set_needs_edit(&id, None, note, format!("answered {id}: block cleared"));
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

    pub(crate) fn commit_fuzzy(&mut self, mut fp: FuzzyPick) {
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
        // Ref autocomplete: insert the picked id (if any) and always return to
        // the stashed editor — even on an empty pick, so the editor is never lost.
        if let FuzzyAction::InsertRef = fp.action {
            let picked = target.flatten();
            if let Some(back) = fp.return_to.take() {
                let mut ov = *back;
                if let Some(id) = &picked {
                    // Replace the typed `<prefix>-` with the chosen id, in
                    // whichever editor the picker was opened over.
                    match &mut ov {
                        Overlay::Edit(ed) => {
                            delete_into(&mut ed.handler, &ed.state, fp.replace_len);
                            insert_into(&mut ed.handler, &ed.state, id);
                        }
                        Overlay::Create(f) => {
                            if let Some(i) = f.content_index() {
                                delete_into(&mut f.handler, &f.blocks[i].editor, fp.replace_len);
                                insert_into(&mut f.handler, &f.blocks[i].editor, id);
                            }
                        }
                        _ => {}
                    }
                }
                self.overlay = ov;
            }
            return;
        }
        let Some(target) = target else {
            self.notification = Some("nothing selected".into());
            return;
        };
        match fp.action {
            FuzzyAction::AddDep(id) => {
                let Some(dep) = target else { return };
                let Some(h) = &self.farm else { return };
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
                let Some(h) = &self.farm else { return };
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
            FuzzyAction::InsertRef => unreachable!("InsertRef is handled before this match"),
        }
    }

    pub(crate) fn resolve_pick(&mut self, action: PickAction, c: char) {
        match action {
            PickAction::BulkState(ids) => {
                let dest = match c {
                    'h' => Status::Hairy,
                    's' => Status::Shaving,
                    'n' => Status::Shorn,
                    'x' => Status::Dead,
                    _ => return,
                };
                // Slaughter must not orphan children: mirror the single-path
                // guard (open_slaughter_confirm) by skipping any marked id with
                // live children and reporting the count. Non-slaughter bulk
                // transitions are unaffected (yaks-5c51).
                let mut skipped = 0usize;
                let targets: Vec<&String> = if dest == Status::Dead {
                    ids.iter()
                        .filter(|id| {
                            let keep = self.live_child_count(id) == 0;
                            if !keep {
                                skipped += 1;
                            }
                            keep
                        })
                        .collect()
                } else {
                    ids.iter().collect()
                };
                let Some(h) = &self.farm else { return };
                let (mut moved, mut failed) = (0usize, 0usize);
                for id in &targets {
                    match h.transition(id, dest) {
                        Ok(MoveOutcome::Moved) => moved += 1,
                        Ok(_) => {}
                        Err(_) => failed += 1,
                    }
                }
                let total = ids.len();
                self.selected.clear();
                self.reload();
                let mut msg = if failed > 0 {
                    format!("{moved}/{total} → {} ({failed} failed)", status_word(dest))
                } else {
                    format!("{moved}/{total} → {}", status_word(dest))
                };
                if skipped > 0 {
                    msg.push_str(&format!(" · {skipped} skipped: have children"));
                }
                self.notification = Some(msg);
            }
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
                let Some(h) = &self.farm else { return };
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
            PickAction::Herd(id, herds) => {
                // The digit key is 1-based into the candidate herds.
                let Some(idx) = c.to_digit(10).map(|d| d as usize) else {
                    return;
                };
                let Some(dest) = idx.checked_sub(1).and_then(|i| herds.get(i)) else {
                    return;
                };
                // Moving herd = renaming the id's prefix, keeping the tail.
                let Some((_, tail)) = id.split_once('-') else {
                    self.notification = Some(format!("{id} has no herd"));
                    return;
                };
                let new_id = format!("{dest}-{tail}");
                let Some(h) = &self.farm else { return };
                match h.rename(&id, &new_id, false) {
                    Ok(RenameOutcome::Done(_)) => {
                        self.reload();
                        self.select_id(&new_id);
                        self.notification = Some(format!("moved to {dest}"));
                    }
                    Ok(RenameOutcome::Collision(t)) => {
                        self.notification = Some(format!("{t} already exists in {dest}"));
                    }
                    Ok(RenameOutcome::NotFound(t)) => {
                        self.notification = Some(format!("{t} not found"));
                    }
                    Ok(RenameOutcome::Invalid(t)) => {
                        self.notification = Some(format!("invalid id {t}"));
                    }
                    Ok(RenameOutcome::NothingToRename) => {
                        self.notification = Some(format!("{id} unchanged"));
                    }
                    Err(e) => self.notification = Some(format!("error: {e}")),
                }
            }
        }
    }

    pub(crate) fn apply_edit(&mut self, id: &str, edit: TaskEdit, ok_msg: String) {
        let Some(h) = &self.farm else { return };
        match h.update(id, edit) {
            Ok(UpdateOutcome::Updated) => {
                // Follow the edited yak to its new sorted slot. The cursor is
                // index-based, so an edit that changes the sort key (e.g.
                // priority) would otherwise leave it pointing at whatever yak
                // now sits in that slot (yaks-f207). Capture the id first so we
                // can tell whether the edit filtered it out of view.
                let edited = self.selected_id();
                self.reload_preserving_selection();
                // If the edit pushed the yak out of the current filtered/sorted
                // list entirely, close the detail pane -- what you were looking
                // at is no longer here (human-confirmed drop-out behavior).
                if self.focus == Focus::Detail
                    && edited.is_some_and(|id| !self.rows().iter().any(|r| r.task.id == id))
                {
                    self.focus = Focus::List;
                }
                self.notification = Some(ok_msg);
            }
            Ok(UpdateOutcome::NoChanges) => self.notification = Some(format!("{id} unchanged")),
            Ok(UpdateOutcome::NotFound) => self.notification = Some(format!("{id} not found")),
            Err(e) => self.notification = Some(format!("error: {e}")),
        }
    }

    pub(crate) fn resolve_confirm(&mut self, action: ConfirmAction) {
        match action {
            ConfirmAction::DiscardEdit => {
                self.dirty_cancel = None; // drop the stashed editor
                self.notification = Some("changes discarded".into());
            }
            ConfirmAction::Slaughter(id) => {
                let Some(h) = &self.farm else { return };
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

pub(crate) fn handle_key(app: &mut App, k: KeyEvent) {
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
            // Multi-select: mark/unmark the cursor's yak for a bulk action.
            KeyCode::Char('m') => app.toggle_selected(),
            // Mutations (single-key pickers + confirm). `S` is selection-aware:
            // with marks it drives a bulk state transition, else the single pick.
            KeyCode::Char('S') => {
                if app.selected.is_empty() {
                    app.open_state_picker()
                } else {
                    app.open_bulk_state_picker()
                }
            }
            KeyCode::Char('P') => app.open_priority_picker(),
            KeyCode::Char('T') => app.open_type_picker(),
            KeyCode::Char('X') => app.open_slaughter_confirm(),
            KeyCode::Char('L') => app.open_labels(),
            KeyCode::Char('c') => app.open_create(false),
            KeyCode::Char('C') => app.open_create(true),
            KeyCode::Char('E') => app.open_edit(),
            KeyCode::Char('D') => app.open_dep_picker(),
            KeyCode::Char('R') => app.open_reparent_picker(),
            KeyCode::Char('H') => app.open_herd_picker(),
            KeyCode::Char('/') => app.open_search(),
            KeyCode::Char('f') => app.open_drawer(),
            KeyCode::Char('*') => app.toggle_star(),
            KeyCode::Char('y') => app.copy_selected_id(),
            KeyCode::Char('M') => app.open_comment(),
            KeyCode::Char('A') => app.open_attach(),
            KeyCode::Char('a') => app.open_ask_or_answer(),
            KeyCode::Char('v') => app.open_view_picker(),
            KeyCode::Char('V') => app.open_save_view(),
            KeyCode::Char('?') => app.open_help(),
            KeyCode::Esc => {
                if app.is_view_modified() {
                    app.revert_filter_to_view();
                    app.notification = Some("reverted to view".into());
                }
            }
            KeyCode::Char('h') => app.cycle_family_scope(),
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
                // Multi-select mark, mirrored from the list pane (yaks-5c51,
                // deferred from yaks-de85); the mark set is shared, so a bulk
                // `S` back on the list acts on marks made here too.
                KeyCode::Char('m') => app.toggle_selected(),
                KeyCode::Char('S') => app.open_state_picker(),
                KeyCode::Char('P') => app.open_priority_picker(),
                KeyCode::Char('T') => app.open_type_picker(),
                KeyCode::Char('L') => app.open_labels(),
                KeyCode::Char('X') => app.open_slaughter_confirm(),
                KeyCode::Char('E') => app.open_edit_at_cursor(),
                KeyCode::Char('D') => app.open_dep_picker(),
                KeyCode::Char('R') => app.open_reparent_picker(),
                KeyCode::Char('H') => app.open_herd_picker(),
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
                KeyCode::Char('a') => app.open_ask_or_answer(),
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
