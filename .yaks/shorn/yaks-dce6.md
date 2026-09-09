---
id: yaks-dce6
title: 'Prototype: lean LLM-parseable SVG for TUI docshots (classes + inheritance + coalesced rects + per-row text)'
type: task
priority: 4
created: '2026-09-09T02:29:07Z'
updated: '2026-09-09T04:54:54Z'
parent: yaks-89b9
labels:
- ui,docs
---

Spike under yaks-1a96 side-quest. Refactor svg_of in src/tui/docshots.rs to cut bloat: (1) <style> class block for palette+modifiers, (2) hoist font-family/size/lengthAdjust/xml:space to root via inheritance, (3) coalesce contiguous same-bg cells into one <rect> per run, (4) one <text> per row with <tspan> only on style change + explicit per-run x (keep column pinning; avoid risky row-level textLength given mid-row emoji). Goal: feel out byte savings + whether the SVG source reads row-by-row for direct LLM parsing. Verify by driving docshots and eyeballing rendered frame + diffing bytes/element counts.

---
▸ 2026-09-09T02:45:15Z [agent]
Prototype done (uncommitted, in working tree). Rewrote svg_of in src/tui/docshots.rs + regenerated docs/assets/*.svg. Verified by rendering baseline vs new via headless Chrome and pixel-diffing with ImageMagick (render noise floor = 0, deterministic).

FINAL = Option A: <style> class block (dedup palette+modifiers) + 'text{white-space:pre}' + coalesced background rects (per-run, not per-cell) + font attrs hoisted to root (inherit) + per-run <text> keeping x+textLength+lengthAdjust. Result: tui-list -64% (12504->4572B), detail -37%, drawer -39%, inbox -76%; PIXEL-IDENTICAL to baseline (AE<0.07%, residual = coalesced-rect seam). Suite green (238+25).

Key SVG findings (static, no scripting):
1. Chrome IGNORES textLength on a <tspan> (honors it only on <text>); it DOES honor x on a tspan. Proved with a probe SVG.
2. A single row-level textLength stretches each row independently -> DESTROYS vertical column alignment (Option B, dead). Emoji also visibly distort under spacingAndGlyphs stretch.
3. Chrome does NOT inherit xml:space='preserve' from the root -> multi-space gaps collapse while textLength still stretches -> drift. Fix: CSS 'white-space:pre' (inherits). This was the hidden ~5% error in early attempts.

Alt = Option C (row <text> + per-run <tspan x>, runs split after wide glyphs via unicode-width, no textLength): smallest + most LLM-parseable (one source line per row), columns hold, BUT ~4% cosmetic intra-run drift (no textLength stretch; white-space:pre doesn't fix it). Good if we ever want the SVG itself agent-parseable and accept minor drift; A chosen as default since (per yaks-1a96) the agent should read toque TEXT, so the SVG's job is human fidelity.

Not landed: needs A-vs-C decision + docs/skills note if the format change ships (docs/tui.md regen guidance unchanged; format is internal). Evidence imgs in target/svgcmp/ (gitignored).

---
▸ 2026-09-09T03:24:20Z [agent]
Switched tree to Option C (row <text> + per-run <tspan x>, wide-glyph split via unicode-width, white-space:pre, no textLength). tui-list -78%, detail -66%, drawer -67%, inbox -85%; columns hold; ~4% cosmetic intra-run drift (fine for a TUI per Joel). Suite green.

Explored Joel's 3 Qs with probes:
Q1 (drop repeated attrs via SVG abstraction): textLength/lengthAdjust are non-inheritable geometry attrs with no CSS-property form -> CANNOT be hoisted to <g>/<style>; only removable by not needing them (C already has none; remaining per-run attr is just x, irreducible position). Inheritable/CSS attrs (fill/font-*/opacity/text-decoration/white-space) already hoisted.

Q2 (equal-length rows + whitespace, drop textLength entirely): built a pure-FLOW variant (one <text>/row, literal full-row text, color via in-flow <tspan class> with NO x/textLength). Leanest yet: tui-list 2545B (-80%) and source reads near-verbatim like the toque snapshot. BUT AE 5.8% and a real break: text rides the FONT grid while bg rects ride the PX grid (CW=8.4); they only stay together if font advance == 8.4px. Emoji (advance != 2 cells) detach the header highlight box from 'Shaving (3)'; an unknown viewer monospace would drift cumulatively. So flow alone is unreliable.

Q3 (custom font w/ 2x1 emoji): this is exactly what makes Q2 safe -- an EMBEDDED @font-face with known advance pins font grid == px grid (kills all textLength/x AND the unknown-viewer drift), and 2-cell emoji glyphs remove the local break. Feasibility: needs fonttools (NOT installed) to subset a mono + remap our ~8 emoji to 2ch advance (color emoji => COLR/bitmap subset, ~5-20KB; mono outline emoji smaller). Cost = fixed per-SVG font embed (bad for small SVGs, self-contained though) vs per-element savings. Only system mono is SFNSMono.ttf 224KB (too big to embed raw). => Q2+Q3 are ONE design (font-pinned flow); a separate follow-up yak if we pursue it.

Recommendation: ship C now (lean+parseable+robust, no new deps). Font-pinned flow (Q2+Q3) is the 'maximally lean' endgame but is a build-a-font project.

![flow-fontpinned](artifacts/yaks-dce6/flow-fontpinned.svg)

---
▸ 2026-09-09T03:52:23Z [agent]
Posterity: structural example of 'font-pinned flow' SVG (Q3 endgame we decided NOT to pursue). Won't render without the (unbuilt) metric-pinned font; kept to show the structure — pure-flow body with zero textLength/x, one @font-face doing all the load-bearing work.

---
▸ 2026-09-09T04:12:43Z [agent]
Reparented under umbrella yaks-89b9 as the RENDERER child. Scope extends from prototype -> landing: promote the Option C buffer->SVG generator out of src/tui/docshots.rs into TOQUE as a public render_to_svg (or SvgEncoder), parameterized by the color palette (currently hardcoded Tomorrow-Night in docshots.rs); rewire yaks docshots to be a thin caller. The C impl + regenerated docs/assets/*.svg currently sit UNCOMMITTED in the working tree from the spike — land them via this promotion. Keep the wide-glyph split (unicode-width) + white-space:pre + coalesced rects + class block. Shear once it's in toque and committed.

---
▸ 2026-09-09T04:54:54Z [agent]
Shorn. Promoted the Option C renderer into toque as public render_to_svg + buffer_to_svg (new crates/toque/src/svg.rs); the Tomorrow-Night palette + geometry moved with it. src/tui/docshots.rs is now a thin caller (toque::render_to_svg). Output byte-identical to the in-tree spike modulo one redundant root xml:space dropped (-21B/file; white-space:pre does the work). Added unicode-width dep to toque. Rendered tui-list via headless Chrome post-move — columns hold. Suite green.
