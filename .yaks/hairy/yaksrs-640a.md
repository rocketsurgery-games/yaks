---
id: yaksrs-640a
title: Explicit label definitions in config.yaml
type: feature
priority: 3
created: '2026-08-23T02:49:59Z'
updated: '2026-08-23T02:49:59Z'
labels:
- config
---

Carried over from Python-repo yak-9332. Let .yaks/config.yaml declare the project's known labels (and maybe descriptions/colors), so agents reuse a defined vocabulary rather than inventing taxonomies. Pairs with the new label guidance in the yak skill and yaksrs-7cd1.
