---
id: yaksrs-2a15
title: 'edtui PR: >> / << indent/dedent'
type: feature
priority: 3
created: '2026-08-24T22:12:10Z'
updated: '2026-08-25T23:11:59Z'
parent: yaksrs-6099
labels:
- ui
- edtui
- upstream
---

No indent action exists. Add IndentLine/DedentLine actions (linewise + visual), shiftwidth-configurable, undoable. Bigger than the others (operator + linewise). Requires GitHub fork.

---
▸ 2026-08-25T23:11:59Z
Done natively: indent module in edtui (branch yaks/indent) - >>/<< (IndentLine/DedentLine, count-aware) and visual >/< (IndentSelection/DedentSelection). Shiftwidth = tab width; blank lines untouched; undoable. Merged to yaks-integration; yaks consumes it.
