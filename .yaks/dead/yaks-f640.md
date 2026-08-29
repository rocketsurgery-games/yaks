---
id: yaks-f640
title: Enable md-syntax by default once edtui is C-free
type: task
priority: 3
created: '2026-08-24T22:19:46Z'
updated: '2026-08-25T03:26:31Z'
parent: yaks-6099
depends_on:
- yaks-d635
labels:
- ui
- edtui
---

Once the fancy-regex edtui change lands (and yaks points at it), flip md-syntax on by default (or fold into default features) so shipped binaries get markdown coloring. Verify the release matrix still builds with 0 C deps.

---
▸ 2026-08-25T03:26:31Z
Obviated by yaks-2956 (md-syntax feature is being removed, not enabled by default).
