---
id: yaks-a09b
title: Rename attachment from TUI
type: feature
priority: 3
created: '2026-08-23T18:28:32Z'
updated: '2026-09-23T21:44:50Z'
labels:
- ui
---

They very often start as just "pasted ..." when created through the TUI. But there's no trivial way to rename them without touching the file and yak contents manually.

---
▸ 2026-09-23T21:35:38Z [coordinator]
Lightning-round lane 'attach' (worktree wt/attach, branch lane-attach). Scope: TUI rename attachment (with yaks-fe19). Evidence contract: gate = cargo test -p yaks green (re-accept + eyeball any snapshot ripple); PLUS drive the headless TUI (toque) to the relevant state and yaks attach the frame (text, or docshot SVG when color matters). Judge: coordinator at merge review; subjective forks -> yaks ask and hand back.

![rename-attachment-tui-frames](artifacts/yaks-a09b/rename-attachment-tui-frames.txt)

---
▸ 2026-09-23T21:44:40Z [lane-attach]
Headless TUI (yaks tui --headless, scratch farm): l, Tab onto image link, r -> 'Rename dashboard.png to (ext kept if omitted):' prompt, type signin, Enter -> 'renamed dashboard.png → signin.png', link rewritten in detail.

---
▸ 2026-09-23T21:44:50Z [lane-attach]
Done. Detail pane 'r' on an attachment's image link (Tab/[ ] to it) opens 'Rename <old> to (ext kept if omitted):'; Enter renames via Farm::rename_attachment (file moved, links rewritten farm-wide), empty cancels; off-link r just notifies. help_content + docs/tui.md updated (not in the compact bar — niche key). Test: tui rename_attachment_from_detail_link; headless frames attached.
