---
id: yaks-006e
title: Herd filtering option for `yaks --search`
type: feature
priority: 3
created: '2026-09-21T20:03:01Z'
updated: '2026-09-21T20:44:11Z'
labels:
- herds
- cli
---

There's no way to search for a yak by herd in the CLI, except by name-grepping.

---
▸ 2026-09-21T20:44:11Z [agent]
SHORN — already delivered by yaks-3290. The repeatable --herd <prefix> filter is on every query command including search (yaks search demo --herd test scopes correctly), matching on id prefix via FilterSpec.herds. No new work needed.
