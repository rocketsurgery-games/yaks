---
id: yaks-a221
title: 'Finish Node-24 bump: artifact actions still on Node 20 at v5'
type: bug
priority: 2
created: '2026-08-23T16:56:00Z'
updated: '2026-08-23T16:59:40Z'
labels:
- dist
- ci
---

Follow-up to bbfe. The dry-run showed checkout@v5 and setup-node@v5 run on Node 24 (warnings cleared), but upload-artifact@v5 and download-artifact@v5 still target Node 20. Their Node-24 runtime landed in later majors: bump upload-artifact to v7 and download-artifact to v8 (kept interoperable), then re-validate via dry-run.

---
▸ 2026-08-23T16:59:40Z
Validated on CI (run 32653229296): all actions now on Node 24 (no deprecation annotations), download-artifact@v8 read all 5 upload-artifact@v7 binaries (interop confirmed), publish dry-ran all 6 packages. Node bump complete.
