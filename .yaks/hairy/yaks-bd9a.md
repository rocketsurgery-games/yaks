---
id: yaks-bd9a
title: Skill update and validation
type: feature
priority: 3
created: '2026-09-08T02:26:00Z'
updated: '2026-09-08T02:26:00Z'
labels:
- skills
---

When the `yaks` CLI tool is updated, locally-installed skills can still be out of date. It would be good for `yaks` to validate them somehow, and require (or at least recommend) that they be upgraded before continuing.

This is a little tricky, because they could in theory be installed just about anywhere. Maybe we just have a simple `yak skills upgrade` tool. But it would be nice if the tool could at least detect stale skills in the standard location.

Bonus points if we can find a way to get agents to reliably pass some kind of skill version/hash to `yaks` that we can validate.
