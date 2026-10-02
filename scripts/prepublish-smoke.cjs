// Pre-publish round-trip assertion for the release `publish` job.
//
// Proves the same three guarantees as the post-publish registry check — cell style,
// merged range, row style — but against the binary this job is about to publish,
// BEFORE any `npm publish` runs. That ordering is the point: a style-loss
// regression must fail the release while nothing is on npm yet, not after.
//
// Loads through the hand-maintained index.js glue (via
// NAPI_RS_NATIVE_LIBRARY_PATH) rather than requiring the .node directly, because
// the glue is the published entrypoint; asserting against it also catches a glue
// regression that a direct require would miss.
//
// The publish job runs on ubuntu-22.04, so the binary it can load is the
// linux-x64-gnu one. Per-target verification of the other five matrix targets
// lives in the `build` job, which runs each target natively.
//
// Usage: node scripts/prepublish-smoke.cjs <path-to-.node>
'use strict'

const path = require('path')

const { assertRoundTrip } = require('./assert-roundtrip.cjs')

const binary = process.argv[2]
if (!binary) {
  console.error('Usage: node scripts/prepublish-smoke.cjs <path-to-.node>')
  process.exit(2)
}

const abs = path.resolve(binary)
process.env.NAPI_RS_NATIVE_LIBRARY_PATH = abs

const { Workbook } = require('../index.js')

async function main() {
  console.log('Pre-publish round-trip against ' + abs)
  await assertRoundTrip(Workbook, (m) => console.log(m))
  console.log('PASS pre-publish assertions; safe to publish')
}

main().catch((err) => {
  console.error('FAIL:', err.message)
  process.exit(1)
})