---
id: yaks-451d
title: read_config drops verify/herds entries after an inline comment
type: bug
priority: 3
created: '2026-09-21T21:01:13Z'
updated: '2026-09-21T21:02:22Z'
labels:
- config
- cli
---

Surfaced while shearing the 15f7 herds family: running yaks verify on any yaks-herd yak errors with 'no config default for its labels', even though .yaks/config.yaml has a full verify: map. Root cause: read_config's line loop skips blank lines but not COMMENTS. A comment line inside the verify:/herds: block has no colon, so it hits the else branch that resets sec=Top, dropping every entry after the first comment. Fix: also continue on comment lines (trimmed starts with #) at the top of the loop, matching parse_pointer. Add a regression test with a commented verify block.

---
▸ 2026-09-21T21:02:03Z [agent]
verify: `cargo test -p yaks` -> PASS (exit 0)
