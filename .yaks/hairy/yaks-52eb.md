---
id: yaks-52eb
title: 'Bug: yaks attach writes to gitignored .yaks/artifacts -> team-mode attachments dangle'
type: bug
priority: 2
created: '2026-09-08T22:46:23Z'
updated: '2026-09-08T22:46:35Z'
labels:
- cli,git
---

---
▸ 2026-09-08T22:46:35Z [coordinator]
Found while Joel asked whether any yak actually HAS an attachment (none do). Root cause: root .gitignore line 7 'artifacts/' (a broad rule, likely for build output) also matches .yaks/artifacts/, so yaks attach writes a file that never commits. In team mode the yak body's ![](artifacts/<id>/<file>) link is committed but the file is not -> dangles for everyone but the author; git clean -fdx wipes it. Contradicts herd.attach's comment ('committed alongside .yaks'). The only real link in the herd (dead yaks-8222) already dangles; artifacts dir is empty. DECISION NEEDED (load-bearing): (A) un-ignore .yaks/artifacts (e.g. '!.yaks/artifacts/' or narrow the rule) so attachments commit -> but binary bloat in a code repo (SVG/text diffs fine; PNG/screenshots bloat); or (B) keep artifacts local-only and make it HONEST (fix the herd.attach comment; skill says attachments are local scratch, use a committed path like docs/assets for shared evidence). Likely: A for text artifacts (SVG) we want shared; be deliberate about binaries. Either way: fix the misleading comment + add skill/doc guidance.
