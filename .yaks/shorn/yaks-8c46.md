---
id: yaks-8c46
title: Consistency + error pass over all skills
type: task
priority: 2
created: '2026-09-06T23:02:22Z'
updated: '2026-09-06T23:44:37Z'
parent: yaks-012b
labels:
- skills
---

After docs: read all skills (yaks, yaks-tracker, yaks-working, yaks-coordinating) carefully for errors + inconsistencies with each other and with intended design. Fix.

---
▸ 2026-09-06T23:44:37Z [coordinator]
Consistency pass complete. Fixed 4 issues found by reading all 4 skills + cross-checking every CLI claim against --help: (1) yaks-tracker x2 stale 'the `yak` skill' -> '`yaks` skill' (missed in the 865d rename); (2) yaks-working attribution section described '--as' as a future candidate primitive though it's shipped -> rewrote to document the real flag + resolution chain; (3) yaks-coordinating referenced 'yaks commits --follow' but there is no --follow flag (commits already follows the file across status moves by default) -> dropped it; (4) yaks skill --json list omitted log+doctor (both support it) -> added. Verified: log --since/--by, list/bulk --needs, inbox/show/next/tangled/search/stats/rollup/log/doctor --json, doctor --strict, create positional title, rename/rename-prefix --dry-run, --parent-of, init --emacs all exist. Internal yak refs (yaks-2610, yaks-3901) exist. cargo test skills green; yaks doctor clean.
