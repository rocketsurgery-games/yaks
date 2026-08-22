//! Adapter: implement [`toque::HeadlessApp`] for the yaks [`App`], so the TUI
//! can be driven headlessly (agent exploration + snapshot tests). All the
//! generic machinery — the stdin protocol, the style encoders, the stable-id
//! registry — lives in the `toque` crate; this file only wires the yaks types
//! into that seam.
//!
//! This adapter lives in a child module of `tui`, so it can reach `App`'s
//! private fields (`page`, `detail_page`, `quit`) and private methods without
//! widening their visibility.

use ratatui::Frame;
use ratatui::crossterm::event::KeyEvent;
use toque::HeadlessApp;

use super::{App, handle_key, render};

impl HeadlessApp for App {
    fn render(&self, frame: &mut Frame) {
        render(self, frame);
    }

    fn handle_key(&mut self, key: KeyEvent) {
        handle_key(self, key);
    }

    /// Derive the paging basis from the viewport instead of having the driver
    /// mutate app state (the live loop uses the same arithmetic): the main area
    /// is the height minus the tab + status lines, and the detail viewport also
    /// drops the blank gap line.
    fn on_resize(&mut self, _width: u16, height: u16) {
        self.page = height.saturating_sub(2).max(1);
        self.detail_page = height.saturating_sub(3).max(1);
    }

    fn state_header(&self) -> String {
        App::state_header(self)
    }

    fn should_quit(&self) -> bool {
        self.quit
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::{Status, Task};
    use toque::{DriverOpts, Session, StyleEncoding};

    fn sample_task(id: &str, title: &str) -> Task {
        Task {
            id: id.into(),
            title: title.into(),
            kind: "task".into(),
            priority: 3,
            status: Status::Hairy,
            created: None,
            updated: None,
            parent: None,
            labels: vec![],
            depends_on: vec![],
            source: None,
            body: String::new(),
        }
    }

    fn drive(script: &[&str], style: Option<StyleEncoding>) -> String {
        let app = App::new(vec![
            sample_task("a0", "Root A"),
            sample_task("a1", "Child A1"),
        ]);
        let mut s = Session::new(
            app,
            DriverOpts {
                width: 60,
                height: 10,
                style,
                diff: false,
            },
        );
        let mut out: Vec<u8> = Vec::new();
        s.emit(&mut out).unwrap();
        for line in script {
            s.step(line, &mut out).unwrap();
        }
        String::from_utf8(out).unwrap()
    }

    #[test]
    fn snapshot_has_header_and_grid() {
        let out = drive(&["key j"], None);
        // Two frames (initial + after j); each framed with a state header.
        assert_eq!(out.matches("=== frame ").count(), 2);
        assert!(out.contains("focus=list"));
        assert!(out.contains("cursor=0")); // initial frame
        assert!(out.contains("cursor=1")); // after moving down
        assert!(out.contains("Root A"));
    }

    #[test]
    fn style_layer_emitted_and_aligned() {
        let out = drive(&[], Some(StyleEncoding::Parallel));
        assert!(out.contains("--- styles ---"));
        assert!(out.contains("legend:"));
        assert!(out.contains("default"));
    }

    #[test]
    fn spans_encoding_has_legend_and_no_style_grid() {
        let out = drive(&[], Some(StyleEncoding::Spans));
        assert!(out.contains("legend:"));
        assert!(!out.contains("--- styles ---"));
    }

    #[test]
    fn interleaved_encoding_has_legend_and_no_style_grid() {
        let out = drive(&[], Some(StyleEncoding::Interleaved));
        assert!(out.contains("legend:"));
        assert!(!out.contains("--- styles ---"));
    }

    #[test]
    fn paging_basis_tracks_viewport_height() {
        // on_resize is called by Session::new; the app should have derived its
        // page size from the height (10 - 2 = 8, detail 10 - 3 = 7).
        let app = App::new(vec![sample_task("a0", "Root A")]);
        let s = Session::new(
            app,
            DriverOpts {
                width: 60,
                height: 10,
                style: None,
                diff: false,
            },
        );
        assert_eq!(s.app().page, 8);
        assert_eq!(s.app().detail_page, 7);
    }

    #[test]
    fn diff_mode_full_then_delta() {
        let app = App::new(vec![
            sample_task("a0", "Root A"),
            sample_task("a1", "Child A1"),
        ]);
        let mut s = Session::new(
            app,
            DriverOpts {
                width: 50,
                height: 8,
                style: None,
                diff: true,
            },
        );
        let mut out: Vec<u8> = Vec::new();
        s.emit(&mut out).unwrap(); // frame 0: full
        s.step("key S", &mut out).unwrap(); // opens the state picker -> bottom line changes
        let s = String::from_utf8(out).unwrap();
        assert!(s.contains(" · full · "));
        assert!(s.contains(" · diff · "));
        assert!(s.contains("\nL")); // at least one changed-line label
    }
}
