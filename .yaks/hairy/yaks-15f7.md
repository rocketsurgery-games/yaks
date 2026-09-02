---
id: yaks-15f7
title: Allow the TUI to handle multiple .yaks dirs
type: idea
priority: 3
created: '2026-08-27T22:51:04Z'
updated: '2026-08-30T01:40:39Z'
depends_on:
- yaks-0187
---

This could be really useful when working across multiple project repos, especially if they have different upstream issue trackers.
On the yaks side, we could use this unification to manage cross-cutting concerns, especially useful when a single agent needs to make changes to both projects.
Each .yaks would still get its own config and yak prefix. We could also allow cross-references, though that might require an explicit "friend" relationship to be modeled in the config, so that the tools know where to find all the yak files.

---
▸ 2026-08-29T18:37:56Z
This is the federation layer, and it sets a hard requirement on the core (yaks-0187): the resolver must take the id set as a parameter so it can widen from one herd to a friend-set without changing callers. Cross-herd refs need a herd-qualified reference form (reserved in 0187 grammar). Consequence for rename (7a92): prefixes must be UNIQUE across the friend-set or a qualified ref is ambiguous — which loops straight back to reconciling the yaks/yaksrs split. "Friend" relationship modeled in each config so tools know where to find sibling yak files.

---
▸ 2026-08-30T00:57:06Z
DESIGN RESTATEMENT (multi-herd / friends), carried forward for a fresh session.

GOAL: let the TUI (and CLI) operate over several .yaks/ herds at once — working across multiple project repos — so cross-cutting work is visible/manageable in one place. Each herd keeps its own config + prefix. Support cross-herd references.

FOUNDATION ALREADY BUILT (this herd, yaks-688d):
- refs resolver (yaks-0187) is validation-based + prefix-agnostic and takes the id-set as a PREDICATE (known: impl Fn(&str)->bool). It can widen from one herd to a UNION across friend herds with zero caller changes — this was a deliberate federation hook.
- A herd-qualified ref form is RESERVED: `otherherd:prefix-1234`. `:` is not a ref char, so today `other:yak-0001` scans as local `yak-0001`; a test pins this. refs will parse the `<herd>:` qualifier when federation lands.
- Prefixes are now unique per herd (the yaksrs->yaks migration standardized this repo). rename-prefix is the tool to reconcile a collision when friending two herds.

KEY INSIGHT: because prefixes are unique per herd and resolution is validation-based over a supplied id-set, cross-herd references "just work" once you union the friend herds ids into the known-set — the bare full id already names its herd via its unique prefix. So the `herd:` qualifier is optional sugar, not required for basic cross-linking.

OPEN DESIGN QUESTIONS:
1. Friend model in config: per-repo `friends:` list (machine-specific paths — awkward to commit) vs a USER-level workspace file (~/.config/yaks/workspaces.yaml) naming sets of herd paths. Leaning user-level (paths are machine-specific), maybe + opt-in per-repo friends.
2. Herd addressing: prefix already disambiguates (unique), so bare ids suffice; `herd:` qualifier is a fallback/explicitness aid.
3. Facade: single-root Herd today. Need a multi-herd view — either a Herds(plural) facade unioning task lists for queries + routing mutations to the owning root (by prefix / by which root holds the id), or App holds Vec<Herd> and tags each task with its root.
4. Mutations: create routes to the "current" herd; rename_many must scan ALL friend roots for references (0af1/7a92 are single-root today); cross-herd MOVE is separate (see yaks-54e9 export/import).
5. UI: herd badge/column per row, a current-herd for creation, optional per-herd scope/filter.
6. Enforce prefix uniqueness when friending (detect collision -> offer rename-prefix).

SUGGESTED SLICES: (a) read-only multi-herd VIEW (load union, herd badges, cross-herd refs resolve via unioned known-set) — delivers most value; (b) friend/workspace config; (c) cross-herd mutations (create routing, rename across roots); (d) explicit herd: qualifier + collision enforcement.
