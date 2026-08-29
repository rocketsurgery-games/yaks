---
id: yaks-182f
title: 'Integrate merged edtui features: git-dep, then swap to release + delete yaks shims'
type: task
priority: 3
created: '2026-08-24T22:12:19Z'
updated: '2026-08-25T23:25:00Z'
depends_on:
- yaks-37be
- yaks-b3a0
- yaks-7e59
- yaks-cdf9
- yaks-2a15
- yaks-d168
- yaks-114a
labels:
- ui
- edtui
- upstream
---

As upstream features land: point yaks Cargo.toml at the fork branch (git dep is fine — yaks ships prebuilt npm binaries, not a crates.io publish), then swap back to the crates.io release once merged. Delete the corresponding yaks-side shims in route_multiline_key / display_line_nav / editor_theme as their features become native. Depends on the upstream PRs.

---
▸ 2026-08-25T22:16:23Z
yaks now consumes the fork via git dep: edtui = git rocketsurgery-games/edtui branch yaks-integration (merges r + counts + W/B/E; made W/B/E count-aware on the integration branch). Public fork -> CI fetches it fine; .edtui is just the local authoring checkout; Cargo.lock pins the commit. MSRV OK on the local toolchain. Removed now-redundant yaks-side shims (r, W, B) since edtui handles them natively AND the shims broke count composition (a shim that handles a key without forwarding leaves edtui's pending count dangling -> it would leak onto the next command). Fixed toggle_case/r ReplaceChar(Some(..)) for the new signature. Tests green. RESIDUAL: the shims that remain (g-prefix gj/gk/g0/g$, ~, X) still leak a pending count if you prefix them (3gj, 3~, 3X) because they don't forward to edtui. x-yank forwards x so it's fine. Endgame: implement ~, x-yank, and gj/gk (needs wrap exposure) natively in the fork and delete ALL yaks shims, so counts compose everywhere.

---
▸ 2026-08-25T22:47:29Z
Endgame done: ported the remaining shims (~, x/X-yank, gj/gk/g0/g$) into the fork on feature branches, merged all into yaks-integration (+ made the new count-bearing actions count-aware), pushed. cargo update -p edtui. DELETED all remaining yaks-side vim shims: route_multiline_key, display_line_nav/wrap_segments/GMotion/row_chars/max_col_for/toggle_case, Editor.pending/wrap_width, CreateForm.desc_pending/desc_wrap_width, the DeleteChar/ReplaceChar import, and the shim-internal tests. route now just forwards to edtui. Count-leak gone (everything native). yaks tests green (120), 0 warnings. Remaining under this yak: swap back to a crates.io release once features land upstream.
