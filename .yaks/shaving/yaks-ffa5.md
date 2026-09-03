---
id: yaks-ffa5
title: TUI auto-attributes comments to the current actor
type: feature
priority: 3
created: '2026-09-03T17:59:19Z'
updated: '2026-09-03T18:25:26Z'
parent: yaks-594b
depends_on:
- yaks-1603
- yaks-1edb
labels:
- ui
---

tui.rs EditAction::Comment builds TaskEdit { note, ..Default } with actor=None, so human TUI comments are unattributed while CLI ones are. Wire the comment path to resolve the actor (git user / $YAKS_ACTOR) so both surfaces match. Depends on a shared resolve_actor (see extract yak) and on the TUI comment parser understanding actor (yaks-1edb) so it renders correctly.
