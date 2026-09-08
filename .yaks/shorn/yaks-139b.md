---
id: yaks-139b
title: 'yaks-working: sharpen prove-it-works + ask-when-no-lever'
type: task
priority: 2
created: '2026-09-08T01:32:42Z'
updated: '2026-09-08T01:54:45Z'
parent: yaks-6624
labels:
- skills,eval
---

Encode the core rule in yaks-working (the per-yak verification discipline). Before shearing you must verify against the REAL ARTIFACT using the project's verification lever (drive the TUI/browser, reproduce the state, read the actual value) — 'it compiles'/self-report/a green proxy is not evidence (pstack prove-it-works). AND: if the project has NO lever for this kind of change (a TUI you can't snapshot, UI you can't drive, state you can't reproduce), do NOT shear on faith — 'yaks ask' the human whether to go build one, and/or create a 'build a lever for X' yak. The needs-block makes it unbypassable; the inbox makes it visible. Keep it minimal — a sharpened evidence rule + the ask escalation, not a framework.

---
▸ 2026-09-08T01:52:22Z [coordinator]
[accept] Done = yaks-working gains a rule in its existing terse voice (NO new framework/bloat): (1) author an evidence contract / definition-of-done BEFORE work (coordinator, or self in solo mode); (2) before shearing, verify against the REAL ARTIFACT via the project's lever, not 'it compiles'/self-report/a green proxy; (3) if NO lever exists for this kind of change, 'yaks ask' the human rather than shear on faith. Stay consistent with 6624/f814: evidence generalized beyond code 'correctness' to 'outcome achieved+seen, appropriate to the yak kind'; dead exempt. Evidence to attach: quote the exact added lines in a note + confirm no contradiction with the existing 'Before you shear' bullets. Scope: skills/dev/yaks-working/SKILL.md ONLY.

---
▸ 2026-09-08T01:54:42Z [worker-skill]
Added to '## Before you shear' in skills/dev/yaks-working/SKILL.md. Exact added lines:
- **Evidence contract first.** The definition-of-done — what will count as proof — is authored *before* the work: by the coordinator at claim time, or by the working agent itself in solo mode. You shear against it, not against vibes.
Extended existing 'Evidence over assertion' bullet to: Do not shear on 'it compiles', a self-report, or a green proxy. Verify against the *real artifact* via the project's verification lever — drive the TUI/browser, reproduce the state, read the actual value — and record it in a note (...). Evidence is general: the outcome achieved and seen, appropriate to the yak's kind (a research/decision yak's evidence is its finding/rationale; dead yaks are exempt).
- **No lever? Ask, don't shear on faith.** If the project has no way to verify this kind of change (a TUI you can't snapshot, UI you can't drive, state you can't reproduce), 'yaks ask <id>' the human whether to go build one rather than shearing on trust.
No contradiction with existing 'Before you shear' bullets: the contract/lever/ask bullets sharpen (not replace) the original Evidence-over-assertion bullet, which I extended in place rather than duplicating; the shorn-summary, team-mode-commit, and commit-hooks bullets are untouched and remain consistent.
