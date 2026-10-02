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

// A musl binary loads only in a process whose arch matches the binary. The
// container's arch is whatever `docker run` resolved, so a runner/image/target
// disagreement lands here — and musl's loader reports it as `unsupported
// relocation type N`, which names a relocation, not an architecture. Name the
// artifact and both arches instead of leaking a raw loader stack. This is the
// Alpine path; prepublish-smoke.cjs gets the same treatment for the same reason.
let Workbook;
try {
  Workbook = require(nodePath).Workbook;
} catch (err) {
  // The arch hint only applies when the file was found and then failed to
  // relocate; a missing file is a different mistake and would be misdirected
  // by it.
  const hint =
    err.code === 'MODULE_NOT_FOUND'
      ? '  The artifact was not present at that path.'
      : '  A musl binary loads only in a container whose arch matches the ' +
        'binary. Check that the runner, the docker image, and the built ' +
        'target agree.';
  console.error(
    `FAIL: ${nodePath} did not load in this container.\n` +
      `  container process.arch: ${process.arch}\n` +
      `  loader said: ${err.message.split('\n')[0]}\n` +
      hint
  );
  process.exit(1);
}

async function main() {
  await assertRoundTrip(Workbook, (m) => console.log(m));
}

main().catch((err) => {
  console.error('FAIL:', err.message);
  process.exit(1);
});