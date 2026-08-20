---
id: yaksrs-a2a4
title: create command
type: task
priority: 2
created: '2026-08-20T03:26:09Z'
updated: '2026-08-20T03:38:00Z'
parent: yaksrs-6e21
labels:
- rust
depends_on:
- yaksrs-6b8c
---

---
▸ 2026-08-20T03:35:44Z
Starting. Wiring the create command onto store::write::save(): read prefix/default_type/default_priority from .yaks/config.yaml; collision-checked id generation ({prefix}-{4 lowercase hex}, matching Python); parent-exists validation; created/updated stamped via chrono UTC now_iso(); write to hairy/. Also removes the allow(dead_code) from store::write now that it is used.

---
▸ 2026-08-20T03:38:00Z
Done. create wired onto store::write::save: reads prefix/default_type/default_priority from .yaks/config.yaml; collision-checked id gen ({prefix}-{4 lowercase hex}) via a small hand-rolled xorshift (no rand dep); parent-exists validation; created/updated stamped via chrono UTC. Removed allow(dead_code) from store::write. CROSS-TOOL INTEROP PROVEN: Rust created yaksrs-35b6 and the Python yaks (on PATH) reads it correctly via show + list. Dogfooding spawned yaksrs-35b6 (CI interop test) as a real follow-up under the parity yak yaksrs-5ef5.
