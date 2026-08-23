---
id: yaksrs-bbfe
title: 'Modernize release CI: Node-24 actions + npm Trusted Publishing (OIDC)'
type: task
priority: 2
created: '2026-08-23T16:47:02Z'
updated: '2026-08-23T16:52:14Z'
labels:
- dist
- ci
---

Bump the GitHub Actions to Node-24-targeting majors (clear the deprecation warnings), and switch npm publishing from a long-lived automation token to OIDC Trusted Publishing (no token, 2FA-bypass token not needed, provenance for free).

---
▸ 2026-08-23T16:52:14Z
Bumped actions to v5 (checkout/setup-node/upload-artifact/download-artifact) to clear the Node-20 deprecation; kept upload+download on the same major so artifacts interoperate. Wired OIDC Trusted Publishing: publish job gets id-token: write, Node 24 + npm i -g npm@latest (>=11.5.1 floor), NODE_AUTH_TOKEN kept only as bootstrap fallback (CLI does OIDC-first). RELEASING.md documents the per-package trusted-publisher config + one-time bootstrap. Validating via dry-run.
