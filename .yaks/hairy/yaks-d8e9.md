---
id: yaks-d8e9
title: skills install --force can clobber the repo's own skills/ source
type: bug
priority: 3
created: '2026-09-23T02:29:14Z'
updated: '2026-09-23T02:29:14Z'
labels:
- skills
- cli
---

Footgun that silently reverted skills/yaks/SKILL.md in this repo (lost the yaks-dd85 'Evidence before you shear' section for a day). Mechanism: the SKILL.md files are baked into the binary via include_str!, and 'yaks skills install --dir <path> --force' writes the baked-in copy over <path>/yaks/SKILL.md. Point that at THIS repo's skills/ dir with an older binary (e.g. a v0.0.9-era yaks on PATH) and it overwrites the newer source with the stale bundled copy -- a silent revert that looks like an authored edit in git status. Evidence: SKILL.md mtime Sep 22 22:00:31, binary rebuilt 22:00:48, working tree byte-identical to the pre-dd85 version. Also note cargo test -p yaks skills does NOT catch this: it only checks install writes/skips/forces into a temp dir, not that the on-disk skills/ source matches the embedded copy -- and after a rebuild the two agree again anyway, so the regression self-conceals. Options to consider: (a) refuse to install into the directory the binary was built from (or any path under a repo containing skills/yaks/SKILL.md) without an explicit override; (b) warn when the target file differs from the embedded copy and is NEWER; (c) a test/CI check that the shipped binary's embedded skill matches skills/ at HEAD.
