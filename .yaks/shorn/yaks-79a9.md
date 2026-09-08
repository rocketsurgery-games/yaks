---
id: yaks-79a9
title: Evidence over assertion -> rerunnable artifact
type: task
priority: 3
created: '2026-09-08T00:50:52Z'
updated: '2026-09-08T03:33:56Z'
parent: yaks-a2ca
labels:
- skills,eval
verify: cargo test -p yaks doctor
---

Push 'evidence over assertion' (yaks-working) toward pstack's 'the tool is the artifact a reviewer can rerun.' Prefer an evidence note that carries a RERUNNABLE command / test name / the project's verification-lever invocation, not a narrative self-report. Cheap skill nudge now (strengthen yaks-working; if a project has its own verification lever a la pstack's create-verification-skill CLI+Feature Map, point the evidence step at it). Candidate mechanization later: doctor --strict could check a shorn yak's evidence note contains a command-shaped token, a step up from 'has any note'. Ties to pstack build-the-lever + encode-lessons-in-structure + prove-it-works.

---
▸ 2026-09-08T01:32:49Z [coordinator]
Cross-ref yaks-6624 (verification-lever discipline): this is the MECHANIZED BACKSTOP for that thread — strengthening doctor --strict so evidence is a rerunnable-command/seen-artifact (not just 'has a note') is what turns the shear gate into a real bullshit-caller for 'declared done without seeing output'. Pairs with 6624/139b's skill rule (verify-or-ask) and 6624/c3b1's per-repo lever registry.

---
▸ 2026-09-08T03:29:25Z [coordinator]
[accept] Done = doctor --strict gains a check: a SHORN yak that has a verify: command must have a recorded verify PASS as its MOST RECENT verify note (else flag 'shorn with a verify: command but last verify was not a PASS'). Opt-in by design: only yaks that set verify: are held to it; qualitative/research yaks without verify: fall under the existing 'has a note' strict check. Honest limit: catches sheared-with-failing/never-run verify, NOT verified-then-changed-then-sheared (that needs a re-run, deliberately explicit). Also fold the skill rule into yaks-working/yaks-coordinating: coordinator sets the judge at claim via verify:/ask; worker doesn't self-shear a contract it can't fully script. Evidence: new unit test name(s); cargo test --workspace green; 'yaks verify yaks-79a9' (cargo test -p yaks doctor) PASS; a live doctor --strict run flagging a shorn+verify+FAIL yak then going clean after a PASS. Judge: coordinator (scriptable). Scope: herd.rs (doctor), skills/dev/*.

---
▸ 2026-09-08T03:33:14Z [coordinator]
verify: `cargo test -p yaks doctor` -> PASS (exit 0)

---
▸ 2026-09-08T03:33:56Z [coordinator]
Shorn. doctor --strict now flags a SHORN yak whose verify: command's most recent recorded run was not a PASS (new IssueKind::UnverifiedShear + last_verify_passed helper in herd.rs). Opt-in: only yaks that set verify: are held to it. Honest limit noted: catches sheared-over-failing/never-run verify, not verify-then-edit-then-shear (needs a re-run). Test: herd::tests::doctor_strict_flags_shorn_yak_whose_verify_did_not_pass. cargo test --workspace GREEN (236+25+13). yaks verify yaks-79a9 (cargo test -p yaks doctor) -> PASS. LIVE: real herd shows 0 UnverifiedShear (d4d2/d954 last-PASS -> clean); a scratch shorn+verify:false was flagged with an actionable message, stayed flagged after a FAIL run, and went clean after a PASS. Docs+skills updated in the same change (cli.md, yaks skill, --strict help; yaks-working got verify: + 'don't self-shear what you can't judge'; yaks-coordinating claim beat states the contract+judge; no judge: field per f814).
