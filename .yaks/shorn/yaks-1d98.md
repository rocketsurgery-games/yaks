---
id: yaks-1d98
title: discover an out-of-tree farm (symlink now, pointer file next)
type: feature
priority: 2
created: '2026-09-21T17:32:32Z'
updated: '2026-09-21T18:17:45Z'
parent: yaks-15f7
labels:
- cli
---

For private farms living OUTSIDE the code repos: discover_root walks up from cwd for a .yaks dir, so an external farm is not found from inside a repo. Phase 1 (zero code): a gitignored .yaks symlink in each repo pointing at the external farm; is_dir follows symlinks so discovery already resolves it, and .git/info/exclude keeps it fully private. Phase 2 (small feature): teach discovery that .yaks may be a pointer file carrying path (the farm) and prefix (this repo herd) so agents need not pass --prefix nor rely on env vars, which are unreliable inside embedded agent runners. Document the symlink recipe regardless.

---
▸ 2026-09-21T18:17:45Z [agent]
SHORN. Out-of-tree farm discovery via a pointer file (phase 2), plus the symlink recipe (phase 1) documented. store::discover(start) now returns Discovered { root, prefix }: at each level up from cwd the .yaks entry resolves as a directory (or symlink to one) = the farm root, OR a regular FILE = a pointer with 'path:' (absolute, ~/-relative, or relative to the pointer dir; accepts a .yaks dir or a dir containing one) and optional 'prefix:' (this repo's default herd). discover_root removed (only caller was a test; now discover().root). Farm gained pointer_prefix; create precedence is explicit --prefix > pointer prefix > config default (all validated via the reference grammar). Because the shared farm's root is the cache/views slug key, all repos pointing at it share one TUI-state cache. No env var needed - the very thing that was fragile in embedded agent runners. Tests: store discover-follows-pointer + create-routes-via-pointer(+explicit wins); suite green (250 lib + 25 cli), warning-free. Verified on scratch: two repos (.yaks pointer files, prefix web/api) over one external core farm - create with no flag routed web-/api- into the shared farm, list from either repo showed both herds. Docs: README + docs/README + the yaks skill got a 'several repos, one farm' recipe (pointer file + symlink + yaks merge). Deferred: a 'yaks link' convenience to write the pointer (hand-authoring 2 lines is trivial).
