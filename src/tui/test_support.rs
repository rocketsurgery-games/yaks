//! Shared test helpers for the tui module and its submodules.
//!
//! As `tui.rs` is split into submodules (yaks-b1cc), each submodule's
//! `#[cfg(test)] mod tests` reuses these via `use crate::tui::test_support::*`.
//! Kept `pub(crate)` and compiled only under `cfg(test)` (the `mod test_support`
//! declaration in `tui.rs` is `#[cfg(test)]`), so nothing ships in release.

use super::{App, handle_key, render};
use crate::herd::Herd;
use crate::model::{Status, Task};
use crate::store::{self, SCHEMA};
use ratatui::Terminal;
use ratatui::backend::TestBackend;
use ratatui::crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use std::fs;
use std::path::PathBuf;
use std::sync::atomic::{AtomicU64, Ordering};

pub(crate) fn task(
    id: &str,
    title: &str,
    status: Status,
    priority: u8,
    parent: Option<&str>,
) -> Task {
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
        needs: None,
        verify: None,
        extra: Vec::new(),
        body: String::new(),
    }
}

pub(crate) fn buffer_to_string(buf: &ratatui::buffer::Buffer) -> String {
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

pub(crate) fn draw(app: &App, w: u16, h: u16) -> String {
    let mut term = Terminal::new(TestBackend::new(w, h)).unwrap();
    term.draw(|f| render(app, f)).unwrap();
    buffer_to_string(term.backend().buffer())
}

pub(crate) fn key(c: char) -> KeyEvent {
    KeyEvent::new(KeyCode::Char(c), KeyModifiers::NONE)
}

pub(crate) fn enter_key(app: &mut App) {
    handle_key(app, KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE));
}

pub(crate) fn esc_key(app: &mut App) {
    handle_key(app, KeyEvent::new(KeyCode::Esc, KeyModifiers::NONE));
}

pub(crate) fn down_key(app: &mut App) {
    handle_key(app, KeyEvent::new(KeyCode::Down, KeyModifiers::NONE));
}

pub(crate) fn tab_key(app: &mut App) {
    handle_key(app, KeyEvent::new(KeyCode::Tab, KeyModifiers::NONE));
}

pub(crate) fn sample() -> App {
    // root-a (hairy) with children a1 (hairy) + a2 (shorn, ghost in Hairy tab);
    // root-b (shaving, not in Hairy universe).
    App::new(vec![
        task("a0", "Root A", Status::Hairy, 2, None),
        task("a1", "Child A1", Status::Hairy, 3, Some("a0")),
        task("a2", "Child A2 done", Status::Shorn, 3, Some("a0")),
        task("b0", "Root B shaving", Status::Shaving, 1, None),
    ])
}

pub(crate) fn editable() -> App {
    let mut t = task("e0", "Editable", Status::Hairy, 3, None);
    t.labels = vec!["rust".into(), "tui".into()];
    t.body = "First line.\nSecond line.".into();
    App::new(vec![t])
}

pub(crate) fn linked() -> App {
    let mut a0 = task("a0", "Root A", Status::Hairy, 2, None);
    a0.body = "follow a1 then a2".into();
    App::new(vec![
        a0,
        task("a1", "Child A1", Status::Hairy, 3, Some("a0")),
        task("a2", "Child A2", Status::Hairy, 3, Some("a0")),
    ])
}

static SEQ: AtomicU64 = AtomicU64::new(0);

/// A temp project dir containing a `.yaks/` herd seeded with `tasks`.
pub(crate) fn temp_herd(tasks: &[Task]) -> (PathBuf, Herd) {
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

pub(crate) fn press(app: &mut App, chars: &str) {
    for c in chars.chars() {
        handle_key(app, key(c));
    }
}
