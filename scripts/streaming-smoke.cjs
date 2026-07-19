// Streaming XLSX round-trip smoke test.
// Run after `npx napi build` so the native addon is loadable via ./index.js.
// Covers the `release-verification` and `streaming-xlsx` spec scenarios:
// a workbook written through the streaming writer must read back through the
// streaming reader with cell values preserved.
'use strict';

const ex = require('./index.js');

async function main() {
  const wb = new ex.Workbook();
  const sheets = [
    {
      name: 's',
      rows: [
        {
          r: 1,
          cells: [
            { col: 1, value: { number: 1 } },
            { col: 2, value: { text: 'a' } },
            { col: 3, value: { boolean: true } },
            { col: 4, value: { formula: 'B1&C1' } },
          ],
        },
      ],
    },
  ];

  const buf = await wb.stream.xlsx.write(sheets);
  if (!buf || buf.length < 100) {
    throw new Error('stream write produced an empty buffer');
  }

  const out = await wb.stream.xlsx.read(buf);
  if (!out || out.length !== 1) {
    throw new Error('expected 1 sheet, got ' + (out ? out.length : 0));
  }

  const cells = out[0].rows[0].cells;
  if (cells[0].value.number !== 1) throw new Error('number lost');
  if (cells[1].value.text !== 'a') throw new Error('text lost');
  if (cells[2].value.boolean !== true) throw new Error('boolean lost');
  if (cells[3].value.formula !== 'B1&C1') throw new Error('formula lost');

  console.log('OK streaming writer -> reader round-trip preserves values');
}

main().catch((e) => {
  console.error('FAIL:', e.message);
  process.exit(1);
});