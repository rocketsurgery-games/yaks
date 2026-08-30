---
id: yaks-c9ee
title: 'Read side: recency and activity queries (notes/changes since T)'
type: feature
priority: 2
created: '2026-08-30T19:34:21Z'
updated: '2026-08-30T20:08:52Z'
parent: yaks-3901
labels:
- cli
---

A yaks log --since <ts|duration> [filters] that returns timestamped notes and state changes across a filtered set, with --json. Powers catch-up between Pi sessions and draining subagent reports in a rich harness, and makes check-before-starting cheap. Fold in yaks-953f, yaks-158d, yaks-71c2.

---
▸ 2026-08-30T20:08:52Z
Implemented yaks log [--since <spec>] [filters] [--json]. Core is pure/testable in store.rs: parse_notes() splits the timestamped note blocks append_note() writes; parse_since() accepts 2h/3d/1w durations, YYYY-MM-DD, naive datetime, or RFC3339; parse_ts() for cutoff compares. herd::log() applies the shared FilterSpec, keeps notes at/after the cutoff, sorts oldest-first. Evidence: 3 new unit tests pass (parse_notes x2, parse_since); full suite green (cargo test --workspace); live run 'yaks log --since 2h --parent-of yaps-3901' returned the two seeded notes, --json shape verified, 'yaks log --since banana' exits 1 with a clear message. Not done: state-transition timestamps (transitions are not individually recorded) -> spun out as a follow-up.
