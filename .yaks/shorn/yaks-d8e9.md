---
id: yaks-d8e9
title: skills install --force can clobber the repo's own skills/ source
type: bug
priority: 3
created: '2026-09-23T02:29:14Z'
updated: '2026-09-23T04:41:48Z'
parent: yaks-1d51
labels:
- skills
- cli
---

Footgun that silently reverted skills/yaks/SKILL.md in this repo (lost the yaks-dd85 'Evidence before you shear' section for a day). Mechanism: the SKILL.md files are baked into the binary via include_str!, and 'yaks skills install --dir <path> --force' writes the baked-in copy over <path>/yaks/SKILL.md. Point that at THIS repo's skills/ dir with an older binary (e.g. a v0.0.9-era yaks on PATH) and it overwrites the newer source with the stale bundled copy -- a silent revert that looks like an authored edit in git status. Evidence: SKILL.md mtime Sep 22 22:00:31, binary rebuilt 22:00:48, working tree byte-identical to the pre-dd85 version. Also note cargo test -p yaks skills does NOT catch this: it only checks install writes/skips/forces into a temp dir, not that the on-disk skills/ source matches the embedded copy -- and after a rebuild the two agree again anyway, so the regression self-conceals. Options to consider: (a) refuse to install into the directory the binary was built from (or any path under a repo containing skills/yaks/SKILL.md) without an explicit override; (b) warn when the target file differs from the embedded copy and is NEWER; (c) a test/CI check that the shipped binary's embedded skill matches skills/ at HEAD.

---
▸ 2026-09-23T04:41:32Z [agent]
Fixed, and the root cause was NOT what I first diagnosed. The real mechanism: ~/.agents/skills/yaks and ~/.agents/skills/yaks-tracker are SYMLINKS into this repo (-> ../../src/rs/yaks/skills/...), a handy dev setup for running the live skill. So a plain 'yaks skills install' with no --dir at all resolves through the link onto the repo source and rewrites it with the binary's baked-in copy. That is why the working tree showed an exact revert to the v0.0.9 bundled content and looked like an authored edit. My first guess (someone pointed --force at the repo skills dir) was wrong about the target; the symlink did it.

Proof it was live: my own 'cargo test' runs re-stamped skills/*/SKILL.md repeatedly, because the integration tests spawn the binary for ordinary commands, startup auto-sync fired, and it wrote to the real ~/.agents/skills -> through the symlinks -> into the source.

Fixes: (1) is_source_tree now CANONICALIZES the target and checks every ancestor, so it catches a symlinked entry, not just a literal repo path; (2) a new SkillState::SourceLinked, checked before anything else, is never written -- force or not -- and surfaces in skills status/install as 'source'; (3) the --force hint is suppressed for it, since force genuinely cannot override it; (4) tests/cli.rs now sets YAKS_SKILLS_AUTOSYNC=0 so the suite can never mutate a developer's real skills; (5) a test asserts the EMBEDDED source is never itself stamped, which is the invariant that catches recurrence loudly; (6) a dedicated symlink_guard_tests module reproduces the exact production failure. Verified on the real setup: install and auto-sync both decline, repo source stays pristine.

---
▸ 2026-09-23T04:41:33Z [agent]
verify: `cargo test -p yaks skills` -> PASS (exit 0)
