---
id: yaks-64c4
title: 'Yak archaeology: fast retrieval of past reasoning & provenance stitching'
type: idea
priority: 4
created: '2026-09-09T00:38:19Z'
updated: '2026-09-09T00:38:19Z'
labels:
- cli,search
---

Umbrella/holding-pen (no concrete design yet) for making it fast for humans AND agents to do yak archaeology: find past reasoning, reconstruct the history/provenance of a change, idea, decision, or architecture, and stitch a narrative across yaks + git. Motivating context: the 'parking' discussion. Dead/ yaks (decided-against or abandoned) carry real research output; today they're RECOVERABLE ('list --all' includes dead, 'show <id>' reads any state, 'revive' resurrects) but not easily DISCOVERABLE (which dead yak holds the reasoning I want?) or STITCHABLE. So dead-invisibility is 'both': a FEATURE that dead is hidden from default 'list'/'next' (out of sight is the point for terminal work), and a latent GAP that there's no search/stitch to make burial non-lossy. Candidate sub-ideas, NONE committed: (1) keyword search -> candidate set across ALL states incl. dead (maybe existing affordances + a --include-dead/--all-aware search); (2) provenance stitching — build on 'yaks commits' (file-follow) plus a 'git blame'-equivalent that maps a code line/decision back to the yak(s) that produced it (long-standing idea; the circular-dependency of stamping commit hashes onto yaks is why we anchor on the file-follow, not a stored hash); (3) one-view lineage of a yak (parent/children/refs/commits/notes). PARKING PRINCIPLE this reframes (candidate skill nudge, not built): dead = terminal archive, not a black hole; record the decision + rationale as a note BEFORE slaughter (a dead yak is only worth keeping if it carries its conclusion — a trigger can't fire from the grave); deferred-but-live work stays hairy (it needs to resurface). Don't rabbit-hole now; this yak just holds the concept so the parked/dead visibility question has a home.
