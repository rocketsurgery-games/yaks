---
id: yaks-f5e3
title: Color yak herds differently so they stand out
type: feature
priority: 3
created: '2026-09-21T19:56:37Z'
updated: '2026-09-21T20:44:11Z'
labels:
- herds
- tui
---

Let's make sure the yaks are color-coded by herd. Bonus points for making it configurable in config.yaml, though I'm not entirely certain what the color naming convention should be. Whatever makes the most sense in a terminal context.

---
▸ 2026-09-21T20:44:11Z [agent]
SHORN (core) — per-herd id colouring shipped in yaks-a7eb: each row's id prefix is tinted by herd (render::herd_color, FNV-1a into a 6-colour palette), gated to multi-herd farms; verified live on this repo. Integrator note: tightened herd detection to real (dashed) id prefixes so test fixtures are not misread as multi-herd. BONUS (configurable palette in config.yaml) not done — deferred as optional; the terminal-native default palette is sensible. Open a follow-up if a config knob is wanted.
