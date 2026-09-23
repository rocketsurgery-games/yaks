---
id: yaks-7cb3
title: Parse label edits with/without commas
type: bug
priority: 3
created: '2026-08-26T13:23:02Z'
updated: '2026-09-23T21:45:01Z'
parent: yaks-f04d
labels:
- ui
- labels
---

At present, it's possible to create labels with spaces in them. We should disallow this.
Then we can enforce label structure at parse time (we should probably normalize them in the CLI tools as well).
If the user types:
- `foo, bar` -> `[foo bar]`
- `foo bar`  -> `[foo bar]`
- `foo,bar`  -> `[foo bar]`

And so forth. Ie, we should disallow spaces and commas in labels.

---
▸ 2026-09-10T23:46:27Z [coordinator]
CLI evidence (UI-batch recon): 'yaks create --labels meta,skills' stores a SINGLE joined label 'meta,skills' -- --labels is space-variadic and does not split on comma. Existing herd yaks show the same smell (ui,docs; skills,eval; ui,editing). Whatever normalization this yak lands for the TUI label editor should also cover the CLI --labels/--add-label path: split+trim on comma/space, drop empties.

---
▸ 2026-09-23T21:35:37Z [coordinator]
Lightning-round lane 'labels' (worktree wt/labels, branch lane-labels). Scope: label normalization (split on comma/space, no spaces/commas) in store/CLI/TUI label editor. Evidence contract: gate = cargo test -p yaks green (re-accept + eyeball any snapshot ripple); PLUS drive the headless TUI (toque) to the relevant state and yaks attach the frame (text, or docshot SVG when color matters). Judge: coordinator at merge review; subjective forks -> yaks ask and hand back.

![labels-cli-transcript](artifacts/yaks-7cb3/labels-cli-transcript.txt)

---
▸ 2026-09-23T21:44:51Z [lane-labels]
CLI transcript on a scratch farm: --labels/--add-label/--remove-label/bulk split on commas+spaces; doctor flags a legacy 'ui,docs'; any label edit re-splits it; doctor clean after.

![labels-tui-frames](artifacts/yaks-7cb3/labels-tui-frames.txt)

---
▸ 2026-09-23T21:44:51Z [lane-labels]
Headless TUI (toque) frames: L on a legacy 'ui,docs' yak + Enter re-splits to [ui, docs]; L typing 'rust tui, rust,,cli' commits [rust, tui, cli].

---
▸ 2026-09-23T21:45:01Z [lane-labels]
Done. One normalizer: model::normalize_labels (split on commas+whitespace, trim, drop empties, dedupe in order) + is_canonical_label + label_edit (TUI diff vs normalized current). Applied at Farm::create/update (choke point for CLI+TUI), CLI bulk (so dry-run shows normalized) and --label filters, TUI L editor / create-edit form labels_vec / drawer label filter. Any label edit re-splits a yak's legacy labels (canonicalize-on-touch); TUI L+Enter on a legacy yak re-asserts to canonicalize. doctor: new non-strict MalformedLabel ('malformed-label') issue. Existing farm NOT rewritten: doctor flags 13 yaks (139b 1a96 52eb 64c4 79a9 7d49 b716 c3b1 d4d2 d954 dce6 f433 f814) -- so plain 'yaks doctor' on this farm now exits non-zero until fixed; fix per yak with e.g. 'yaks update <id> --add-label <any-existing-piece>'. No migration command built. Docs: docs/cli.md, docs/tui.md, README.md, skills/yaks/SKILL.md, clap help on --labels/--add-label/--remove-label/--label. Tests: unit (model::label_tests), CLI labels_normalized_on_create_update_bulk_and_doctor, TUI labels_edit_normalizes_commas_and_spaces; cargo test --workspace green (284+26+7+1). NOTE: shared CARGO_TARGET_DIR is unsafe for tests/cli.rs across lanes (deps/cli-HASH and target/debug/yaks collide; cargo even reports fresh against another worktree's sources) -- I ran tests under a lane-private profile in the shared dir: --config 'profile.lanelabels.inherits="dev"' --profile lanelabels (target/lanelabels, ~400M).
