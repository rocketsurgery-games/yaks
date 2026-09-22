---
id: yaks-156d
title: Release v0.0.9
type: chore
priority: 3
created: '2026-09-22T00:35:59Z'
updated: '2026-09-22T00:37:13Z'
---

Cut v0.0.9: the multi-herd farm feature set (one .yaks/ holding several herds) plus follow-ups. Highlights since v0.0.8: multi-herd farms with per-yak --herd routing; yaks merge; out-of-tree .yaks pointer discovery; per-herd config cascade (global->herd); --herd filter; TUI per-herd id colour, create/edit herd picker, drawer herd chip row, H move key + detail Herd field; farm/herd/family vocabulary migration (--herd, rename-herd, config herd:); and a config-parser fix (comments inside verify:/herds: blocks).

---
▸ 2026-09-22T00:37:06Z [agent]
verify: `cargo test --workspace` -> PASS (exit 0)
