# yaks-rs

A Rust port of [yaks](https://github.com/joelgwebber/yaks), the filesystem-native
task tracker. This repository is the **Phase 0 spike** (tracked as `yak-94e7`),
the outcome of the Rust-conversion research arc (`yak-2219`).

## Why a separate repo?

The port proceeds in a fresh repo rather than converting in place, so the Python
tree stays stable and shippable during the transition. The two implementations
**interoperate at the data layer** — both read and write the same `.yaks/`
directory (task files + a per-user derived index) — so they can coexist until
the Rust version reaches parity and we flip the canonical repo.

## Status: Phase 0 spike

Read-only commands over the existing on-disk format, to prove fs interop +
parsing and to measure cold-start latency against the Python baseline
(~42–48 ms per invocation, of which only ~2–5 ms is real work).

Implemented:

- `yaks list [--all]` — list active tasks (add `--all` to include shorn)
- `yaks show <id>` — show one task
- `yaks next` — hairy tasks whose dependencies are all resolved

All three discover the nearest `.yaks/` directory by walking up from the cwd,
exactly like the Python tool.

## Build & run

```sh
cargo build --release
./target/release/yaks list
```

## Measuring startup

```sh
cargo build --release
python3 bench/startup.py        # rust vs python; prefers hyperfine if installed
# or directly: hyperfine -N './target/release/yaks list'
```

Current numbers on the dev herd: Rust `yaks list` ~6 ms median vs Python ~45 ms.

## Roadmap (from the yak-2219 research)

| Phase | Scope | Key crates |
|-------|-------|-----------|
| 0 (here) | read-only CLI over existing files; startup benchmark; distribution skeleton | clap |
| 1 | full CLI parity (create/update/shave/shorn/dep/…), `--json`, byte-parity tests | clap, serde |
| 2 | TUI | ratatui + crossterm + edtui |
| 3 | demo/recording via rendered Buffer | avt |
| 4 | distribution cutover | cargo-dist + npm installer |

Storage stays fs-native: files authoritative, the index a derived, per-user,
rebuildable cache (rkyv zero-copy archive; no embedded DB). See the `yak-1622`
notes in the parent repo for the full rationale.
