---
id: yaks-e3aa
title: Document insta snapshot authoring + attach side-effects in yaks-working
type: task
priority: 3
created: '2026-09-11T00:11:13Z'
updated: '2026-09-11T01:01:55Z'
labels:
- meta
- skills
---

Two per-yak working-trail frictions from the parallel UI dogfood run, for the yaks-working evidence section: (1) authoring NEW insta snapshots needs INSTA_UPDATE=always (or cargo insta accept) -- plain 'cargo test' writes .snap.new and FAILS, and a single test emitting two new snapshots cannot self-bootstrap in one plain run; note the accept step. (2) 'yaks attach' MUTATES the tracked yak .md (adds the artifact ref) in addition to copying the file, so the yak file shows modified before commit -- call this out so lanes stage it deliberately.

---
▸ 2026-09-11T01:01:55Z [coordinator]
Shorn: added to yaks-working (1) a 'yaks attach also modifies the tracked yak file' side-effect note on the evidence bullet, and (2) a 'new snapshot/golden needs your tool's accept step' bullet (generic principle; insta accept / INSTA_UPDATE=always as the example). Evidence: edits in skills/dev/yaks-working/SKILL.md.
