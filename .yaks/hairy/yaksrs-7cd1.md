---
id: yaksrs-7cd1
title: Per-repo agent context for private herds (config + .yaks/ instructions file)
type: feature
priority: 2
created: '2026-08-23T02:09:20Z'
updated: '2026-08-23T02:09:20Z'
labels:
- agent
- config
---

Carried over from Python-repo yak-4930 + yak-011e. When .yaks/ is gitignored (the common private case), users can't put agent instructions in AGENTS.md. Let the herd carry its own: (a) simple agent-instruction keys in .yaks/config.yaml (label conventions, privacy decision, naming), and/or (b) a special .yaks/ file the yak skill directs agents to read. Once it exists, reference it from skills/yak/SKILL.md.
