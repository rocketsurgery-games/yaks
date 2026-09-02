---
id: yaks-61a9
title: Crash on shift-tabbing back from desc to labels
type: bug
priority: 1
created: '2026-08-30T19:03:40Z'
updated: '2026-08-30T19:03:40Z'
labels:
- ui
- crash
---

Here's the panic trace:

```
thread 'main' panicked at /Users/joel/.cargo/git/checkouts/edtui-d79fc0b17f7a57af/9fbee79/src/events/key/input.rs:146:18:
not implemented
stack backtrace:
   0: __rustc::rust_begin_unwind
             at /rustc/17067e9ac6d7ecb70e50f92c1944e545188d2359/library/std/src/panicking.rs:697:5
   1: core::panicking::panic_fmt
             at /rustc/17067e9ac6d7ecb70e50f92c1944e545188d2359/library/core/src/panicking.rs:75:14
   2: core::panicking::panic
             at /rustc/17067e9ac6d7ecb70e50f92c1944e545188d2359/library/core/src/panicking.rs:145:5
   3: <edtui::events::key::input::KeyCode as core::convert::From<crossterm::event::KeyCode>>::from
             at /Users/joel/.cargo/git/checkouts/edtui-d79fc0b17f7a57af/9fbee79/src/events/key/input.rs:146:18
   4: <T as core::convert::Into<U>>::into
             at /Users/joel/.rustup/toolchains/stable-aarch64-apple-darwin/lib/rustlib/src/rust/library/core/src/convert/mod.rs:761:9
   5: <edtui::events::key::input::KeyInput as core::convert::From<crossterm::event::KeyEvent>>::from
             at /Users/joel/.cargo/git/checkouts/edtui-d79fc0b17f7a57af/9fbee79/src/events/key/input.rs:220:18
   6: <T as core::convert::Into<U>>::into
             at /Users/joel/.rustup/toolchains/stable-aarch64-apple-darwin/lib/rustlib/src/rust/library/core/src/convert/mod.rs:761:9
   7: edtui::events::EditorEventHandler::on_key_event
             at /Users/joel/.cargo/git/checkouts/edtui-d79fc0b17f7a57af/9fbee79/src/events/mod.rs:70:35
   8: yaks::tui::App::handle_create_key
             at ./src/tui.rs:1568:21
   9: yaks::tui::App::handle_overlay_key
             at ./src/tui.rs:2469:13
  10: yaks::tui::handle_key
             at ./src/tui.rs:3029:9
  11: yaks::tui::event_loop
             at ./src/tui.rs:2974:21
  12: yaks::tui::run
             at ./src/tui.rs:2946:15
  13: yaks::main
             at ./src/main.rs:552:17
  14: core::ops::function::FnOnce::call_once
             at /Users/joel/.rustup/toolchains/stable-aarch64-apple-darwin/lib/rustlib/src/rust/library/core/src/ops/function.rs:250:5
note: Some details are omitted, run with `RUST_BACKTRACE=full` for a verbose backtrace.
```
