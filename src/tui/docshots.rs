//! Doc "screenshots": drive the headless [`App`] to a handful of representative
//! states and write each to a self-contained **color SVG** under `docs/assets/`
//! for embedding in `docs/tui.md`.
//!
//! This is not a normal test — it is a code-gated asset generator. It lives in a
//! `#[cfg(test)]` child module of `tui` (like `headless`) so it can construct an
//! `App` and reach the same private drive path the snapshot tests use, without a
//! `lib` target (yaks is a bin crate, so an `examples/` file cannot `use` it).
//!
//! The buffer→SVG rendering itself lives in [`toque::render_to_svg`]; this module
//! only picks the states worth capturing.
//!
//! Regenerate the assets with:
//!
//! ```sh
//! cargo test -p yaks docshots -- --ignored
//! ```

use ratatui::crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use toque::HeadlessApp;

use crate::model::{Status, Task};
use crate::tui::{App, handle_key};

// -- sample herd ----------------------------------------------------------

fn t(id: &str, title: &str, kind: &str, pri: u8, status: Status, parent: Option<&str>) -> Task {
    Task {
        id: id.into(),
        title: title.into(),
        kind: kind.into(),
        priority: pri,
        status,
        created: Some("2026-09-01T10:00:00Z".into()),
        updated: Some("2026-09-06T12:00:00Z".into()),
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

/// A small, realistic herd: an umbrella with children across statuses, labels,
/// a dependency, and one child awaiting a human (the ⏳ needs badge).
fn herd() -> Vec<Task> {
    let mut coord = t(
        "yaks-3901",
        "Coordination substrate for parallel yak-shaving",
        "task",
        1,
        Status::Shaving,
        None,
    );
    coord.labels = vec!["coord".into(), "git".into()];

    let mut inbox = t(
        "yaks-b517",
        "HITL: needs-human frontmatter + inbox surfacing",
        "feature",
        2,
        Status::Shaving,
        Some("yaks-3901"),
    );
    inbox.labels = vec!["cli".into(), "ui".into()];
    inbox.needs = Some("human".into());
    inbox.body = "Add a `needs:` block agents raise via `ask` and a human clears via \
                  `answer`; surface blocked yaks in an Inbox view.\n\n\
                  ---\n▸ 2026-09-06T11:40:00Z [wtA]\nProposal drafted — see the drawer \
                  `inbox` chip. Which glyph should mark a blocked row?\n\n\
                  ---\n▸ 2026-09-06T12:00:00Z [joel]\nGo with the hourglass; ship it."
        .into();

    let mut diff = t(
        "yaks-70e5",
        "yaks diff <refA> <refB>: ref-generic herd diff",
        "feature",
        3,
        Status::Hairy,
        Some("yaks-3901"),
    );
    diff.labels = vec!["cli".into()];
    diff.depends_on = vec!["yaks-b517".into()];

    let mut shot = t(
        "yaks-3677",
        "TUI doc screenshots via headless buffer->SVG",
        "feature",
        3,
        Status::Shaving,
        Some("yaks-3901"),
    );
    shot.labels = vec!["ui".into(), "docs".into()];
    shot.body = "Walk the ratatui Buffer's per-cell symbol+fg/bg into an SVG grid so \
                 docs render in color."
        .into();

    let done = t(
        "yaks-865d",
        "Normalize skill names to yaks[-*]",
        "task",
        2,
        Status::Shorn,
        Some("yaks-3901"),
    );

    let mut solo = t(
        "yaks-10fb",
        "Coordination follow-ups (post-3901)",
        "idea",
        3,
        Status::Hairy,
        None,
    );
    solo.labels = vec!["agent".into()];

    vec![coord, inbox, diff, shot, done, solo]
}

fn app_at(w: u16, h: u16) -> App {
    let mut app = App::new(herd());
    HeadlessApp::on_resize(&mut app, w, h);
    app
}

fn press(app: &mut App, c: char) {
    handle_key(app, KeyEvent::new(KeyCode::Char(c), KeyModifiers::NONE));
}

fn press_key(app: &mut App, code: KeyCode) {
    handle_key(app, KeyEvent::new(code, KeyModifiers::NONE));
}

/// Switch to a built-in view by its stable key (e.g. `status:shaving`, `inbox`).
fn view_to(app: &mut App, key: &str) {
    let i = app
        .views
        .iter()
        .position(|v| v.key == key)
        .unwrap_or_else(|| panic!("view {key:?} present"));
    app.set_view(i);
}

fn write_svg(name: &str, app: &App, w: u16, h: u16) {
    let svg = toque::render_to_svg(app, w, h);
    let path = format!("docs/assets/{name}.svg");
    std::fs::create_dir_all("docs/assets").unwrap();
    std::fs::write(&path, svg).unwrap();
    eprintln!("wrote {path}");
}

/// Regenerate every SVG in `docs/assets/`. Ignored by default; run explicitly:
/// `cargo test -p yaks docshots -- --ignored`.
#[test]
#[ignore = "asset generator; run with --ignored to refresh docs/assets/*.svg"]
fn docshots() {
    let (w, h) = (96u16, 20u16);

    // 1. List view (tree): the Shaving tab shows the umbrella + children across
    //    statuses, labels, a dependency, and the ⏳ needs badge on the HITL yak.
    let mut app = app_at(w, h);
    view_to(&mut app, "status:shaving");
    write_svg("tui-list", &app, w, h);

    // 2. Detail pane: open the HITL yak (only member of the Inbox), showing the
    //    Needs: field and attributed [wtA]/[joel] notes. Taller so the
    //    description + both comments fit under the metadata block.
    let dh = 34u16;
    let mut app = app_at(w, dh);
    view_to(&mut app, "inbox");
    press_key(&mut app, KeyCode::Enter); // open detail on the sole inbox row
    write_svg("tui-detail", &app, w, dh);

    // 3. Inbox view: flat list of every yak awaiting a human, across statuses.
    let mut app = app_at(w, h);
    view_to(&mut app, "inbox");
    write_svg("tui-inbox", &app, w, h);

    // 4. Filter drawer: the chip facets (status/type/priority/deps incl. inbox).
    let mut app = app_at(w, h);
    press(&mut app, 'f');
    write_svg("tui-drawer", &app, w, h);
}
