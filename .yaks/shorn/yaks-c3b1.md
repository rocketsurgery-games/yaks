---
id: yaks-c3b1
title: Per-repo context names the project's verification lever(s)
type: feature
priority: 2
created: '2026-09-08T01:32:42Z'
updated: '2026-09-08T22:13:47Z'
parent: yaks-6624
labels:
- agent,config,eval
verify: cargo test -p yaks config
---

Ties yaks-7cd1 (per-repo agent context). The repo declares what 'verify' concretely MEANS for it — its levers and how to invoke them (e.g. 'TUI: drive toque and look + cargo test docshots'; 'web: sightmap/CDP'; 'stateful: this seed script'). Gives the agent the real-artifact definition so 'it was too hard' is not an excuse, and gives doctor/ask something concrete to point at. This is yaks' lightweight analog of pstack's Feature Map / verification-skill, but yaks supplies the escalation+tracking substrate rather than shipping a create-verification-skill.

---
▸ 2026-09-08T22:10:02Z [coordinator]
[accept] Done = (1) config.yaml gains a nested 'verify:' map (label -> command, + optional 'default'), parsed by read_config (block-tracking like parse_task); (2) a resolver Config::resolve_verify(labels) -> first label with an entry (yak's label order), else 'default', else None; (3) 'yaks verify <id>' falls back to the resolved config command when the yak has no explicit verify: field, printing the source (explicit vs config[label] vs default). doctor --strict stays keyed on the EXPLICIT verify: field (no retroactive flag-flood on existing herds; enforcement stays opt-in per-yak). Dogfood: this repo's config.yaml gets ui->docshots, cli->cargo test -p yaks, default->cargo test --workspace, and 'yaks verify' on a label-only yak resolves it. Evidence: unit tests (config parse + resolver precedence); cargo test --workspace green; live 'yaks verify' resolving from config. Docs+skill in same change. Judge: coordinator (scriptable). Scope: store.rs (Config), main.rs (verify dispatch), config.yaml, docs, skills.

---
▸ 2026-09-08T22:12:22Z [coordinator]
verify: `cargo test -p yaks config` -> PASS (exit 0)

---
▸ 2026-09-08T22:13:47Z [coordinator]
verify: `cargo test -p yaks config` -> PASS (exit 0)

---
▸ 2026-09-08T22:13:47Z [coordinator]
Shorn. config.yaml gains a nested verify: map (label->command + default), parsed by read_config with block-tracking; Config::resolve_verify(labels) does first-label-then-default; 'yaks verify' falls back to it when a yak has no explicit verify:, printing the source (yak|config). doctor --strict deliberately UNCHANGED (keys on explicit verify: only) so an existing herd's old shorn yaks don't flag-flood; enforcement stays opt-in per-yak. Tests: config_parses_verify_map_and_block_closes, config_resolve_verify_precedence. Suite green (238+25+13). LIVE: this repo's config maps ui/cli/skills/default; a scratch labeled 'skills' with no verify: resolved 'cargo test -p yaks skills' (source config) -> PASS; c3b1's own explicit verify: (source yak) -> PASS. Docs+skills updated same change. Layering matches views/config: per-yak verify: overrides config default.
