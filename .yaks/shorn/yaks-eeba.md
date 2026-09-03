---
id: yaks-eeba
title: Render preserved unknown frontmatter fields in TUI detail
type: feature
priority: 3
created: '2026-09-03T19:58:25Z'
updated: '2026-09-03T20:02:36Z'
parent: yaks-594b
labels:
- ui
---

TUI counterpart to the CLI show render: surface Task.extra (unmodeled frontmatter kept by yaks-031d) read-only in the detail pane, so hand-added/newer keys are visible instead of silently invisible. A small 'Other fields:' section near the end of the header block. Scope: src/tui/detail.rs (build). Read-only.

---
▸ 2026-09-03T20:02:36Z [wt-tui]
Done. Added a read-only 'Other fields:' section to detail.rs build() after the Source block, echoing task.extra verbatim (2-space indent, no links/md). Test extra_renders_read_only_other_fields_section covers empty->absent and non-empty->verbatim lines under the header; asserts no link targets. cargo build --release clean; cargo test --release 211 passed + 20 integration passed.
