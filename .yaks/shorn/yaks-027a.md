---
id: yaks-027a
title: Answer mode needs a little spacing
type: bug
priority: 3
created: '2026-09-11T01:46:31Z'
updated: '2026-09-23T21:43:06Z'
labels:
- ui
---

The "answer" popup has no spacing between the instructions and beginning of text (starts with "Strong..." below):
`Answer yaks-b1cc (clears the block) — Enter save · Ctrl-C cancelStrong agreement on aggressive checkpointing`

---
▸ 2026-09-23T21:35:37Z [coordinator]
Lightning-round lane 'polish' (worktree wt/polish, branch lane-polish). Scope: ask/answer overlay render spacing. Evidence contract: gate = cargo test -p yaks green (re-accept + eyeball any snapshot ripple); PLUS drive the headless TUI (toque) to the relevant state and yaks attach the frame (text, or docshot SVG when color matters). Judge: coordinator at merge review; subjective forks -> yaks ask and hand back.

![yaks-027a-answer-after](artifacts/yaks-027a/yaks-027a-answer-after.txt)

---
▸ 2026-09-23T21:42:50Z [lane-polish]
Headless frame (110x10): answer prompt now reads 'Answer a0 — clears the block (Enter save · Ctrl-C cancel): Strong agreement…' — label ends in ': ' so typed text is separated.

![yaks-027a-ask-after](artifacts/yaks-027a/yaks-027a-ask-after.txt)

---
▸ 2026-09-23T21:42:50Z [lane-polish]
Same for the ask prompt.

---
▸ 2026-09-23T21:43:06Z [lane-polish]
Shorn. The single-line ask/answer prompts rendered label then typed text with no separator. Relabelled them (src/tui/handlers.rs open_ask/open_answer) to 'Ask <id> — blocks on a human (Enter save · Ctrl-C cancel): ' / 'Answer <id> — clears the block (Enter save · Ctrl-C cancel): ', matching the ': ' convention of the other single-line prompts (Labels/Save view as/Rename view/Attach path). Comment is the multi-line right-pane editor with the label on its own header row — no issue there. Regression test: tui::tests::ask_and_answer_prompts_separate_label_from_typed_text. No key/behavior change, so no docs/help ripple (grep confirmed no docs quote the old label).
