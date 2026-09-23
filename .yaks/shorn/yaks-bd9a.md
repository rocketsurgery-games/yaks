---
id: yaks-bd9a
title: Skill update and validation
type: feature
priority: 3
created: '2026-09-08T02:26:00Z'
updated: '2026-09-23T04:41:48Z'
parent: yaks-1d51
labels:
- skills
---

When the `yaks` CLI tool is updated, locally-installed skills can still be out of date. It would be good for `yaks` to validate them somehow, and require (or at least recommend) that they be upgraded before continuing.

This is a little tricky, because they could in theory be installed just about anywhere. Maybe we just have a simple `yak skills upgrade` tool. But it would be nice if the tool could at least detect stale skills in the standard location.

Bonus points if we can find a way to get agents to reliably pass some kind of skill version/hash to `yaks` that we can validate.

---
▸ 2026-09-23T04:41:44Z [agent]
Delivered. Installed skills now carry a provenance stamp in their frontmatter under the spec's metadata: field (yaks-version + yaks-digest, quoted strings per the spec's string-values rule), which is what makes staleness decidable at all. inspect() yields seven states: current / stale / adoptable / held / modified / unmanaged / source.

Validation and upgrade: 'yaks skills status' reports the verdict per skill (with --dir for other skills dirs), and 'yaks skills install' upgrades a cleanly-stale copy with no flag while refusing to touch a locally-edited one without --force. yaks doctor adds an advisory listing only the ones a human must decide about (it sits outside Farm::doctor, since this is environment, not farm integrity, and must not affect its exit code).

Answering the 'bonus points' item -- getting agents to reliably pass a version/hash -- we do not need a new agent-facing protocol: ordinary 'yaks ...' invocations auto-sync at startup (absent or cleanly-stale only). Guardrails: user-level ~/.agents/skills only (never project-local, which would write into someone's repo), atomic temp+rename for parallel agents, YAKS_SKILLS_AUTOSYNC=0 opt-out, skipped for init/skills subcommands, and never a downgrade.

Two deliberate consequences: (a) auto-upgrade keys on version, so a skills-only edit reaches users at the next release -- that is what makes two same-version binaries unable to ping-pong; use --force when iterating locally. (b) Pre-stamp installs are handled by the 'adoptable' state: an unstamped file that is byte-identical to ours is stamped without --force, so the existing installed base migrates itself instead of sitting unmanaged forever.

---
▸ 2026-09-23T04:41:44Z [agent]
verify: `cargo test -p yaks skills` -> PASS (exit 0)
