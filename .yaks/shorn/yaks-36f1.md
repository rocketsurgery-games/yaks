---
id: yaks-36f1
title: 'npm manifests: use object repository form to silence publish warning'
type: task
priority: 3
created: '2026-08-23T05:00:02Z'
updated: '2026-08-23T05:00:29Z'
labels:
- dist
---

npm publish --dry-run warns it auto-corrects repository from a string to an object. Use the object form {type:git, url:git+https://github.com/rocketsurgery-games/yaks.git} in npm/yaks/package.json and npm/platform-template/package.json so the release is warning-clean. Cargo.toml keeps its string form (correct for Cargo).

---
▸ 2026-08-23T05:00:29Z
Done. npm/yaks/package.json + npm/platform-template/package.json now use the object repository form; re-ran build-npm + dry-run, warning gone, mapping test still passes. dist/ and artifacts/ are gitignored.
