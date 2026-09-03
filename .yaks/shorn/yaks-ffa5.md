---
id: yaks-ffa5
title: TUI auto-attributes comments to the current actor
type: feature
priority: 3
created: '2026-09-03T17:59:19Z'
updated: '2026-09-03T18:26:54Z'
parent: yaks-594b
depends_on:
- yaks-1603
- yaks-1edb
labels:
- ui
---

tui.rs EditAction::Comment builds TaskEdit { note, ..Default } with actor=None, so human TUI comments are unattributed while CLI ones are. Wire the comment path to resolve the actor (git user / $YAKS_ACTOR) so both surfaces match. Depends on a shared resolve_actor (see extract yak) and on the TUI comment parser understanding actor (yaks-1edb) so it renders correctly.

---
▸ 2026-09-03T18:26:54Z [coordinator]
BUILT. TUI EditAction::Comment now sets actor: crate::actor::resolve(None) ($YAKS_ACTOR else git user; no --as in the TUI), so TUI comments are attributed like CLI ones — closing the gap Joel hit ('yaks tui' comment on yaks-012b had no provenance). Resolved lazily in the handler (not cached on App) so the read-only/snapshot constructor stays git-free/deterministic. Path is the same append_note-with-actor already covered by CLI e2e + content.rs round-trip; 209 tests green. LIVE-TUI verification deferred: a fresh 'yaks tui' comment will now carry [actor] and render 'comment · <date> · <actor>' (1edb).
