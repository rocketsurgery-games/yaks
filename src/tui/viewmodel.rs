//! `App`'s view-model / read-model layer: the farm-derived row lists (tree +
//! flat + working-set), selection and cursor, per-view family-scope, view/tab
//! management, and the reload paths. Split out of `tui.rs` (yaks-b1cc / b1cc/6)
//! as an `impl App` block. `use super::*` inherits everything `tui.rs` has in
//! scope; the methods are `pub(crate)` so the rest of the crate can call them.

use super::*;

impl App {
    /// Persist the (rebuildable) UI state — collapsed rows and family-scope
    /// overrides — to the per-user cache.
    pub(crate) fn save_ui_state(&self) {
        if let Some(h) = &self.farm {
            cache::save(
                h.root(),
                &cache::UiState {
                    collapsed: self.collapsed.clone(),
                    family: self.family_scope.clone(),
                },
            );
        }
    }

    /// The family scope in effect for `v`: its persisted override, else the global
    /// default. Only meaningful for tree views.
    pub(crate) fn resolved_family_scope(&self, v: &view::View) -> view::FamilyScope {
        self.family_scope
            .get(&v.key)
            .copied()
            .unwrap_or(view::FamilyScope::DEFAULT)
    }

    /// Cycle the active view's family scope (the `h` key): auto -> lone ->
    /// remaining -> all -> auto. Flat/working-set views have no tree, so it's a
    /// no-op there with a hint.
    pub(crate) fn cycle_family_scope(&mut self) {
        let v = self.active_view();
        if v.is_flat() || v.key == "working-set" {
            self.notification = Some("family scope applies to tree views".into());
            return;
        }
        let key = v.key.clone();
        let next = view::FamilyScope::cycle(self.family_scope.get(&key).copied());
        match next {
            Some(s) => {
                self.family_scope.insert(key, s);
            }
            None => {
                self.family_scope.remove(&key);
            }
        }
        self.save_ui_state();
        self.notification = Some(match next {
            Some(s) => format!("family: {}", s.as_str()),
            None => format!("family: ~{}", view::FamilyScope::DEFAULT.as_str()),
        });
    }

    /// The create-form herd picker's choices: the farm's declared herds unioned
    /// with the id prefixes actually present, sorted and de-duplicated.
    pub(crate) fn herd_choices(&self) -> Vec<String> {
        let mut set: std::collections::BTreeSet<String> =
            self.config_herds.iter().cloned().collect();
        for t in &self.all {
            if let Some(p) = t.id.split('-').next() {
                set.insert(p.to_string());
            }
        }
        set.into_iter().collect()
    }

    /// True when the farm spans more than one herd (declared or present) — what
    /// activates the per-herd id colour and the create-form herd picker.
    pub(crate) fn is_multi_herd(&self) -> bool {
        self.herd_choices().len() > 1
    }

    /// Re-query the farm view after a mutation and keep the cursor in range.
    pub(crate) fn reload(&mut self) {
        if let Some(h) = &self.farm {
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

    /// Re-read the farm from disk (external change) while keeping the cursor on
    /// the same task by id. Silent: it must not clobber a mutation's own
    /// notification, and the event loop only calls it while idle (no overlay).
    pub(crate) fn reload_preserving_selection(&mut self) {
        let sel = self.selected_id();
        if let Some(h) = &self.farm {
            if let Ok(all) = h.list(FilterSpec::default(), true) {
                self.all = all;
            }
        }
        match sel.and_then(|id| self.rows().iter().position(|r| r.task.id == id)) {
            Some(pos) => self.cursor = pos,
            None => self.clamp_cursor(),
        }
    }

    pub(crate) fn active_view(&self) -> &view::View {
        &self.views[self.view]
    }

    /// Count for a view: non-ghost tree rows of its own spec, or working-set
    /// membership. Independent of the live filter (a stable per-view size).
    pub(crate) fn view_count(&self, v: &view::View) -> usize {
        if v.key == "working-set" {
            return self
                .working_set
                .iter()
                .filter(|id| self.task(id).is_some())
                .count();
        }
        // Flat views (Recent/Inbox/custom sorted) count via the flat predicate
        // so facets the tree ignores for pruning — notably `needs_only` — are
        // honored (an Inbox tab shows the awaiting-a-human count, not the farm
        // size). Mirrors `flat_rows`: exclude dead unless the spec asks, respect
        // the row cap.
        if v.is_flat() {
            let resolved = filter::resolved_ids(&self.all);
            let want_dead = v.spec.statuses.contains(&Status::Dead);
            let n = self
                .all
                .iter()
                .filter(|t| (want_dead || t.status != Status::Dead) && v.spec.matches(t, &resolved))
                .count();
            return v.limit.map_or(n, |l| n.min(l));
        }
        tree::build(&self.all, &v.spec, self.resolved_family_scope(v))
            .iter()
            .filter(|r| !r.ghost)
            .count()
    }

    /// Hairy tasks with at least one unresolved dependency (the blocked set;
    /// Python marks these with a magenta `*`).
    pub(crate) fn blocked_ids(&self) -> HashSet<String> {
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
    pub(crate) fn pinned_indices(&self) -> Vec<usize> {
        (0..self.views.len())
            .filter(|&i| self.views[i].pinned)
            .collect()
    }

    /// Rows the starred working set resolves to, in star order (flat).
    pub(crate) fn working_set_rows(&self) -> Vec<tree::Row<'_>> {
        self.working_set
            .iter()
            .filter_map(|id| self.task(id))
            .map(tree::Row::leaf)
            .collect()
    }

    /// Flat, sorted rows for a sorted view (Recent / custom sorted).
    pub(crate) fn flat_rows(&self) -> Vec<tree::Row<'_>> {
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
    pub(crate) fn rows(&self) -> Vec<tree::Row<'_>> {
        let v = self.active_view();
        if v.key == "working-set" {
            return self.working_set_rows();
        }
        if v.is_flat() {
            return self.flat_rows();
        }
        let scope = self.resolved_family_scope(v);
        let flat = tree::build(&self.all, &self.filter, scope);
        tree::apply_collapse(flat, &self.collapsed)
    }

    /// Keep the cursor within the current row count (after a filter change).
    pub(crate) fn clamp_cursor(&mut self) {
        let len = self.rows().len();
        self.cursor = if len == 0 {
            0
        } else {
            self.cursor.min(len - 1)
        };
    }

    pub(crate) fn selected(&self) -> Option<&Task> {
        self.rows().into_iter().nth(self.cursor).map(|r| r.task)
    }

    pub(crate) fn selected_id(&self) -> Option<String> {
        self.selected().map(|t| t.id.clone())
    }

    pub(crate) fn task(&self, id: &str) -> Option<&Task> {
        self.all.iter().find(|t| t.id == id)
    }

    /// Activate view `i`: load its saved spec into the live filter and reset.
    pub(crate) fn set_view(&mut self, i: usize) {
        self.view = i;
        self.filter = clone_spec(&self.views[i].spec);
        self.cursor = 0;
        self.detail_scroll = 0;
        self.focus = Focus::List;
        self.clamp_cursor();
    }

    /// Cycle through the pinned views (the visible tabs) only.
    pub(crate) fn switch_tab(&mut self, delta: i32) {
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
    pub(crate) fn is_view_modified(&self) -> bool {
        !spec_eq(&self.filter, &self.active_view().spec)
    }

    /// Esc: revert the live filter to the active view's saved spec.
    pub(crate) fn revert_filter_to_view(&mut self) {
        if self.is_view_modified() {
            let i = self.view;
            self.set_view(i);
        }
    }
}
