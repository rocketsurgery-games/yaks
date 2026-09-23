---
id: yaks-095a
title: '`yaks merge` fails to update config for new herd'
type: bug
priority: 3
created: '2026-09-23T01:53:59Z'
updated: '2026-09-23T02:01:14Z'
---

When it copies yaks into the target farm, it should also update the config to include the new herd.

![merge-herd-evidence](artifacts/yaks-095a/merge-herd-evidence.txt)

---
▸ 2026-09-23T02:01:01Z [agent]
End-to-end lever: merging a web-herd farm into a core-herd farm now declares both herds in the destination config, and --herd web resolves. Transcript shows config before/after.

---
▸ 2026-09-23T02:01:08Z [agent]
Fixed: merge now registers incoming herds. store::declare_herds(root, herds) adds bare '<herd>:' entries to config.yaml's herds: map — insert-in-block when one exists (comments + per-herd overrides preserved byte-for-byte), else append a fresh block SEEDED with the farm's current known herds (config herd + on-disk prefixes) so materializing the block can't shrink the known-herd set. Idempotent (already-declared skipped). Farm::merge computes the incoming prefixes not already declared, reports them on MergePlan.herds (dry-run reports what it *would* declare and writes nothing), and calls declare_herds on apply; CLI prints 'declared herd(s): …'. Docs: cli.md merge row, skills merge row, clap --help. 4 new tests (269 lib green).

---
▸ 2026-09-23T02:01:10Z [agent]
verify: `cargo test --workspace` -> PASS (exit 0)
