---
id: yaks-75cc
title: 'Skill guidance: treat unexpected yak updates as useful signals from the user'
type: task
priority: 1
created: '2026-09-08T03:26:58Z'
updated: '2026-09-08T03:26:58Z'
labels:
- skills
---

We frequently see crap from agents like:
> I left your live drift (the `d4d2` note, `bd9a`, the `a2ca` move) alone

And they often spend an unreasonable amount of time trying to figure out why the working tree is dirty, or whether they inadvertently did something to a yak.

Instead, it should simply serve as signal that the user is also doing something at the same time. This may or may not be useful signal as to the user's focus, but it shouldn't be a surprise.
