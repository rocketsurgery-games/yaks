---
id: yaksrs-a49c
title: attach / detach artifacts (clipboard PNG via arboard)
type: task
priority: 4
created: '2026-08-20T19:13:25Z'
updated: '2026-08-22T20:53:22Z'
parent: yaksrs-0a93
labels:
- rust
---

---
▸ 2026-08-21T01:49:29Z
Also fold TUI visual-yank here (select a range in the detail pane and copy it): it needs the same clipboard primitive (arboard) as attach/detach. Deferred with the rest of the clipboard work.

---
▸ 2026-08-22T20:53:22Z
Added artifacts: A attaches a file (or clipboard PNG when path blank) into .yaks/artifacts/{id}/ and appends ![stem](artifacts/{id}/{name}) to the body (herd.attach + AttachOutcome); O opens the artifact/URL on the current detail line via the OS opener (open/xdg-open). New Target::Artifact parsed from ![alt](path) in detail.rs (parse_image_link); follow_link/Enter opens URLs+artifacts externally now too. Added png crate + clipboard::read_png (arboard image -> PNG). Committed-to-git storage per decision. Tests: attach_file_writes_artifact_and_links_body; verified end-to-end via headless. Visual-yank half was delivered in c9eb. 113 unit + 19 CLI green, no new clippy. docs #15.
