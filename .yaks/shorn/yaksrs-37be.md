---
id: yaksrs-37be
title: 'edtui PR: bind r (replace char) via char_arg'
type: feature
priority: 2
created: '2026-08-24T22:11:59Z'
updated: '2026-08-24T22:34:04Z'
parent: yaksrs-6099
labels:
- ui
- edtui
- upstream
source: https://github.com/preiter93/edtui/pull/71
---

Upstream probe PR. ReplaceChar(pub char) is in the Action enum but unbound because it has no char_arg (unlike f/t). Wire char_arg into ReplaceChar and bind r in vim normal mode. Smallest, low-controversy change; use it to gauge maintainer responsiveness. Our yaks-side r-prefix in route_multiline_key is the reference impl. Requires a GitHub fork of preiter93/edtui to push + open the PR.

---
▸ 2026-08-24T22:34:03Z
Implemented + opened PR preiter93/edtui#71 from branch yaks/replace-char-r on the fork. Change: ReplaceChar(char) -> ReplaceChar(Option<char>) + char_arg (mirrors FindForward); bind r in vim normal mode; r<char> replaces, r<Esc> cancels, . repeats; undo preserved. Fixed README (r was mislabeled 'redo'; redo is ctrl+r). Added test_replace_char_keybinding; cargo test --lib green (150). yaks NOT yet switched to the fork — integration/shim-removal is tracked in yaksrs-182f (batch once more PRs land). Built/tested edtui with the 1.95 toolchain since the repo's dev-deps need rustc>=1.88 (yaks pins stable=1.87).
