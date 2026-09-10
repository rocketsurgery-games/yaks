---
id: yaks-6c93
title: 'Reconcile the ui verify lever: snapshot gate vs docshot generator'
type: task
priority: 2
created: '2026-09-10T23:45:35Z'
updated: '2026-09-10T23:45:35Z'
labels:
- meta
---

config.yaml sets verify.ui = cargo test -p yaks docshots -- --ignored, but that target is #[ignore]d and asset-PRODUCING (SVG generation), not a pass/fail gate. The actual gating channel is plain cargo test -p yaks (insta snapshots). Since doctor --strict enforces that a shorn yak's verify: last PASSed, a ui yak that runs only the generator gets weak evidence. Decide whether verify.ui should run the snapshot gate, the visual generator, or a combined command -- aligning with the two-channel model (text=gate, SVG=visual).
