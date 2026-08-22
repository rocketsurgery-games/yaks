---
id: yaksrs-1861
title: 'Encoding: double-width per-char inline (negative control)'
type: task
priority: 4
created: '2026-08-22T13:29:05Z'
updated: '2026-08-22T13:41:45Z'
parent: yaksrs-9b8d
labels:
- tui
- eval
---

The double-width schemes that interleave a style id before every character. Included as a calibration / negative control: expected to hurt by shredding words into per-char tokens, fighting the tokenizer. Confirms whether word-token integrity is the dominant factor.

---
▸ 2026-08-22T13:41:45Z
Confirmed DOA: 836 tokens (6.33x plain) because id+char interleaving defeats BPE word-merging (~1 token/char). Worst cost by far, no upside. Negative control validated.
