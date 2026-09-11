---
id: yaks-7a1c
title: Move ephemeral yellow status line to second row
type: bug
priority: 3
created: '2026-09-10T22:55:24Z'
updated: '2026-09-10T23:58:56Z'
labels:
- ui
needs: human
---

These status lines frequently overlap the tabs, but we also leave a blank line _below_ the tabs, which seems like a good fallback. Why don't we keep them in the usual spot, but drop them to the line below if they don't fit. This would have to be recomputed on viewport size change, obviously.

---
▸ 2026-09-10T23:47:07Z [coordinator]
LANE A (wt/statusline). Scope: src/tui.rs render_tabs (~4209) + render_status (~4728) ONLY; reuse existing App.notification (NO new App field). Behavior: keep the ephemeral status line in its usual spot, but when it would overlap the tab strip (does not fit on the tabs row), drop it to the blank line below the tabs; recompute on width. Evidence contract: SVG docshot showing the drop at a narrow width AND inline at a wide width; attach to this yak + 'yaks ask' for visual sign-off. JUDGE = human (subjective visual) -> do NOT self-shear; hand back.

![tui-status-below-tabs](artifacts/yaks-7a1c/tui-status-below-tabs.svg)

---
▸ 2026-09-10T23:57:21Z [lane-a]
narrow width: notification dropped below tabs

---
▸ 2026-09-10T23:58:56Z [lane-a]
Implemented drop-to-second-row. Scriptable snapshot proves narrow-vs-wide placement; SVG attached for visual sign-off. Please review + answer to approve.
