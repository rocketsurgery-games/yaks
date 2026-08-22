---
id: yaksrs-9b8d
title: TUI agent testing interface
type: task
priority: 3
created: '2026-08-22T12:04:00Z'
updated: '2026-08-22T12:39:50Z'
---

Library and tools allowing agents to effectively operate and test TUIs.
- CLI interface for simple stdin/stdout mode.
- Remove VT commands - raw text interface.
- The ability to send arbitrary input (keys, virtual pointer, etc).
- Specify the terminal size & resize.
- Take snapshots of the current UI state.
- Include style information in snapshots, specified in such a way that LLMs can interpret it effectively.
- Including any techniques for effectively working with ratatui code and idioms.
