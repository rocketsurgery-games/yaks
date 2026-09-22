# AGENTS.md — yaks

yaks is a filesystem-native task tracker: a single self-contained Rust binary.
Tasks are markdown files with YAML frontmatter under a `.yaks/` directory; a
task's status is implicit in which subdirectory it lives in.

## Invariants

- **Files are authoritative.** Status is implicit from the directory a task file
  lives in (`hairy/ shaving/ shorn/ dead/`). Parentage is a frontmatter `parent:`
  field; ids are flat and stable (any legacy dotted ids are inert — the `parent:`
  field is the only source of hierarchy).
- **Any index is derived.** If a lookup index is added, it must be a per-user,
  rebuildable cache — never committed, never a second source of truth.
- **Don't change the on-disk format lightly.** The `.yaks/` layout is the
  contract; task files are meant to be readable, greppable, and diffable.

## Farms, herds, families

- A **farm** is one `.yaks/` directory (what `Farm` opens). A **herd** is a group
  of yaks sharing an id prefix within a farm; a farm may hold several (config
  `herds:` map; `create --herd`), so one private farm can track several
  projects. A **family** is a parent yak + its descendants (the TUI tree /
  `view::FamilyScope`).
- **Per-herd config cascades one level:** `default_type` / `default_priority` /
  `verify` resolve herd-value-else-global (`store::Config::{default_type_for,
  default_priority_for, resolve_verify}`, `known_herds`). Add new per-herd
  settings through that same cascade.
- **A repo can point at an out-of-tree farm** via a `.yaks` pointer file
  (`path:` + optional `herd:`) or a symlink, resolved in `store::discover`, so
  several repos share one farm with no env var. `create` routes a new yak's
  herd as: explicit `--herd` > pointer `herd:` > config default.
- **Consolidate** separate farms with `Farm::merge` (`yaks merge`), which is
  collision-checked and non-destructive.

## Layout

A Cargo workspace: the root is the `yaks` binary package *and* the workspace
root; reusable pieces live under `crates/`.

- `src/model.rs` — `Status`, `Task`.
- `src/store.rs` — `.yaks/` discovery + frontmatter parsing (hand-rolled fast path).
- `src/main.rs` — clap CLI + command dispatch.
- `src/farm.rs` — the core facade the CLI and TUI both call.
- `src/tui.rs` (+ `src/tui/`) — the interactive TUI. `tui.rs` is a thin root (the
  `App` struct, the `Focus`/`Overlay`/`PickAction`/`ConfirmAction` enums, the
  constructors, and shared form helpers/consts); behavior lives in focused
  submodules (split out under yaks-b1cc): `render` (all `render_*` + shared render
  helpers), `editor` (edtui glue), `drawer`/`create`/`fuzzy` (overlay forms +
  pickers), `viewmodel`/`detail_nav`/`actions`/`handlers` (the `impl App` split by
  concern, each an `impl App` block using `use super::*`), and `runtime`
  (`run`/event loop/terminal setup) — alongside the existing
  `detail`/`tree`/`content`/`markdown`/`view`/`views_store`/`cache` models +
  persistence. `src/tui/headless.rs` is a thin adapter implementing
  `toque::HeadlessApp` for `App`; `src/tui/docshots.rs` renders doc SVGs. The
  whole-frame integration tests + their insta snapshots are central in
  `src/tui/tests.rs` + `src/tui/snapshots/`; shared test helpers live in
  `src/tui/test_support.rs`.
- `crates/toque/` — publishable library: drive any ratatui app headlessly
  (inject keys, capture LLM-/test-legible plain-text snapshots, and render frames
  to SVG for visual inspection). yaks is its first consumer. See
  `crates/toque/README.md`; `docs/research/tui-style-eval.md` archives the
  now-retired text style-encoding research.

## Build

```sh
cargo build --release
cargo test --workspace   # --workspace is required: `cargo test` alone only
                         # tests the root `yaks` package, not crates/toque
```

## Verifying changes

Verify against the real artifact via the project's lever, not "tests pass"
alone. A TUI change is verified by driving `toque` to the relevant state and
LOOKING at the rendered frame (and via `cargo test -p yaks docshots --
--ignored`, which renders color SVG frames to `docs/assets/`, plus the `insta`
snapshots in `src/tui.rs`). If a change has no lever to see its effect,
`yaks ask` rather than declare it done on faith.

Don't keep that frame to yourself: `yaks attach` the rendered frame (or a
headless text serialization) to the yak as evidence, so a reviewer sees what you
saw — the shipped `skills/yaks` "Evidence before you shear" rule, applied here.
For a durable UI state, prefer a `docshots` scene + a `docs/` embed over a
one-off frame.

To actually *look* at an SVG frame, rasterize it to PNG with headless Chrome
(recipe in `docs/tui.md`) — yaks stays out of rasterization, and a pure-Rust
rasterizer botches the color emoji, so a browser engine is the lever.

## Keeping docs current

A user-facing change is not done until its docs **and its own in-app help** are.
When you add or change a CLI command or flag, a TUI key or behavior, or a
workflow, update every surface that describes it, in the **same** change — never
a follow-up:

- `docs/` (`docs/cli.md`, `docs/tui.md`, `docs/README.md`) and `README.md`.
- the bundled skills (`skills/yaks`, `skills/yaks-tracker`).
- for a CLI change: the clap `--help` text — the `///` doc comments and
  `#[arg(...)]`/`#[command(...)]` help on the command/flag in `src/main.rs`.
- for a TUI key/behavior: the `?` help overlay (`help_content` in
  `src/tui/render.rs`) **and**, if the key is common enough to belong there, the
  compact bottom help bar (`help_hint`). A new key that lands in the handler but
  not in `help_content` is invisible to users (this is how yaks-71d1's `H`
  shipped half-done).

The bundled skills are embedded in the binary (`src/skills.rs`), so
`cargo test -p yaks skills` guards them; the other surfaces have no gate, so
treat docs/help↔reality parity as part of the change's evidence — grep for the
old name/key across `docs/`, `skills/`, `README.md`, and `src/` and confirm every
description matches the shipped behavior.

## Releasing

`.github/workflows/release.yml` builds the 5-platform binaries and publishes the
`@rocketsurgery/yaks` npm packages on a `vX.Y.Z` tag (dry-run on manual
dispatch). See `RELEASING.md`.

## Task tracking

This project uses yaks to track its own work. The yaks skill has the full
workflow.

1. Never start coding without a shaving yak. No exceptions.
2. Shear a yak as soon as its work is done, and commit the shorn yak file
   alongside the code that completed it (`.yaks/` is committed here — team mode).
3. Check existing yaks before creating new ones.
4. Append progress notes to yak descriptions as you work.
5. When unsure what's next, run `yaks next` — don't freelance.
