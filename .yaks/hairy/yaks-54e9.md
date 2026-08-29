---
id: yaks-54e9
title: 'Cross-herd move: yaks export / import (JSON)'
type: feature
priority: 2
created: '2026-08-23T03:43:37Z'
updated: '2026-08-23T03:43:37Z'
parent: yaks-8d53
labels:
- cli
---

yaks export [filters] --json emits full yaks (frontmatter + body + status); yaks import FILE recreates them, with a policy for id collisions (preserve vs remap). Motivation: porting the Python herd into yaks-rs was cat + re-create + restore-descriptions by hand; export/import makes moving yaks between herds a two-command operation.
