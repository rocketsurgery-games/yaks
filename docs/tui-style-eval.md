# Encoding TUI snapshots for LLMs: a style-encoding evaluation

Research report seeding the design of the agent-drivable TUI snapshot format.
Captured under `yaksrs-9b8d` (TUI agent testing interface) and its meta-probe
`yaksrs-9dae`. If we later hoist the headless-harness + differential toolkit into
a shared crate, this is the justification seed for its README.

## Problem

We drive the Rust `yaks` TUI headlessly (`yaks tui --headless`), rendering the
real `App` into a ratatui `TestBackend` `Buffer` and emitting a plain-text
snapshot after each input. Layout (the character grid) is easy to serialize.
The open question was **how to encode per-cell *style* (colour, bold, reverse —
which convey selection, focus, blocked markers, links, borders) so that a
language model can actually use it**, and at what token cost.

A secondary, sharper question: **do current models read 2D character-grid
layout at all** — alignment, containment, grouping, vertical column identity —
or does tokenization destroy it?

## Method

- **One source of truth, many encodings.** A throwaway generator
  (`tools/scratch/style_eval.py`, uncommitted) renders a fixture (grid + style
  spans) into the plain grid, every candidate encoding, and **programmatically
  derived gold answers** — so the answer key can never drift from the encoding.
- **Differential harness.** The Rust `--headless` harness and a Python
  `pyte`-based capture of the original curses TUI emit the identical frame
  format, so encodings can be compared like-for-like.
- **Probes.** Each `(fixture, encoding)` was handed to a fresh, context-free
  sub-agent (frontier model, Claude Opus 4.8) answering a fixed question battery;
  answers scored against the gold. ~40 probes total.
- **Cost.** Real token counts via `tiktoken` `o200k_base` (a modern BPE; a proxy
  for relative ordering, since Anthropic's tokenizer isn't public).

## Encodings evaluated

| name | idea |
|------|------|
| `parallel` | plain grid, then a second grid of style-ids at matching coordinates + legend |
| `interleaved` | each text row immediately followed by its style-id row |
| `ruler` | grid + a column ruler (tens/ones) and row indices, as coordinates |
| `runlist` | plain grid + a list of styled runs `rROW cA-B style` |
| `spans` | each row inline as `style[run text]`, plain spaces literal |
| `doublewidth` | each cell = style-id char + its character (negative control) |

## Findings

### 1. Frontier models read grid layout well; accuracy saturates.

On a simple two-box fixture and a dense 9x60 list (highlighted row, blocked
marker, mid-row label chips), **every style-bearing encoding scored 7/7**. On
deliberately adversarial layout — subtle 1-column misalignment, nested-vs-disjoint
boxes, a list inside a box next to a same-format decoy list — **9/9 across
`plain`/`ruler`/`spans`**. Accuracy did not discriminate encodings at this model
tier. So the deciding axis is **token cost**.

### 2. Token cost (per the dense-list fixture, vs plain-only = 132 tokens)

| encoding | tokens | x plain | notes |
|----------|-------:|--------:|-------|
| **spans** | **259** | **1.96x** | cheapest style-bearing; preserves whole words |
| interleaved | 338 | 2.56x | strictly dominates `parallel` (cheaper + local cross-ref) |
| parallel | 364 | 2.76x | obsoleted by `interleaved` |
| runlist | 451 | 3.42x | coordinate lists tokenize poorly |
| ruler | 508 | 3.85x | no accuracy gain to justify the premium |
| doublewidth | 836 | 6.33x | **dead**: id+char interleave defeats word-merging (~1 tok/char) |

### 3. `spans` survives vertical alignment — the surprising result.

The initial worry was that `spans`, being inline, destroys column alignment and
would fail vertical-alignment questions. It did not:

- **Realistic misalignment** (fixed-width divider/box, a 1-column shift): 12/12
  across `plain`/`ruler`/`spans`, both directions, no yes/no bias.
- **Cumulative offset, cue-free** (wide tables, 6->16 columns to ~100 wide, with
  gaps jittered to 1-3 spaces on every row so *no single gap is anomalous* and
  only the running total reveals a 1-column drift of a deep column): `spans`
  4/4 broken correct **plus** the aligned control correct — no false positives.
  `plain`/`ruler` also correct at max width.

**Mechanism.** `spans` preserves inter-run whitespace *literally*, so the model
recovers a column by **counting/summing whitespace (arithmetic), not by seeing
it**. This is why "hard to eyeball" does not imply "hard for the model":
human-legibility and model-legibility diverge here. We could not break it within
realistic-and-then-some TUI widths on a frontier model.

### 4. Load-bearing constraint

`spans` only survives vertical alignment **because whitespace is emitted
literally**. Collapsing/normalizing runs of spaces (a tempting token
optimization) would destroy column recoverability. **Keep spaces literal.**

## Verdict

- **Default: `spans`** — cheapest and, on a frontier model, robust across every
  layout/alignment probe we threw at it.
- **Aligned-grid fallback: `interleaved`** — if a future need wants an explicit
  column grid; it dominates `parallel`.
- `ruler`/`runlist` don't earn their token premium at this model tier;
  `doublewidth` is dead.

## Caveats / threats to validity

- **Single model.** All probes ran on Claude Opus 4.8. The one flank left open is
  **cross-model-*family*** behavior (Gemini, GPT) — plausibly different at this
  specific capability. The generator can emit a portable bundle (frames +
  questions + gold) for that study; deferred by choice.
- **Frontier-only assumption.** We explicitly do *not* target weaker models — it
  is reasonable to require a frontier model for UI development. Weaker models
  would likely crack `spans`' whitespace-arithmetic first.
- **Ceiling not fully mapped.** We stopped at 16 columns / ~100 wide; extreme
  widths (24/32+) untested.
- **Semantic vs visual labels.** In these fixtures style runs carried *semantic*
  names (border/text/selected). The live Rust harness only knows the *visual*
  ratatui `Style`; recovering semantic names (a `classify` hook / themed palette)
  is a separate, optional layer — the snapshot itself only needs concrete style.

## Reproducing

```
python3 tools/scratch/style_eval.py doc      # every encoding on two fixtures -> encodings-sample.md
python3 tools/scratch/style_eval.py sizes    # token cost per encoding (tiktoken)
python3 tools/scratch/style_eval.py scenario # align / contain / confound
python3 tools/scratch/style_eval.py valign   # divider / box vertical-alignment
python3 tools/scratch/style_eval.py vartables# cue-free cumulative-offset sweep
```

(Generator is throwaway/uncommitted; this report and the winning encoders in the
Rust harness are the durable artifacts.)
