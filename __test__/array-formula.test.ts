import { test, expect } from 'vitest'
import ExcelJS from 'exceljs'
import { Workbook } from '../index'
import fs from 'fs'
import path from 'path'

// Committed fixture: ExcelJS-authored legacy CSE array (`__test__/fixtures`),
// C1:C3 each carrying `<f t="array" ref="C1:C3">SUM(A1:A3*1)</f><v>6</v>`.
const ARRAY_FIXTURE = path.resolve(__dirname, 'fixtures', 'array-formula.xlsx')

function arrayCells(ws: { getCell(addr: string): { formula: string | null; value: unknown } }) {
  return ['C1', 'C2', 'C3'].map((addr) => ws.getCell(addr))
}

test('array fixture reads as plain formulas with cached values', async () => {
  const wb = new Workbook()
  await wb.xlsx.read(fs.readFileSync(ARRAY_FIXTURE) as never)
  const ws = wb.getWorksheet('Array')!
  for (const cell of arrayCells(ws)) {
    expect(cell.formula).toBe('SUM(A1:A3*1)')
    expect(cell.value).toBe(6)
  }
})

test('array fixture round-trips as plain formulas (array-ness dropped)', async () => {
  const wb = new Workbook()
  await wb.xlsx.read(fs.readFileSync(ARRAY_FIXTURE) as never)
  const buf = await wb.xlsx.write()

  // excelrs re-read: text and cached value preserved.
  const wb2 = new Workbook()
  await wb2.xlsx.read(buf as never)
  for (const cell of arrayCells(wb2.getWorksheet('Array')!)) {
    expect(cell.formula).toBe('SUM(A1:A3*1)')
    expect(cell.value).toBe(6)
  }

  // ExcelJS view: no array shareType survives (proves no t/ref on the <f>),
  // and the file opens without repair.
  const wbjs = new ExcelJS.Workbook()
  await wbjs.xlsx.load(buf as never)
  const wsjs = wbjs.getWorksheet('Array')!
  expect(wsjs.getCell('C1').value).toMatchObject({ formula: 'SUM(A1:A3*1)', result: 6 })
  for (const addr of ['C1', 'C2', 'C3']) {
    const model = (wsjs.getCell(addr) as unknown as { model: { shareType?: string } }).model
    expect(model.shareType).toBeUndefined()
  }
})
