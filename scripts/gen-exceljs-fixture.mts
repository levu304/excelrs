/** Generate the ExcelJS-authored cached-formula fixture.
 *  ExcelJS emits <f>..</f><v>..</v> when a cell value carries `formula` + `result`.
 *  See node_modules/exceljs/lib/xlsx/xform/sheet/cell-xform.js (getValueType(model.result)).
 */
import ExcelJS from 'exceljs';
import fs from 'fs';

const out = process.argv[2];
if (!out) { console.error('usage: tsx gen-exceljs-fixture.ts <out.xlsx>'); process.exit(1); }

const wb = new ExcelJS.Workbook();
const ws = wb.addWorksheet('Sheet1');

// Reference values so the formula A2+B2 is meaningful.
ws.getCell('A2').value = 10;
ws.getCell('B2').value = 20;

// ExcelJS-authored cached formula cells. `result` is the cached value;
// exceljs writes <f>{formula}</f><v>{result}</v>.
ws.getCell('A1').value = { formula: 'A2+B2', result: 30, date1904: false } as never;
// string cached result
ws.getCell('C1').value = { formula: 'CONCAT("a","b")', result: 'ab' } as never;
// boolean cached result
ws.getCell('D1').value = { formula: 'A2>B2', result: false } as never;
// error cached result
ws.getCell('E1').value = { formula: '1/0', result: { error: '#DIV/0!', errorCode: '#DIV/0!' } } as never;
// date cached result — ExcelJS serializes Date via dateToExcel serial in <v>
ws.getCell('F1').value = { formula: 'DATE(2025,1,1)', result: new Date(Date.UTC(2025, 0, 1)) } as never;

wb.xlsx.writeFile(out).then(() => {
  console.log(`wrote ${out} (${fs.statSync(out).size} bytes)`);
});