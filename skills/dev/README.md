# skills/dev — experimental, repo-internal workflow skills

These are simplified, pstack-inspired workflow skills we use to develop yaks
itself and to test the coordination concepts tracked under `yaks-3901`.

They are **not shipped**. The binary only bundles `skills/yak` and
`skills/yak-tracker` (see the `BUNDLED` list in `src/skills.rs`); nothing in
`skills/dev/` is embedded or installed by `yaks skills install`. To exercise one
with an agent harness, point the harness at it (for example, symlink or copy the
skill into `~/.agents/skills/` for the session). The source of truth stays here,
under version control.

Keep each skill minimal. The goal is the least guidance that measurably changes
behavior — judged by whether it cuts turns and yak-herding drudgery — not a
heavyweight framework. Promote a convention into the tool (a CLI flag, a lint, a
field) once it has earned its place; until then it lives here as prose.
