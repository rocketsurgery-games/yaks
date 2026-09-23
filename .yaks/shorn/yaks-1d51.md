---
id: yaks-1d51
title: Improved skill installation and management
type: task
priority: 2
created: '2026-09-23T02:39:52Z'
updated: '2026-09-23T04:42:00Z'
---

Make skill installation braindead simple and hard to screw up.

---
▸ 2026-09-23T03:04:51Z [agent]
DESIGN (research-backed; sources: Agent Skills spec via code.claude.com/docs/en/skills, openskills README).

ROOT CAUSE, one sentence: an installed SKILL.md is a vendored copy with NO IDENTITY. Nothing records which producer/version wrote it, nor what was originally written -- so we cannot tell 'stale' (bd9a) from 'user edited' (d8e9). Both children are the same missing primitive: provenance.

KEY RESEARCH FINDINGS
1. The spec allows exactly SIX frontmatter fields outside Claude Code: name, description, license, compatibility, metadata, allowed-tools. Claude Code ignores unknown keys silently, but claude.ai upload / Skills API / package_skill.py HARD-ERROR on extras. So a custom top-level key (yaks-version:) would break portability -> filed yaks-e233 for our existing activation: violation.
2. metadata: is a spec-blessed, free-form YAML map explicitly 'for your own key-value data, read by your own tooling'. That is the sanctioned home for provenance.
3. LANDMINE: do NOT write a manifest/lockfile into the skills dir. Before Claude Code v2.1.280, a manifest.json in ~/.claude/skills/ caused it to MOVE the listed skill folders into .trash/. This kills the obvious 'lockfile' design and argues decisively for in-file frontmatter provenance.
4. Wild west is real: ~/.agents/skills (yaks default, Zed), ~/.claude/skills + ./.claude/skills (Claude Code), ./.agent/skills + ~/.agent/skills (openskills 'universal'), ~/.codex/skills. Note openskills uses .agent (singular), yaks defaults to .agents (plural). openskills does track install source for update, but only for git-sourced skills.

PROPOSAL
A. Stamp provenance at INSTALL time into the installed copy's frontmatter (never the repo source):
     metadata:
       yaks-version: 0.0.9
       yaks-source-sha256: <hash of the embedded content as written>
   Repo sources stay unstamped, which is exactly why they read as 'unmanaged' and get protected.
B. Derive four states by comparing the installed body (minus stamp) against the stamp + the binary's embedded copy -- dpkg/ucf conffile semantics:
     current   = hash matches embedded          -> nothing to do
     stale     = body matches its stamp, version differs -> SAFE to auto-upgrade
     modified  = body does NOT match its stamp  -> NEVER silently clobber; show diff, require explicit --force
     unmanaged = no stamp                       -> treat as modified (conservative); this alone would have prevented d8e9
C. yaks skills status / yaks skills upgrade (bd9a): scan the known dirs above, report per-skill state+version, upgrade only stale-clean ones, skip modified loudly.
D. Clobber guard (d8e9): B already prevents it, plus refuse outright when the target resolves inside the yaks source checkout (ancestor Cargo.toml name=yaks + path under skills/). Note --force is what CAUSED d8e9, so the guard must not be bypassed by plain --force.
E. Staleness surfacing without agent cooperation: fold a skills-staleness check into yaks doctor rather than inventing a hash-passing protocol. Agents already run doctor; bd9a's 'bonus points' protocol needs no new contract.

OPEN FORKS for the human: (i) should upgrade be opt-in or should doctor auto-nag? (ii) adopt-on-first-upgrade for unmanaged files, or always require --force? (iii) do we stamp yaks-tracker too, or only yaks?

---
▸ 2026-09-23T04:02:21Z [agent]
CORRECTIONS + VERIFICATION after Joel pushed back on two claims. Both were worth checking; one of my framings was misleading.

1. '.agents' (plural) -- JOEL WAS RIGHT, and yaks is correct as-is. Zed's docs (https://zed.dev/docs/ai/skills, 'Where Skills Live') specify exactly two locations: global ~/.agents/skills/ and project-local <worktree>/.agents/skills/ -- PLURAL. yaks' default_dir() matches Zed exactly. It was openskills that is the outlier: verified at source level (src/utils/dirs.ts, not just the README) it uses '.agent/skills' SINGULAR for its --universal mode, and getSearchDirs() searches only ./.agent, ~/.agent, ./.claude, ~/.claude -- it never looks at .agents at all. So a yaks-installed skill is invisible to openskills, and vice versa. My earlier note framed this as a yaks-vs-world mismatch; it is not. Revised recommendation for fork (iii): keep ~/.agents/skills as the default (it is right for the primary harness) and simply DOCUMENT the openskills path rather than installing to both -- installing to both would create two divergent copies with no shared provenance, which is the very problem this family exists to fix.

2. 'activation:' -- NOT a spec field, verified three ways (see yaks-e233 for detail): absent from the canonical spec table, from Claude Code's superset table, and from Zed's; and present in only Joel's own 7 skills out of the 20 installed, with zero third-party usage. Near-certainly locally invented and copy-pasted.

3. NEW, and it validates approach (A): the canonical spec's OWN example for the metadata: field is literally 'metadata: { author: example-org, version: "1.0" }', with the guidance 'Clients can use this to store additional properties not defined by the Agent Skills spec. We recommend making your key names reasonably unique to avoid accidental conflicts.' That is a direct endorsement of stamping yaks-version / yaks-source-sha256 under metadata:.
   CONSTRAINT to honor: the spec defines metadata as 'a map from string keys to STRING values' -- so stamp quoted strings (version: "0.0.9"), not bare numbers, or a strict validator may reject it.

4. NEW LEVER: the spec ships an official validator, 'skills-ref validate ./my-skill'. Worth wiring into the skills test / CI as the spec-compliance gate, which subsumes e233 and prevents the next invented field.

---
▸ 2026-09-23T04:18:23Z [agent]
ROUND 3 -- two premise corrections + a design problem with auto-install.

A. '.agents' is not merely Zed's choice, it IS the cross-client convention. The spec site's client-implementation guide (https://agentskills.io/client-implementation/adding-skills-support.md, 'Where to scan') tells implementors to scan BOTH their native dir and the .agents/skills/ convention: project <proj>/.agents/skills/ and user ~/.agents/skills/, labelled 'Cross-client interoperability', with the text 'The .agents/skills/ paths have emerged as a widely-adopted convention for cross-client skill sharing.' The spec deliberately does NOT mandate an install location (it only defines what goes inside a skill dir), so there is no official installer -- .agents/skills/ is the emergent answer. yaks' default is therefore exactly right.

B. BUT Claude Code does NOT appear to read ~/.agents/skills. Its documented locations are enterprise / ~/.claude/skills / <proj>/.claude/skills / nested / --add-dir / plugin / claude.ai synced -- no .agents entry (https://code.claude.com/docs/en/skills, 'Choose where skills load'). The interop guide even says implementations scan .claude/skills 'for pragmatic compatibility, since many existing skills are installed there'. So  stays genuinely necessary for Claude Code users; we should not collapse to one path.

C. openskills is NOT a crufty leftover: 42,693 npm downloads last month (api.npmjs.org/downloads/point/last-month/openskills). Recommend KEEPING the README mention. Worth noting its --universal path is '.agent' SINGULAR, which diverges from the .agents convention in (A) -- so cite it as an alternative installer, not as a path authority.

D. DESIGN PROBLEM with 'auto-install on every yaks invocation' (Joel's proposal): version ping-pong. Two binaries commonly coexist -- an npm-installed release on PATH and ./target/release/yaks in this repo. If auto-install fires whenever the embedded content differs from the installed copy, the dev build stamps its skill globally, then the older release binary sees a mismatch and stamps ITS copy back, and they flip-flop forever, each 'fixing' the other. FIX: make auto-install strictly monotonic -- only write when the embedded VERSION is strictly greater than the stamped version. Never auto-downgrade; a downgrade requires an explicit installed yaks -> /Users/joel/.agents/skills/yaks/SKILL.md
installed yaks-tracker -> /Users/joel/.agents/skills/yaks-tracker/SKILL.md

The skill activates when a .yaks/ directory is present. For another agent, re-run with --dir pointing at its skills directory (e.g. --dir ~/.claude/skills)..

REFINED AUTO-INSTALL RULES (for implementation):
  - absent -> install
  - stamped + clean (body hash == stamp) + embedded version > stamped -> upgrade
  - stamped + clean + embedded version <= stamped -> no-op (no churn, no downgrade)
  - modified (hash != stamp) or unmanaged (no stamp) -> NEVER touch; doctor warns; --force required
  - Scope: ONLY the user-level default (~/.agents/skills). Never project-local <worktree>/.agents/skills (that is writing into the user's repo) and never a --dir target.
  - Atomic write (temp + rename): the coordinator spawns parallel yaks invocations that would otherwise race and tear the file.
  - Opt-out via env/config for CI and sandboxes; silent on no-op, one line when it acts; skip for --help/--version and the  subcommands themselves.
  - d8e9 guard is SEPARATE and must survive --force: refuse to write into the yaks source checkout's own skills/ dir (detect ancestor Cargo.toml with name = "yaks"). --force is what caused d8e9, so it cannot be the escape hatch for that case.

---
▸ 2026-09-23T04:19:39Z [agent]
NOTE: the preceding note has three spans mangled by an authoring accident -- I used backticks inside a shell argument and they were command-substituted. One of them EXECUTED a skills install with the v0.0.9 binary on PATH and clobbered ~/.agents/skills/yaks/SKILL.md with the stale bundled copy, losing the Evidence section. Restored from the current build and verified identical to the repo source. Read those spans as: (1) a downgrade requires an explicit skills-install-with-force; (2) never a --dir target; (3) skip the skills subcommands themselves.

The accident is a free live reproduction of d8e9, and it exposes a GAP in the monotonic-version fix I proposed: both binaries report 0.0.9. The homebrew release and this dev build share a version number, but their embedded content differs, because the skills change landed in commit 7a4f543 AFTER the v0.0.9 tag. So a strictly-greater-version rule would NOT have auto-healed this, and equally would not have caused it.

REFINED RULE: auto-install only when the embedded version is strictly greater than the stamped version. Same version with differing content resolves to NO-OP -- not a write. That is what kills the ping-pong: two same-version binaries can never fight, and a stale binary can never downgrade a newer stamp.

CONSEQUENCE worth accepting deliberately: a skills-only fix does not reach users until the next version bump, since auto-upgrade keys on version. That is already true of the shipped binary (the skill is baked in via include_str), so it costs nothing new -- but it does mean skills edits should ride a release, and the dev loop uses an explicit forced install rather than relying on auto-install.

---
▸ 2026-09-23T04:41:58Z [agent]
Family complete. All three children shorn: e233 (spec-compliant frontmatter + a test enforcing the six allowed keys), d8e9 (the symlink clobber -- see its notes; the root cause was a ~/.agents/skills symlink into the repo, not a stray --dir), and bd9a (provenance stamp, seven-state machine, skills status, doctor advisory, and startup auto-sync).

Net effect against the original goal -- make skill installation braindead simple and hard to screw up: installation is now mostly invisible (ordinary commands top it up), and the failure modes that actually bit us are structurally prevented rather than documented away. The design held up under contact: every guardrail in this yak's notes earned its place, and the two that mattered most (monotonic versions, resolve-symlinks-before-guarding) were both discovered by breaking the thing for real rather than by reasoning.

---
▸ 2026-09-23T04:42:00Z [agent]
verify: `cargo test --workspace` -> PASS (exit 0)
