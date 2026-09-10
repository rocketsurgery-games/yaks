---
id: yaks-13f3
title: 'Coordinating skill: scope by function/region when one file is unavoidable'
type: task
priority: 3
created: '2026-09-10T23:45:35Z'
updated: '2026-09-10T23:45:58Z'
labels:
- meta
- skills
---

Recon for the parallel UI batch (yaks-7a1c/yaks-26f0/yaks-f207) hit the wall that src/tui.rs is a 7K-line monolith holding nearly every render fn and key handler, so the yaks-coordinating SOP of disjoint FILES is impossible. Add guidance: when one giant file is unavoidable, scope lanes by function/region within it; run the shared-type scan first to find the collision point -- here App has exactly ONE literal construction (App::new), so any lane adding an App field collides there; prefer yaks needing no new shared-type fields; and anchor each lane's new tests at a distinct line range so the shared test module merges cleanly.
