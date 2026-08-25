---
id: yaksrs-182f
title: 'Integrate merged edtui features: git-dep, then swap to release + delete yaks shims'
type: task
priority: 3
created: '2026-08-24T22:12:19Z'
updated: '2026-08-24T22:12:19Z'
parent: yaksrs-6099
depends_on:
- yaksrs-37be
- yaksrs-b3a0
- yaksrs-7e59
- yaksrs-cdf9
- yaksrs-2a15
- yaksrs-d168
- yaksrs-114a
labels:
- ui
- edtui
- upstream
---

As upstream features land: point yaks Cargo.toml at the fork branch (git dep is fine — yaks ships prebuilt npm binaries, not a crates.io publish), then swap back to the crates.io release once merged. Delete the corresponding yaks-side shims in route_multiline_key / display_line_nav / editor_theme as their features become native. Depends on the upstream PRs.
