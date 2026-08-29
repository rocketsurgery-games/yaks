---
id: yaks-7e59
title: 'edtui PR: W/B/E big-WORD motions'
type: feature
priority: 3
created: '2026-08-24T22:12:00Z'
updated: '2026-08-25T03:52:21Z'
parent: yaks-6099
labels:
- ui
- edtui
- upstream
source: https://github.com/preiter93/edtui/pull/73
---

edtui has WORD delete/change (dW/cW/diW) but no WORD *motion*. Factor a big-word scan into MoveBigWordForward/Backward/End actions + bind W/B/E. Reference impl: yaks big_word_forward/backward(). Requires GitHub fork.

---
▸ 2026-08-25T03:41:14Z
Implemented + opened PR preiter93/edtui#73 from branch yaks/big-word-motions. Added MoveBigWordForward/Backward/ForwardToEndOfWord (W/B/E), bound in normal+visual. Mirror the word motions with a whitespace-vs-not classifier (big_word_class), matching the existing delete_big_word_* boundary logic. Added test_big_word_motions; cargo test --lib green (150). Built with 1.95 toolchain.

---
▸ 2026-08-25T03:52:21Z
Upstream PR withdrawn/closed (opened prematurely). Implementation is preserved on its fork branch for the user to review; re-upstreaming will go issues-first. Kept shorn since the code work is complete.
