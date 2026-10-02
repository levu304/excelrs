// musl native binding smoke test.
// Usage: node scripts/musl-smoke-test.cjs <path-to-.node>
// Loads a musl-built .node binary and asserts the release-verification
// round-trip guarantees (style, merges, row style). The assertions are shared
// with scripts/prepublish-smoke.cjs so the two release paths cannot drift.
'use strict';

const { assertRoundTrip } = require('./assert-roundtrip.cjs');

const nodePath = process.argv[2];
if (!nodePath) {
  console.error('Usage: node scripts/musl-smoke-test.cjs <path-to-.node>');
  process.exit(2);
}

const { Workbook } = require(nodePath);

async function main() {
  await assertRoundTrip(Workbook, (m) => console.log(m));
}

main().catch((err) => {
  console.error('FAIL:', err.message);
  process.exit(1);
});