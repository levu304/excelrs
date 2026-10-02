// Streaming XLSX round-trip smoke test.
// Run after `npx napi build` so the native addon is loadable via ./index.js.
// Covers the `release-verification` and `streaming-xlsx` spec scenarios:
// a workbook written through the streaming writer must read back through the
// streaming reader with every cell value preserved.
//
// Scale: this originally wrote ONE sheet, ONE row, FOUR cells. That is a spot
// check — it cannot reach truncation, corruption, or archive-format limits, so a
// regression that only manifests at size passed. The workbook below spans several
// sheets and tens of thousands of cells while still finishing in ~2.5s, well
// inside the release job's 2-minute step timeout.
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
// ReadableStream path, not `write()`.
'use strict';

const ex = require('../index.js');

const SHEETS = 8;
const ROWS = 4000;
const COLS = 5;
const STR_PAD = 'x'.repeat(40);

// Guards against the test silently shrinking back into a spot check. Roughly
// 8 * 4000 * 5 cells * ~48 chars is ~7.4 MB of cell text; the floor leaves
// headroom for format changes while still failing a regression to 1 row.
const PAYLOAD_FLOOR_BYTES = 4 * 1024 * 1024;

function buildSheets() {
  const sheets = [];
  let payload = 0;
  for (let s = 0; s < SHEETS; s++) {
    const rows = [];
    for (let r = 1; r <= ROWS; r++) {
      const cells = [];
      for (let c = 1; c <= COLS; c++) {
        const text = `s${s}r${r}c${c}${STR_PAD}`;
        payload += text.length;
        // Mix value shapes so the round-trip covers more than strings.
        const value = c === 1 ? { number: s * ROWS + r } : { text };
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

  // Exhaustive fidelity: every sheet, every row, every cell. A sampled or
  // first-row-only check would pass a writer that truncates late in the stream.
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
        const cell = row.cells[c - 1];
        if (c === 1) {
          const want = s * ROWS + r;
          if (cell.value.number !== want) {
            throw new Error(
              `sheet ${s} row ${r} col ${c}: number lost (got ${cell.value.number}, want ${want})`
            );
          }
        } else {
          const want = `s${s}r${r}c${c}${STR_PAD}`;
          if (cell.value.text !== want) {
            throw new Error(`sheet ${s} row ${r} col ${c}: text lost`);
          }
        }
        checked++;
      }
    }
  }

  const elapsed = Date.now() - started;
  console.log(
    `OK streaming writer -> reader round-trip preserved ${checked} cells ` +
      `across ${SHEETS} sheets (payload ${(payload / 1048576).toFixed(1)}MB, ` +
      `xlsx ${(buf.length / 1048576).toFixed(1)}MB) in ${elapsed}ms`
  );
}

main().catch((e) => {
  console.error('FAIL:', e.message);
  process.exit(1);
});