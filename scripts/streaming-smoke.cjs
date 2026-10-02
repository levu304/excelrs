// Streaming XLSX round-trip smoke test.
// Run after `npx napi build` so the native addon is loadable via ./index.js.
// Covers the `release-verification` and `streaming-xlsx` spec scenarios:
// a workbook written through the streaming writer must read back through the
// streaming reader with every cell value preserved.
//
// Scale: this originally wrote ONE sheet, ONE row, FOUR cells. That is a spot
// check — it cannot reach truncation, corruption, or archive-format limits, so a
// regression that only manifests at size passed. The workbook below spans several
// sheets and hundreds of thousands of cells while still finishing in a few
// seconds, well inside the release job's step timeout.
//
// Value shapes: all four shapes the streaming API accepts are written and
// asserted — number, text, boolean, formula. Scaling the assertion up must not
// narrow the set of value types it exercises; an earlier revision of this script
// grew the workbook but quietly dropped boolean and formula, leaving
// `StreamValue::Formula` with no release-path coverage at all.
//
// Deliberately NOT asserted: a memory bound. Measured against this build,
// `write()`'s heap growth is O(total payload) and flat in sheet count (5.5 MB at
// 2 sheets vs 5.2 MB at 128 sheets at fixed payload), so aggregate JS-visible heap
// is not O(one sheet) and no bounded-output property exists to assert. The input
// phase's O(all sheets) behaviour is stated by `openspec/specs/streaming-xlsx` and
// true incremental `writeSheet()` is deferred to ADR-028. The bounded cap-16
// backpressure channel belongs to `finalizeToReadable` (`streaming-write-to-readable`),
// which this script does not exercise.
//
// ponytail: ceiling — PAYLOAD_FLOOR_BYTES is a size floor, not a memory ceiling.
// It guarantees the test is not a spot check; it says nothing about peak RSS. If a
// real memory regression ever needs catching, the observable is the cap-16
// ReadableStream path, not `write()`. The floor is measured against the content
// each cell actually emits, so it cannot be satisfied by text that is never
// written.
'use strict';

// A load failure here is the common case on a mis-built or wrong-libc artifact, and
// index.js reports it as "Cannot find native binding ... npm has a bug related to
// optional dependencies", which sends the reader hunting through npm instead of at
// the build. Name the actual cause.
let ex
try {
  ex = require('../index.js')
} catch (err) {
  console.error(
    'FAIL: the native binding did not load, so no round-trip was attempted.\n' +
      '  ' + String(err.message).split('\n')[0] + '\n' +
      '  This is a build/artifact problem (wrong target, stale binary, or a libc ' +
      'mismatch), not a round-trip failure.'
  )
  process.exit(1)
}

const SHEETS = 8;
const ROWS = 6000;
const COLS = 5;
const STR_PAD = 'x'.repeat(40);

// Every build-matrix target runs this. The step timeout is 5 minutes; measured
// runtime on a macOS arm64 dev box is a few seconds, so this is generous headroom
// for slower Windows and emulated-musl runners rather than a target to tune.
const EXPECTED_RUNTIME_MS = 10_000;

// Guards against the test silently shrinking back into a spot check. Counted from
// the content each cell actually emits (see `emittedLength`), so shrinking ROWS
// fails this rather than quietly passing. Leaves headroom for format changes.
const PAYLOAD_FLOOR_BYTES = 4 * 1024 * 1024;

// The bytes a single cell contributes to the workbook. Measured per value shape
// so the floor tracks real output: a cell written as a number emits digits, not
// the template string that would be used if it were written as text.
function emittedLength(value) {
  if (value.text !== undefined) return String(value.text).length;
  if (value.formula !== undefined) return String(value.formula).length;
  if (value.boolean !== undefined) return String(value.boolean).length;
  if (value.number !== undefined) return String(value.number).length;
  return 0;
}

// col -> value for a cell. Exercises every shape the streaming API accepts.
function cellValue(s, r, c) {
  switch (c) {
    case 1:
      return { number: s * ROWS + r };
    case 2:
      return { text: `s${s}r${r}c${c}${STR_PAD}` };
    case 3:
      return { boolean: (s + r) % 2 === 0 };
    case 4:
      return { text: `s${s}r${r}c${c}${STR_PAD}` };
    default:
      return { formula: `B${r}&C${r}` };
  }
}

function buildSheets() {
  const sheets = [];
  let payload = 0;
  for (let s = 0; s < SHEETS; s++) {
    const rows = [];
    for (let r = 1; r <= ROWS; r++) {
      const cells = [];
      for (let c = 1; c <= COLS; c++) {
        const value = cellValue(s, r, c);
        payload += emittedLength(value);
        cells.push({ col: c, value });
      }
      rows.push({ r, cells });
    }
    sheets.push({ name: `s${s}`, rows });
  }
  return { sheets, payload };
}

async function main() {
  const started = Date.now();
  const { sheets, payload } = buildSheets();

  if (payload < PAYLOAD_FLOOR_BYTES) {
    throw new Error(
      'streaming smoke payload ' + payload + ' is below the floor ' +
        PAYLOAD_FLOOR_BYTES + ' — this would be a spot check'
    );
  }

  const buf = await new ex.Workbook().stream.xlsx.write(sheets);
  if (!buf || buf.length < 100) {
    throw new Error('stream write produced an empty buffer');
  }

  const out = await new ex.Workbook().stream.xlsx.read(buf);
  if (!out || out.length !== SHEETS) {
    throw new Error('expected ' + SHEETS + ' sheets, got ' + (out ? out.length : 0));
  }

  // Exhaustive fidelity: every sheet, every row, every cell, every value shape.
  // A sampled or first-row-only check would pass a writer that truncates late in
  // the stream, or one that drops a whole value type at scale.
  let checked = 0;
  for (let s = 0; s < SHEETS; s++) {
    const got = out[s];
    if (got.name !== `s${s}`) {
      throw new Error(`sheet ${s}: name lost (got ${got.name})`);
    }
    if (!got.rows || got.rows.length !== ROWS) {
      throw new Error(
        `sheet ${s}: expected ${ROWS} rows, got ${got.rows ? got.rows.length : 0}`
      );
    }
    for (let r = 1; r <= ROWS; r++) {
      const row = got.rows[r - 1];
      if (row.r !== r) throw new Error(`sheet ${s}: row ${r} out of order`);
      if (!row.cells || row.cells.length !== COLS) {
        throw new Error(`sheet ${s} row ${r}: expected ${COLS} cells`);
      }
      for (let c = 1; c <= COLS; c++) {
        const expected = cellValue(s, r, c);
        const actual = row.cells[c - 1].value;
        for (const shape of ['number', 'text', 'boolean', 'formula']) {
          if (expected[shape] === undefined) {
            // A shape that was not written must not appear on read-back either:
            // a writer that silently coerced a number into text would land here.
            if (actual && actual[shape] !== undefined && actual[shape] !== null) {
              throw new Error(
                `sheet ${s} row ${r} col ${c}: unexpected ${shape} on read-back ` +
                  `(got ${actual[shape]}, wrote ${JSON.stringify(expected)})`
              );
            }
            continue;
          }
          if (actual[shape] !== expected[shape]) {
            throw new Error(
              `sheet ${s} row ${r} col ${c}: ${shape} lost ` +
                `(got ${JSON.stringify(actual[shape])}, want ${JSON.stringify(expected[shape])})`
            );
          }
        }
        checked++;
      }
    }
  }

  // Every shape must actually have been exercised, or the loop above proved
  // nothing about the shapes it skipped.
  const shapesSeen = new Set();
  for (const sheet of sheets) {
    for (const cell of sheet.rows[0].cells) {
      for (const shape of ['number', 'text', 'boolean', 'formula']) {
        if (cell.value[shape] !== undefined) shapesSeen.add(shape);
      }
    }
  }
  for (const shape of ['number', 'text', 'boolean', 'formula']) {
    if (!shapesSeen.has(shape)) {
      throw new Error(`generator wrote no ${shape} cells — that shape is uncovered`);
    }
  }

  const elapsed = Date.now() - started;
  console.log(
    `OK streaming writer -> reader round-trip preserved ${checked} cells ` +
      `across ${SHEETS} sheets, all ${shapesSeen.size} value shapes ` +
      `(payload ${(payload / 1048576).toFixed(1)}MB emitted, ` +
      `xlsx ${(buf.length / 1048576).toFixed(1)}MB) in ${elapsed}ms`
  );

  if (elapsed > EXPECTED_RUNTIME_MS) {
    console.warn(
      `WARN streaming smoke took ${elapsed}ms, over the ${EXPECTED_RUNTIME_MS}ms ` +
        'this script assumes. Not a failure, but re-check the step timeout.'
    );
  }
}

main().catch((e) => {
  console.error('FAIL:', e.message);
  process.exit(1);
});
