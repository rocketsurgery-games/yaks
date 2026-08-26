---
id: yaksrs-2956
title: Render-time markdown highlighting (detail + editor), hand-rolled
type: feature
priority: 2
created: '2026-08-25T03:26:25Z'
updated: '2026-08-26T01:43:31Z'
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

---
▸ 2026-08-26T01:43:31Z
Implemented. New src/tui/markdown.rs: hand-rolled, pure-Rust markdown highlighter (headings, bold/italic with dimmed markers, inline code, fenced code blocks tracked across lines, list bullets, blockquotes; flanking rules keep snake_case and spaced-asterisk arithmetic from being emphasized). Emits char-coord spans reused by BOTH surfaces. Detail pane: DLine gains an md span vec, computed per body line in build() and remapped through wrap(); render_dline paints it beneath the link/find layers. Editor: set_md_highlights() converts spans to edtui Highlights (logical row/col) set on state each render, so coloring shows in Normal AND Insert and survives edtui wrapping. Removed the md-syntax feature, apply_syntax(), and all editor_syntax config/App plumbing -- Cargo.lock now drops syntect/onig/onig_sys entirely (C-free). Tests: 11 span-level + 2 detail-integration + reworked editor render smoke test; full workspace green, 0 warnings. Stretch (embedded code-block languages) left for later.
