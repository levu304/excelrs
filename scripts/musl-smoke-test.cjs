// musl native binding smoke test.
// Usage: node scripts/musl-smoke-test.cjs <path-to-.node>
// Loads a musl-built .node binary, round-trips a styled workbook
// (write + read back), and asserts style preservation.
'use strict';

const nodePath = process.argv[2];
if (!nodePath) {
  console.error('Usage: node scripts/musl-smoke-test.cjs <path-to-.node>');
  process.exit(2);
}

const { Workbook } = require(nodePath);

async function main() {
  const wb = new Workbook();
  const ws = wb.addWorksheet('test');
  ws.addRow([1, 2, 3]);
  ws.addRow(['x', 'y', 'z']);
  ws.addRow(['p', 'q', 'r']);

  // Cell-level style: font.bold + fill
  ws.setCellStyle(1, 2, {
    font: { bold: true },
    fill: { kind: 'Solid', foreground: 'FFFF0000' },
  });

  // Row-level style
  ws.getRow(3).style = {
    font: { bold: true, color: 'FFFF0000' },
    fill: { kind: 'Solid', foreground: 'FFFFFFFF' },
  };

  // Merge range
  ws.mergeCells('B2:D2');

  const buf = await wb.xlsx.write();
  if (buf.length < 100) {
    throw new Error('xlsx too small: ' + buf.length);
  }
  console.log('OK native binding works, xlsx size: ' + buf.length);

  const wb2 = new Workbook();
  await wb2.xlsx.read(buf);
  const ws2 = wb2.getWorksheet('test');

  // Assert cell style preserved (font.bold and fill.foreground)
  const s = ws2.getCellByRc(1, 2).style;
  if (!s || !s.font || s.font.bold !== true) {
    throw new Error('FAIL: font.bold not preserved on read-back');
  }
  if (!s.fill || s.fill.kind !== 'Solid' || s.fill.foreground !== 'FFFF0000') {
    throw new Error(
      'FAIL: fill not preserved on read-back (got ' +
        (s.fill && s.fill.kind) +
        '/' +
        (s.fill && s.fill.foreground) +
        ')',
    );
  }
  console.log('OK cell style (font.bold + fill) round-trips');

  // Assert merged range preserved
  const merges = ws2.mergedRanges || [];
  if (!merges.includes('B2:D2')) {
    throw new Error('FAIL: merged range lost on read-back');
  }
  console.log('OK merged range round-trips');

  // Assert row style preserved
  const rs = ws2.getRow(3).style;
  if (!rs || !rs.font || rs.font.bold !== true || rs.font.color !== 'FFFF0000') {
    throw new Error('FAIL: row font.bold/color not preserved on read-back');
  }
  if (!rs.fill || rs.fill.foreground !== 'FFFFFFFF') {
    throw new Error('FAIL: row fill not preserved on read-back');
  }
  console.log('OK row style round-trips');
}

main().catch((err) => {
  console.error('FAIL:', err.message);
  process.exit(1);
});