---
id: yaks-1d98
title: discover an out-of-tree farm (symlink now, pointer file next)
type: feature
priority: 2
created: '2026-09-21T17:32:32Z'
updated: '2026-09-21T17:32:32Z'
parent: yaks-15f7
labels:
- cli
---

For private farms living OUTSIDE the code repos: discover_root walks up from cwd for a .yaks dir, so an external farm is not found from inside a repo. Phase 1 (zero code): a gitignored .yaks symlink in each repo pointing at the external farm; is_dir follows symlinks so discovery already resolves it, and .git/info/exclude keeps it fully private. Phase 2 (small feature): teach discovery that .yaks may be a pointer file carrying path (the farm) and prefix (this repo herd) so agents need not pass --prefix nor rely on env vars, which are unreliable inside embedded agent runners. Document the symlink recipe regardless.
