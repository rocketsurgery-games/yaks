---
id: yaks-5ef5
title: --json parity + byte-parity tests vs Python
type: task
priority: 2
created: '2026-08-20T03:26:09Z'
updated: '2026-08-20T19:35:03Z'
parent: yaks-6e21
depends_on:
- yaks-a2a4
- yaks-4e7f
- yaks-b315
labels:
- rust
---

---
▸ 2026-08-20T19:29:03Z
Starting. Decisions locked (owner): semantic correctness over byte-identical; golden fixtures we own (not vs live Python). Plan: serde_json (preserve_order) --json for list/show/next/tangled/search/stats/rollup with consistent task shapes; a committed fixture herd under tests/fixtures + insta snapshot tests via assert_cmd (also serves as the assert_cmd smoke tests from c725, leaving just hyperfine there).

---
▸ 2026-08-20T19:35:03Z
Done. json.rs: serde_json (preserve_order) shapes; --json on list/show/next/tangled/search/stats/rollup (+ rollup --keys --json). show gained Children (human) + children (json). Golden harness: tests/fixtures/herd + 17 insta snapshot tests via assert_cmd (also satisfies the assert_cmd smoke tests of yaks-c725; only hyperfine remains there). Semantic parity with Python verified on the live herd (list/stats/tangled/search/show/rollup equal; next intentionally includes status for consistency where Python omits it). Regenerate goldens with INSTA_UPDATE=always. 34 tests total.
