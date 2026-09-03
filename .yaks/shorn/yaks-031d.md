---
id: yaks-031d
title: Preserve unknown/unmodeled frontmatter fields across a read->write round-trip
type: bug
priority: 2
created: '2026-09-03T18:02:05Z'
updated: '2026-09-03T18:10:43Z'
parent: yaks-3901
labels:
- store
- ui
---

ROOT CAUSE (store-level, not TUI-specific): store::parse has a '_ => {}' catch-all, so any frontmatter key not in the Task struct is dropped at parse time; the canonical writer only re-emits known fields. => ANY mutation that rewrites a file (update, ask/answer, reparent, rename, create, and every TUI edit) silently strips unmodeled keys. Pure status transitions are safe (they rename the file without rewriting).

WHY IT MATTERS: (1) Forward-compat -- an older binary editing a herd written by a NEWER yaks (with a field it doesn't model) strips that field; 'files are authoritative' breaks across versions. (2) Team mode -- teammates on different versions clobber each other's newer fields. (3) Custom/experimental keys hand-added to frontmatter get nuked on first mutation.

RELATION TO THE SCHEMA GATE: there is already a schema-version guard (SchemaStatus::Newer -> OpenError::SchemaTooNew refuses to open a herd whose schema is newer). That is the coarse protection (bump schema on additions; old binaries refuse rather than corrupt). But not every field addition bumps the schema (e.g. 'needs' likely didn't), and the gate is all-or-nothing per herd; ad-hoc/unknown keys within a compatible schema still get dropped. Preservation is graceful defense-in-depth. Decide how the two relate.

FIX SKETCH (do not over-prescribe): capture unknown keys into a preserved bag on Task (e.g. an ordered Vec<(key, raw-value)>) and re-emit them on write. Hard parts: placement in canonical output (after known fields, original relative order), inline vs block-style values, and quoting/escaping fidelity (aim byte-stable). Even a conservative 'preserve scalar keys verbatim' pass beats silent loss.

The user surfaced this from the TUI editing angle; it also underpins yaks-1edb (comment/actor round-trip) and the whole yaks-594b cluster -- the TUI is just where it is most likely to bite. Add a round-trip test: parse a body with an unknown key, save, assert the key survives.

---
▸ 2026-09-03T18:10:43Z [coordinator]
BUILT + verified. Task.extra: Vec<String> captures unmodeled frontmatter lines verbatim on parse (scalar keys AND block-list keys via a pending='extra' state) and the writer re-emits them after known fields. Files with no unknown keys are byte-unchanged (extra empty). Verified: hand-added 'assignee: alice' + a 'reviewers:' block on a throwaway survived a 'yaks update' mutation while the new attributed note landed. 206 tests green incl. unknown_frontmatter_survives_a_round_trip (scalar + block, render->reparse equal). Relates to schema gate (SchemaTooNew): that stays the coarse per-herd guard; this is graceful per-field defense-in-depth for non-schema-bumped/custom keys. Makes editing safe for the whole 594b cluster.
