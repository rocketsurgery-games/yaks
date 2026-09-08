---
id: yaks-52eb
title: 'Bug: yaks attach writes to gitignored .yaks/artifacts -> team-mode attachments dangle'
type: bug
priority: 2
created: '2026-09-08T22:46:23Z'
updated: '2026-09-08T22:56:14Z'
labels:
- cli,git
verify: '! git check-ignore .yaks/artifacts/yaks-52eb/tui-list.svg'
---

---
▸ 2026-09-08T22:46:35Z [coordinator]
Found while Joel asked whether any yak actually HAS an attachment (none do). Root cause: root .gitignore line 7 'artifacts/' (a broad rule, likely for build output) also matches .yaks/artifacts/, so yaks attach writes a file that never commits. In team mode the yak body's ![](artifacts/<id>/<file>) link is committed but the file is not -> dangles for everyone but the author; git clean -fdx wipes it. Contradicts herd.attach's comment ('committed alongside .yaks'). The only real link in the herd (dead yaks-8222) already dangles; artifacts dir is empty. DECISION NEEDED (load-bearing): (A) un-ignore .yaks/artifacts (e.g. '!.yaks/artifacts/' or narrow the rule) so attachments commit -> but binary bloat in a code repo (SVG/text diffs fine; PNG/screenshots bloat); or (B) keep artifacts local-only and make it HONEST (fix the herd.attach comment; skill says attachments are local scratch, use a committed path like docs/assets for shared evidence). Likely: A for text artifacts (SVG) we want shared; be deliberate about binaries. Either way: fix the misleading comment + add skill/doc guidance.

---
▸ 2026-09-08T22:54:52Z [coordinator]
[accept] Done = (1) root .gitignore un-ignores .yaks/artifacts/ (negation after the broad artifacts/ rule) so attachments commit in team mode, WITHOUT un-ignoring a stray root artifacts/ (verified via git check-ignore); (2) herd.attach's comment made truthful; (3) guidance: evidence artifacts go via 'yaks attach' as EXTERNAL files (committed in team mode) — never paste large/binary (incl. SVG) inline into a note/description (yaks-working + cli.md); (4) a REAL committed attachment exists on this yak (a docshots SVG) so 'does any yak have an attachment?' is finally yes. verify: asserts the attached file is not gitignored. Judge: coordinator (scriptable check + I'll view the committed SVG). Scope: .gitignore, src/herd.rs comment, skills, docs, + the attached artifact.

![tui-list](artifacts/yaks-52eb/tui-list.svg)

---
▸ 2026-09-08T22:55:19Z [coordinator]
Proof attachments now commit: this TUI frame is stored under .yaks/artifacts/yaks-52eb/ and (team mode) committed — viewable evidence for a human or coordinator, external file, only a link in the body.

---
▸ 2026-09-08T22:55:53Z [coordinator]
verify: `! git check-ignore .yaks/artifacts/yaks-52eb/tui-list.svg` -> PASS (exit 0)

---
▸ 2026-09-08T22:56:14Z [coordinator]
Shorn. Root .gitignore now un-ignores .yaks/artifacts/ (negation after the broad artifacts/ rule); verified a stray root artifacts/ still ignored (no regression) and a file inside .yaks/artifacts/ is trackable. herd.attach comment made truthful. Guidance added (yaks-working + cli.md): attach keeps evidence an EXTERNAL file under .yaks/artifacts/ (committed in team mode), never inline large/binary (SVG incl.). Durable proof: docs/assets/tui-list.svg is attached to THIS yak and committed under .yaks/artifacts/yaks-52eb/ — so a yak finally HAS a real, viewable, committed attachment. verify: (! git check-ignore ...) PASS; suite green (238+25+13). NOTE for follow-up: whether to emit toque's LLM-friendly TEXT snapshot format vs SVG for TUI evidence (Joel's Q) is a separate design choice; either way, external files.
