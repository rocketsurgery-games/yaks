---
id: yaksrs-c725
title: Smoke tests (assert_cmd) + in-repo hyperfine bench
type: task
priority: 2
created: '2026-08-20T03:26:09Z'
updated: '2026-08-20T19:43:14Z'
parent: yaksrs-26c0
labels:
- rust
---

---
▸ 2026-08-20T19:42:46Z
assert_cmd smoke tests already landed via the snapshot harness (tests/cli.rs, 5ef5). Remaining here: an in-repo startup bench. hyperfine not installed, so bench/startup.py prefers hyperfine when present and otherwise uses a perf_counter loop; compares the Rust release binary vs the Python yaks (if on PATH).

---
▸ 2026-08-20T19:43:14Z
Done. bench/startup.py: portable startup bench (prefers hyperfine, falls back to a perf_counter loop), Rust release vs Python yaks. Current dev-herd numbers: rust yaks list ~6ms median vs python ~45ms (~7x). README updated to point at it. The assert_cmd smoke-test half of this yak already shipped as the snapshot harness (tests/cli.rs) in yaksrs-5ef5.
