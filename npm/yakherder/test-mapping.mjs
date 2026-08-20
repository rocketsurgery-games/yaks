// Pure unit test for the launcher's host->package mapping. Run: node test-mapping.mjs
import { createRequire } from "node:module";
import assert from "node:assert/strict";
const require = createRequire(import.meta.url);
const { pkgFor, TARGETS } = require("./bin/yaks.js");

assert.equal(pkgFor("darwin", "arm64"), "yakherder-darwin-arm64");
assert.equal(pkgFor("linux", "x64"), "yakherder-linux-x64");
assert.equal(pkgFor("win32", "x64"), "yakherder-win32-x64");
assert.equal(pkgFor("sunos", "sparc"), null);
// every mapped package is also declared as an optionalDependency
const pkg = require("./package.json");
for (const name of Object.values(TARGETS)) {
  assert.ok(pkg.optionalDependencies[name], `missing optionalDependency: ${name}`);
}
console.log("ok: launcher mapping + optionalDependencies consistent");
