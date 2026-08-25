---
id: yaksrs-7e59
title: 'edtui PR: W/B/E big-WORD motions'
type: feature
priority: 3
created: '2026-08-24T22:12:00Z'
updated: '2026-08-24T22:12:00Z'
parent: yaksrs-6099
labels:
- ui
- edtui
- upstream
---

edtui has WORD delete/change (dW/cW/diW) but no WORD *motion*. Factor a big-word scan into MoveBigWordForward/Backward/End actions + bind W/B/E. Reference impl: yaks big_word_forward/backward(). Requires GitHub fork.
