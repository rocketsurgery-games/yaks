---
id: yaks-d4d2
title: 'yaks verify: per-yak verify command + runner (automated evidence)'
type: feature
priority: 2
created: '2026-09-08T02:07:57Z'
updated: '2026-09-08T02:17:09Z'
parent: yaks-6624
labels:
- eval,cli
verify: cargo test -p yaks verify
---

The automation core of yaks-6624/f814: a yak carries its own RERUNNABLE verification command, so evidence stops being prose and becomes a check anyone can re-run. (1) optional 'verify:' frontmatter field on Task, round-trip-preserved; (2) settable via create --verify / update --verify; (3) 'yaks verify <id>...' runs the stored command, streams output, reports PASS/FAIL by exit code, and records an attributed evidence note. c3b1 (per-repo lever registry) will later supply the DEFAULT verify command per change-type; this is the runner it feeds. Safety: verify is EXPLICIT (never auto-run by doctor) — running your own herd's command is like a Makefile target.

---
▸ 2026-09-08T02:08:14Z [coordinator]
[accept] Done = (1) optional 'verify:' frontmatter field on Task, preserved byte-stable on read/write round-trip; (2) settable via 'update --verify <cmd>' (and create --verify if cheap); (3) 'yaks verify <id>...' runs the stored command via the shell, streams its output, exits/returns PASS iff exit 0, and records an attributed note 'verify: <cmd> -> PASS/FAIL (exit N)'; a yak with no verify: command errors cleanly. Evidence to attach (use the lever on itself): the unit-test names covering field round-trip + the runner, cargo test --workspace GREEN, AND a live 'yaks verify' run on a scratch yak showing PASS recorded as a note. Scope: model.rs, store.rs, herd.rs, main.rs (+ tests).

---
▸ 2026-09-08T02:15:37Z [coordinator]
verify: `cargo test -p yaks verify_field_round_trips run_verify_command_reports_pass_and_fail` -> FAIL (exit 1)

---
▸ 2026-09-08T02:16:15Z [coordinator]
verify: `cargo test -p yaks verify` -> PASS (exit 0)

---
▸ 2026-09-08T02:17:09Z [coordinator]
Shorn. Delivered: Task.verify field (round-trips, byte-stable — test store::verify_field_round_trips); create/update --verify to set/clear; 'yaks verify <id>...' runs the command via sh -c with live stdio, records 'verify: <cmd> -> PASS/FAIL (exit N)' as an attributed note, exits non-zero on any FAIL; a yak with no verify: errors cleanly + nonzero (confirmed on yaks-6624). Runner extracted as run_verify_command (test tests::run_verify_command_reports_pass_and_fail). cargo test --workspace GREEN (235 unit + 25 cli + 13 toque). LIVE (used the lever on ITSELF): d4d2's verify: = 'cargo test -p yaks verify' -> PASS (exit 0) recorded; a scratch yak with verify:false -> FAIL (exit 1) recorded + command exited nonzero. d4d2 keeps its verify: so it stays re-runnable. Safety: explicit only, never auto-run by doctor. Next in 6624: c3b1 (config supplies default verify per change-type); doctor --strict could require a recorded verify-PASS before shear (79a9).
