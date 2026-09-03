---
id: yaks-1edb
title: TUI comment parser must understand the [actor] stamp (unify with store::parse_notes)
type: bug
priority: 2
created: '2026-09-03T17:59:04Z'
updated: '2026-09-03T18:25:09Z'
parent: yaks-594b
labels:
- ui
---

eb66 changed the note marker line to '▸ <ts> [actor]', but the TUI has its OWN parser (tui/content.rs parse/assemble) that predates it and folds the whole line into BlockKind::Comment.timestamp. Effects: the actor is glued into the 'timestamp' string; render_block_separator shows only timestamp.get(..10) so the actor is invisible; and edit/reassemble round-trips actor as part of ts (fragile). Fix: give Block a first-class actor (parse it like store::parse_notes; ideally share one parser), render it in the separator (e.g. 'comment · <date> · <actor>'), and preserve it correctly through assemble on edit. Load-bearing correctness gap.

---
▸ 2026-09-03T18:25:09Z [coordinator]
BUILT. Shared the marker primitives from store (note_head(ts,actor) formats '<ts> [actor]'; split_note_head now pub) so the log writer and the TUI comment (re)assembly can't drift. content.rs BlockKind::Comment gains actor: Option<String>, parsed via store::split_note_head and re-emitted via store::note_head; render_block_separator shows 'comment · <date> · <actor>'. Fixes the latent bug where the actor was glued into the timestamp string (invisible + fragile on edit). Test parses_and_round_trips_the_actor_stamp (parse + byte-identical assemble). 209 tests green. Unblocks yaks-ffa5 (auto-attributed TUI comments) and yaks-548b.
