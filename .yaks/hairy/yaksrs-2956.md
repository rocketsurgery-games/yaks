---
id: yaksrs-2956
title: Render-time markdown highlighting (detail + editor), hand-rolled
type: feature
priority: 2
created: '2026-08-25T03:26:25Z'
updated: '2026-08-25T03:26:25Z'
labels:
- ui
---

Replace edtui's syntect-based highlighting (the opt-in md-syntax feature) with one hand-rolled markdown highlighter used in BOTH surfaces, pure-Rust, no onig/syntect, no fork.

Key finding: edtui exposes a public, non-feature-gated EditorState.highlights (Vec of {start,end,Style}), applied at render beneath selection styling and computed in logical coords so it survives wrapping. So we can color the editor ourselves without syntect.

Plan:
- Write a markdown tokenizer -> styled spans (headings, bold/italic, inline code, code fences, list bullets, blockquotes, links). Reuse the existing detail link scan.
- Detail pane (read view, the important one): apply the spans in render_dline / detail::build so viewing a yak shows colored markdown. Compose with existing link + find-match + wrap logic.
- Editor: compute highlights from the buffer each render and set state.highlights before rendering the EditorView (comment M + description). Full parity, no syntect.
- Remove the md-syntax cargo feature, apply_syntax(), edtui syntax-highlighting dep, and the editor_syntax config/App plumbing (supersedes shorn yak d8c7).
- Stretch: embedded language highlighting inside fenced code blocks (our own small tokenizers, or an optional scoped highlighter) - fully our choice.

Obviates d635 (edtui fancy-regex PR) and f640 (enable md-syntax by default) since we drop syntect entirely.
