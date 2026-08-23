---
id: yaksrs-aa49
title: 'id to path resolution: yaks path'
type: feature
priority: 3
created: '2026-08-23T03:43:37Z'
updated: '2026-08-23T03:43:37Z'
parent: yaksrs-8d53
labels:
- cli
- git
---

yaks path ID prints the current on-disk file path for a yak (and yaks path [filters] for a set), so git add is precise even though transitions move files between status dirs. Motivation: repeatedly hand-built .yaks/status/id.md paths and fought staging drift when committing yak moves alongside code.
