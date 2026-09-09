---
id: yaks-4d6d
title: Cut v0.0.7 release (verify/attach/doctor --strict, coordination skills)
type: task
priority: 2
created: '2026-09-09T00:58:59Z'
updated: '2026-09-09T01:00:11Z'
labels:
- release
---

Cut v0.0.7. Since v0.0.6 (which lacked them) this ships: yaks verify + Task.verify field, yaks attach (external evidence under .yaks/artifacts/), doctor --strict UnverifiedShear, the config verify: map, plus updated shipped skills (yaks command table, yaks-tracker team+tracker discouragement). Ritual (RELEASING.md): bump Cargo.toml + npm/yaks/package.json to 0.0.7 (lockstep enforced by the release workflow), refresh Cargo.lock, commit, tag v0.0.7, push main + tag -> release.yml matrix-builds 5 binaries and publishes the 6 npm packages via OIDC.

---
▸ 2026-09-09T01:00:11Z [coordinator]
Verified: 'cargo build --release' compiles as yaks v0.0.7; 'cargo test --workspace' ALL_GREEN (238 unit + 25 cli + 13 toque + 1 doctest, 0 failed). Version lockstep set — Cargo.toml + npm/yaks/package.json = 0.0.7, matching tag v0.0.7; Cargo.lock refreshed by the release build.
