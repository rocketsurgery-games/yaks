---
id: yaks-9ccc
title: 'Bulk state transitions: accept multiple ids'
type: feature
priority: 2
created: '2026-08-23T03:43:29Z'
updated: '2026-09-02T23:31:53Z'
parent: yaks-8d53
labels:
- cli
---

shave/shorn/regrow/slaughter/revive accept multiple ids in one call, e.g. yaks shorn a b c. Motivation: shearing and creating whole herds one id at a time was tedious across recent sessions.

---
▸ 2026-09-02T23:31:48Z
Implemented: shave/shorn/regrow/slaughter/revive now take Vec<String> ids (clap required, num_args=1..). New transition_many() loops per id via the existing single-id transition() path, processes ALL ids (no abort-on-first-error), prints per-id result line, exits non-zero if any id NotFound. transition() helper now returns Result<bool> instead of exiting inline. Single-id usage unchanged. Updated clap help text for the five commands. Added herd.rs test transition_batch_moves_valid_ids_and_flags_missing (all-good + partial-failure). Evidence -- cargo test --workspace: yaks crate 203 passed 0 failed; cli 20 passed; toque 13 passed; 1 doc-test. Real binary: 'yaks shorn yaks-33b5 yaks-a15a' -> both 'Shorn!', exit 0, both in shorn/. Partial: 'yaks regrow yaks-33b5 yaks-zzzz' -> 'Regrown: yaks-33b5' + 'error: task yaks-zzzz not found', exit 1, good id moved. Throwaways deleted; herd left as found. Touched src/main.rs, src/herd.rs only.
