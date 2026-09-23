//! Terminal lifecycle + the event loop: `run` (the public entry point),
//! `event_loop`, the `.yaks/` filesystem watcher, and raw-mode `setup`/`restore`.
//! Split out of `tui.rs` (yaks-b1cc / b1cc/8). `use super::*` inherits `tui.rs`'s
//! scope (App, handle_key, render, the crossterm/notify imports, …).

use super::*;
use ratatui::crossterm::event::{DisableMouseCapture, EnableMouseCapture};
pub fn run(mut app: App) -> Result<()> {
    let (mut term, kitty) = setup()?;
    let res = event_loop(&mut term, &mut app);
    let _ = restore(kitty);
    res
}

fn event_loop(term: &mut Terminal<CrosstermBackend<Stdout>>, app: &mut App) -> Result<()> {
    // Best-effort filesystem watch on the farm's `.yaks/` tree so external edits
    // (including the user's own, from another process) refresh the UI. Kept
    // alive for the loop's duration; `_watcher` must not be dropped early.
    let watch_path = app.farm.as_ref().map(|h| h.root().to_path_buf());
    let (_watcher, rx) = setup_watcher(watch_path);
    let mut dirty = false;
    loop {
        // Apply external changes only while idle (no overlay), so we never yank
        // data out from under an open editor/picker mid-interaction.
        if dirty && matches!(app.overlay, Overlay::None) {
            app.reload_preserving_selection();
            dirty = false;
        }
        term.draw(|f| render(app, f))?;
        // Record the main-area height (minus tab + help lines) for paging.
        let h = term.size()?.height;
        app.page = h.saturating_sub(2).max(1);
        app.detail_page = h.saturating_sub(3).max(1);
        // Block for input, but wake periodically to service filesystem events.
        if event::poll(Duration::from_millis(250))? {
            match event::read()? {
                Event::Key(k) if k.kind == KeyEventKind::Press => handle_key(app, k),
                Event::Mouse(m) => app.handle_mouse(m),
                _ => {}
            }
        }
        // Coalesce any pending fs notifications into one deferred refresh.
        if let Some(rx) = &rx {
            while rx.try_recv().is_ok() {
                dirty = true;
            }
        }
        if app.quit {
            break;
        }
    }
    Ok(())
}

/// Set up a recursive watcher on `path`, returning the watcher (which the caller
/// must keep alive) and a receiver that yields once per content-changing event.
/// Best-effort: any failure yields `(None, None)` and the TUI just won't
/// auto-refresh. Access/metadata-only events are filtered out so that our own
/// reads can't trigger a reload feedback loop.
fn setup_watcher(path: Option<PathBuf>) -> (Option<RecommendedWatcher>, Option<Receiver<()>>) {
    let Some(path) = path else {
        return (None, None);
    };
    let (tx, rx) = mpsc::channel();
    let handler = move |res: notify::Result<notify::Event>| {
        if let Ok(ev) = res {
            let interesting = match ev.kind {
                EventKind::Create(_) | EventKind::Remove(_) => true,
                EventKind::Modify(ModifyKind::Metadata(_)) => false,
                EventKind::Modify(_) => true,
                _ => false, // Access / Any / Other
            };
            if interesting {
                let _ = tx.send(());
            }
        }
    };
    let mut watcher = match notify::recommended_watcher(handler) {
        Ok(w) => w,
        Err(_) => return (None, None),
    };
    if watcher.watch(&path, RecursiveMode::Recursive).is_err() {
        return (None, None);
    }
    (Some(watcher), Some(rx))
}

// -- terminal lifecycle ---------------------------------------------------

fn setup() -> Result<(Terminal<CrosstermBackend<Stdout>>, bool)> {
    enable_raw_mode()?;
    let mut out = io::stdout();
    execute!(out, EnterAlternateScreen)?;
    // Mouse capture (yaks-97a2): wheel + clicks. Most terminals still do native
    // text selection with Shift held while capture is on.
    let _ = execute!(out, EnableMouseCapture);
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
    let _ = execute!(out, DisableMouseCapture);
    execute!(out, LeaveAlternateScreen)?;
    disable_raw_mode()?;
    Ok(())
}
