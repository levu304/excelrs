import { test, expect } from 'vitest'

import { Workbook } from '../index'

import fs from 'fs'
import path from 'path'

// Path to committed fixtures for this change.
const CHANGE_FIXTURES = path.resolve(
  __dirname,
  'fixtures',
)

function makeWorkbook() {
  const wb = new Workbook()
  wb.addWorksheet('S')
  return wb
}

/** Read back the first worksheet of an .xlsx buffer via excelrs. */
async function readXlsx(buf: Buffer) {
  const wb = new Workbook()
  await wb.xlsx.read(buf as never)
  return wb.getWorksheet('Sheet1') ?? wb.getWorksheet('S')!
}

// ---------------------------------------------------------------------------
// 4.1 ExcelJS-authored cached formula round-trips (numeric)
// ---------------------------------------------------------------------------

test('exceljs-authored cached formula: cell.value === cached number, formula preserved', async () => {
  const buf = fs.readFileSync(path.join(CHANGE_FIXTURES, 'exceljs-cached-formula.xlsx'))
  const ws = await readXlsx(buf)

  // A1 = { formula: "A2+B2", result: 30 } → ExcelJS wrote <f>A2+B2</f><v>30</v>
  const a1 = ws.getCell('A1')
  expect(a1.type).toBe('Formula')
  expect(a1.formula).toBe('A2+B2')
  // Cached scalar surfaces as bare typed value (spec §ADDED Req 1 scenario).
  expect(a1.value).toBe(30)
  expect(typeof a1.value).toBe('number')
})

// ---------------------------------------------------------------------------
// 4.2 Boolean / string / error / date cached scalars round-trip
// ---------------------------------------------------------------------------

test('exceljs-authored cached boolean round-trips as bare boolean', async () => {
  const buf = fs.readFileSync(path.join(CHANGE_FIXTURES, 'exceljs-cached-formula.xlsx'))
  const ws = await readXlsx(buf)

  // D1 = { formula: "A2>B2", result: false } → <f>A2>B2</f><v>0</v>
  const d1 = ws.getCell('D1')
  expect(d1.formula).toBeTruthy()
  expect(d1.value).toBe(false)
})

test('exceljs-authored cached string round-trips as bare string', async () => {
  const buf = fs.readFileSync(path.join(CHANGE_FIXTURES, 'exceljs-cached-formula.xlsx'))
  const ws = await readXlsx(buf)

  // C1 = { formula: CONCAT("a","b"), result: "ab" } → <f>..</f><v>ab</v>
  const c1 = ws.getCell('C1')
  expect(c1.formula).toBeTruthy()
  expect(c1.value).toBe('ab')
})

test('exceljs-authored cached error round-trips as bare string', async () => {
  const buf = fs.readFileSync(path.join(CHANGE_FIXTURES, 'exceljs-cached-formula.xlsx'))
  const ws = await readXlsx(buf)

  // E1 = { formula: "1/0", result: #DIV/0! } → <f>1/0</f><v>#DIV/0!</v>
  const e1 = ws.getCell('E1')
  expect(e1.formula).toBe('1/0')
  expect(e1.value).toBe('#DIV/0!')
})

test('exceljs-authored cached date round-trips as bare number', async () => {
  const buf = fs.readFileSync(path.join(CHANGE_FIXTURES, 'exceljs-cached-formula.xlsx'))
  const ws = await readXlsx(buf)

  // F1 = { formula: DATE(2025,1,1), result: Date(2025,0,1) }
  // ExcelJS writes the serial as <v>45658</v> with a date style; calamine
  // re-types it via Data::DateTime, so cell.value is a JS Date.
  const f1 = ws.getCell('F1')
  expect(f1.formula).toContain('DATE')
  expect(f1.value).toBeInstanceOf(Date)
  expect((f1.value as Date).getTime()).toBeCloseTo(Date.UTC(2025, 0, 1), -2)
})

// ---------------------------------------------------------------------------
// 4.2 (cont.) JS-authoring round-trip: write through excelrs setter + read back
// ---------------------------------------------------------------------------

test('JS-authored cached number formula round-trips', async () => {
  const wb = makeWorkbook()
  const ws = wb.getWorksheet('S')!
  ws.getCell('B1').value = 10
  ws.getCell('B2').value = 20
  ws.getCell('A1').value = { formula: 'B1+B2', number: 30 } as never
  expect(ws.getCell('A1').formula).toBe('B1+B2')

  const buf = await wb.xlsx.write()
  const ws2 = await readXlsx(buf)
  const c = ws2.getCell('A1')
  expect(c.formula).toBe('B1+B2')
  expect(c.value).toBe(30)
})

test('JS-authored cached boolean formula round-trips', async () => {
  const wb = makeWorkbook()
  const ws = wb.getWorksheet('S')!
  ws.getCell('A1').value = { formula: 'B1>0', boolean: true } as never

  const buf = await wb.xlsx.write()
  const ws2 = await readXlsx(buf)
  const c = ws2.getCell('A1')
  expect(c.formula).toBe('B1>0')
  expect(c.value).toBe(true)
})

test('JS-authored cached string formula round-trips', async () => {
  const wb = makeWorkbook()
  const ws = wb.getWorksheet('S')!
  ws.getCell('A1').value = { formula: 'CONCAT("a","b")', string: 'ab' } as never

  const buf = await wb.xlsx.write()
  const ws2 = await readXlsx(buf)
  const c = ws2.getCell('A1')
  expect(c.formula).toBeTruthy()
  expect(c.value).toBe('ab')
})

test('JS-authored cached error formula round-trips', async () => {
  const wb = makeWorkbook()
  const ws = wb.getWorksheet('S')!
  ws.getCell('A1').value = { formula: '1/0', errorValue: '#DIV/0!' } as never

  const buf = await wb.xlsx.write()
  const ws2 = await readXlsx(buf)
  const c = ws2.getCell('A1')
  expect(c.formula).toBe('1/0')
  expect(c.value).toBe('#DIV/0!')
})

test('JS-authored cached date formula round-trips as bare number', async () => {
  const wb = makeWorkbook()
  const ws = wb.getWorksheet('S')!
  ws.getCell('A1').value = { formula: 'DATE(2025,1,1)', dateSerial: 45657 } as never

  const buf = await wb.xlsx.write()
  const ws2 = await readXlsx(buf)
  const c = ws2.getCell('A1')
  expect(c.formula).toBe('DATE(2025,1,1)')
  // JS-authored dateSerial emits <v>45657</v> with no t attribute and no date
  // number format, so calamine re-types it as Data::Float(45657) and reads
  // back as a bare number (not a JS Date). The ExcelJS-authored variant in §4.2
  // applies a date style and reads back as a Date — this covers the other path.
  expect(c.value).toBe(45657)
  expect(typeof c.value).toBe('number')
})

// ---------------------------------------------------------------------------
// 4.3 Hand-crafted .xlsx reads back cached scalar + formula
// ---------------------------------------------------------------------------

test('hand-crafted cached formula fixture reads back cell.value === 3 and formula', async () => {
  const buf = fs.readFileSync(path.join(CHANGE_FIXTURES, 'hand-cached-formula.xlsx'))
  const ws = await readXlsx(buf)

  const a1 = ws.getCell('A1')
  expect(a1.formula).toBe('A2+B2')
  expect(a1.value).toBe(3)
})

test('hand-crafted fixture reference values are correct', async () => {
  const buf = fs.readFileSync(path.join(CHANGE_FIXTURES, 'hand-cached-formula.xlsx'))
  const ws = await readXlsx(buf)
  expect(ws.getCell('B1').value).toBe(1)
  expect(ws.getCell('B2').value).toBe(2)
})

// ---------------------------------------------------------------------------
// 4.4 No regression: formula authored WITHOUT a cache still reads back
// ---------------------------------------------------------------------------

test('formula without cached value round-trips formula only (cell.value null)', async () => {
  const wb = makeWorkbook()
  const ws = wb.getWorksheet('S')!
  ws.getCell('A1').value = { formula: 'SUM(A1:B1)' } as never

  const buf = await wb.xlsx.write()
  const ws2 = await readXlsx(buf)
  const c = ws2.getCell('A1')
  expect(c.formula).toBe('SUM(A1:B1)')
  // No <v> emitted → no cached scalar → value is null (no regression).
  expect(c.value).toBeNull()
})

// ---------------------------------------------------------------------------
// 4.6 cachedValue getter behavior unchanged (spec §R4)
// ---------------------------------------------------------------------------

test('cachedValue is null for non-Formula cells', async () => {
  const wb = makeWorkbook()
  const ws = wb.getWorksheet('S')!
  ws.getCell('A1').value = 42
  expect(ws.getCell('A1').type).toBe('Number')
  expect(ws.getCell('A1').cachedValue).toBeNull()
})

test('cachedValue is null for uncached formula cells', async () => {
  const wb = makeWorkbook()
  const ws = wb.getWorksheet('S')!
  ws.getCell('A1').value = { formula: 'SUM(A1:B1)' } as never
  const buf = await wb.xlsx.write()
  const ws2 = await readXlsx(buf)
  const c = ws2.getCell('A1')
  expect(c.type).toBe('Formula')
  expect(c.cachedValue).toBeNull()
})

// ---------------------------------------------------------------------------
// 4.7 recalculate() exposure (formula-eval build)
// ---------------------------------------------------------------------------

test('workbook.recalculate resolves cross-sheet references', () => {
  const wb = new Workbook()
  const s1 = wb.addWorksheet('Sheet1')
  const s2 = wb.addWorksheet('Sheet2')
  s2.getCell('A1').value = 42
  s1.getCell('B1').value = { formula: 'Sheet2!A1' } as never
  // Not evaluated yet — no cached scalar.
  expect(s1.getCell('B1').value).toBeNull()

  wb.recalculate()

  // Cross-sheet ref resolves because workbook context is supplied.
  expect(s1.getCell('B1').value).toBe(42)
  expect(s1.getCell('B1').cachedValue).toEqual({
    valueType: 'Number',
    number: 42,
  })
})

test('workbook.recalculate caches every formula cell across all sheets', () => {
  const wb = new Workbook()
  const s1 = wb.addWorksheet('S1')
  const s2 = wb.addWorksheet('S2')
  s1.getCell('A1').value = { formula: '1+2' } as never
  s2.getCell('B2').value = { formula: '4*5' } as never

  wb.recalculate()

  expect(s1.getCell('A1').value).toBe(3)
  expect(s2.getCell('B2').value).toBe(20)
})

test('worksheet.recalculate populates single-sheet cached values', () => {
  const wb = makeWorkbook()
  const ws = wb.getWorksheet('S')!
  ws.getCell('A1').value = { formula: '1+2' } as never
  expect(ws.getCell('A1').value).toBeNull()

  ws.recalculate()

  expect(ws.getCell('A1').value).toBe(3)
})

test('worksheet.recalculate: cross-sheet ref caches #REF! and recalc still completes', () => {
  const wb = makeWorkbook()
  const ws = wb.getWorksheet('S')!
  ws.getCell('A1').value = { formula: '1+2' } as never
  ws.getCell('B1').value = { formula: 'Other!A1' } as never

  // Must not throw — per-cell error isolation, no abort.
  ws.recalculate()

  expect(ws.getCell('A1').value).toBe(3)
  expect(ws.getCell('B1').value).toBe('#REF!')
})