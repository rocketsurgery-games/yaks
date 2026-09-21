---
id: yaks-eee7
title: 'docs follow-ups: family-scope h key + herd/tracker mapping'
type: task
priority: 3
created: '2026-09-21T18:37:07Z'
updated: '2026-09-21T18:37:07Z'
parent: yaks-15f7
labels:
- docs
---

Doc gaps surfaced while shaving the multi-herd family. (1) docs/tui.md does not document the list-view family-scope h key (the auto/lone/remaining/all cycle) at all - a pre-existing gap made more visible by the herd->family rename; document it, and add the herd badge/column when yaks-3290 lands. (2) skills/yaks-tracker: note that in a consolidated farm, herds commonly map to different upstream trackers (the original four-repos mixed Jira/GitHub motivation); source/rollup stay per-yak but a short pointer helps. (3) AGENTS.md: a brief contributor note on the farm/herd/family model + the multi-herd config once yaks-98d5 lands. NOTE: no changeset workflow in this repo (tag-based releases), so nothing owed there.
