// Pre-publish round-trip assertion for the release `publish` job.
//
// Proves the same three guarantees as the shared module — cell style, merged
// range, row style — but against the binary this job is about to publish, BEFORE
// any `npm publish` runs. That ordering is the point: a style-loss regression
// must fail the release while nothing is on npm yet, not after.
//
// Loads through the hand-maintained index.js glue (via
// NAPI_RS_NATIVE_LIBRARY_PATH) rather than requiring the .node directly, because
// the glue is the published entrypoint; asserting against it also catches a glue
// regression that a direct require would miss.
//
// NAPI_RS_NATIVE_LIBRARY_PATH is a *preference*, not a pin: index.js catches a
// failure to require that path and falls through to ./excelrs.<platform>.node and
// then to the @levu304/* platform package. If either of those ever exists in the
// publish job, this gate would silently assert against a different binary than
// the one being published — passing on the wrong artifact. `assertLoadedFrom`
// below makes that failure loud instead.
//
// The publish job runs on ubuntu-22.04, so the binary it can load is the
// linux-x64-gnu one. Per-target verification of the other five matrix targets
// lives in the `build` job, which runs each target natively.
//
// Usage: node scripts/prepublish-smoke.cjs <path-to-.node>
'use strict';

const fs = require('fs')
const path = require('path')

const { assertRoundTrip } = require('./assert-roundtrip.cjs')

const binary = process.argv[2]
if (!binary) {
  console.error('Usage: node scripts/prepublish-smoke.cjs <path-to-.node>')
  process.exit(2)
}

const abs = path.resolve(binary)

// Fail before loading anything if the artifact is not there. index.js's fallback
// chain would otherwise turn a missing binary into a confusing "Cannot find
// native binding" from deep inside the loader.
if (!fs.existsSync(abs)) {
  console.error('FAIL: no binary at ' + abs)
  process.exit(1)
}

// Load the artifact under test and prove the glue resolved the SAME module.
//
// Requiring `abs` ourselves first gives a definitive answer that does not depend
// on parsing index.js's fallback order: if the glue ends up bound to a different
// module object, it took a fallback path.
//
// Everything lives inside main() so a load failure is reported as this gate
// failing, rather than escaping as a raw dlopen stack that reads like a harness
// bug instead of a bad artifact.
function assertLoadedFrom() {
  let native
  try {
    native = require(abs)
  } catch (err) {
    throw new Error('the binary under test failed to load: ' + err.message)
  }
  const resolvedNative = require.resolve(abs)

  process.env.NAPI_RS_NATIVE_LIBRARY_PATH = abs
  const glue = require('../index.js')

  if (glue.Workbook !== native.Workbook) {
    throw new Error(
      'index.js did not load the binary under test. Expected the binding from ' +
        resolvedNative +
        ', but the glue resolved a different module. The gate would be asserting ' +
        'against an artifact this job is not publishing.'
    )
  }

  const cached = Object.keys(require.cache).filter((k) => k.endsWith('.node'))
  const unexpected = cached.filter((k) => path.resolve(k) !== resolvedNative)
  if (unexpected.length > 0) {
    throw new Error(
      'a different native binary was loaded alongside the one under test: ' +
        unexpected.join(', ')
    )
  }

  return glue.Workbook
}

async function main() {
  console.log('Pre-publish round-trip against ' + abs)
  const Workbook = assertLoadedFrom()
  console.log('OK binding under test is the one the glue loaded')
  await assertRoundTrip(Workbook, (m) => console.log(m))
  console.log('PASS pre-publish assertions; safe to publish')
}

main().catch((err) => {
  console.error('FAIL:', err.message)
  process.exit(1)
})
