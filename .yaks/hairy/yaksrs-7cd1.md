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

When using yaks privately (by far the most common case), it would be helpful if there were a way to add agent context within the .gitignore'd `.yaks/` dir.
I'm thinking the yaks skill could direct agents to look there for some special file, so that each private yaks user can specify precisely how they want them used within that specific repo.

---

Related (yak-011e): it would be really helpful to have some way to configure simple agent instructions directly from within .yaks/config.yaml. Depending upon the project, there are any number of preferences a user might express about how yaks are organized, how labels are used, an explicit decision about whether yaks are private/public, naming conventions, and so on.

This is especially important in the private use-case, where you don't want to put agent instructions in CLAUDE/AGENTS.md, and you've .gitignore'd the whole .yaks directory.
