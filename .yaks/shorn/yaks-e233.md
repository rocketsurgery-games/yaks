---
id: yaks-e233
title: Shipped skills use a non-spec frontmatter key (activation:), breaking portability
type: bug
priority: 3
created: '2026-09-23T03:04:32Z'
updated: '2026-09-23T04:25:37Z'
parent: yaks-1d51
labels:
- skills
---

The Agent Skills spec allows exactly six frontmatter fields outside Claude Code: name, description, license, compatibility, metadata, allowed-tools (https://code.claude.com/docs/en/skills - 'Using skill frontmatter outside Claude Code'). Claude Code itself silently ignores unknown keys, so this is invisible locally -- but claude.ai uploads, the Skills API, and packaging with anthropics/skills package_skill.py FAIL WITH A HARD ERROR on an unexpected key: 'Unexpected key(s) in SKILL.md frontmatter: ... Allowed properties are: allowed-tools, compatibility, description, license, metadata, name'. Both shipped skills (skills/yaks, skills/yaks-tracker) declare an 'activation:' list, which is not a spec field, so neither can be packaged or uploaded to claude.ai as-is. Fix: drop activation: (the description already carries the activation cue, which is what Claude actually matches on) or move it under the free-form metadata: map. Check any dev/ skills too.

---
▸ 2026-09-23T04:02:02Z [agent]
VERIFIED against the canonical spec (https://agentskills.io/specification.md), not just the Claude Code docs. The spec's frontmatter table is exactly: name (req), description (req), license, compatibility, metadata, allowed-tools (experimental). 'activation' is absent. It is also absent from Claude Code's much longer superset table and from Zed's (name/description/disable-model-invocation). Empirical check of all 20 skills in ~/.agents/skills: 'activation:' appears in exactly 7 -- yaks, yaks-tracker, sightmap-authoring, sightmap-browser, sightkick-authoring, sightkick-debug, fsta-sightmap-browser -- i.e. ONLY Joel's own two project families. Zero third-party skills use it (changesets, find-skills, github-stacked-prs, all 7 subtext-*). Conclusion: a locally-invented convention that spread by copy-paste, not a real (or formerly real) spec field. Cannot definitively rule out some early unreleased preview, but there is no trace in any current authority. NOTE the spec ships an official validator: 'skills-ref validate ./my-skill' (https://github.com/agentskills/agentskills/tree/main/skills-ref) -- it would have caught this, and is a candidate CI gate. Fix stands: drop activation: (description already carries the cue) or move it under metadata:. Also affects the sightmap/sightkick skills in Joel's other repos.

---
▸ 2026-09-23T04:25:28Z [agent]
Dropped the non-spec activation: key from both bundled skills. For yaks-tracker its four trigger bullets were folded into the description (which is what agents actually match on), so no activation signal was lost. Added skills::SPEC_FRONTMATTER_KEYS (the spec's six allowed keys) plus a test, bundled_skills_use_only_spec_frontmatter_keys, that rejects any non-spec top-level key, requires name+description, and enforces the spec rule that name must match the skill's directory. Chose a built-in test over shelling out to the external skills-ref validator: keeps yaks self-contained and runs in the existing suite. Also sharpened the README install section with the verified facts: .agents is the cross-client convention, Claude Code needs --dir since it does not scan .agents, and openskills stays (42k npm downloads last month, not a leftover) but is cited as an installer rather than a path authority.

---
▸ 2026-09-23T04:25:31Z [agent]
verify: `cargo test -p yaks skills` -> PASS (exit 0)
