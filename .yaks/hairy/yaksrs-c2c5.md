---
id: yaksrs-c2c5
title: Fix up inter-yak reference rendering/linking
type: feature
priority: 2
created: '2026-08-25T03:35:02Z'
updated: '2026-08-25T03:35:02Z'
labels:
- ui
---

Ensure that all references of the form `yaktype-1234` are treated as followable links, and propertly highlighted.

Also include skill guidance nudging agents to use the full form, rather than just the shorthand `1234`, so that they'll tend to work out of the box.
