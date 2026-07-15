import { test, expect } from 'vitest'
import ExcelJS from 'exceljs'
import { Workbook } from '../index'

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

/** Build a test .xlsx buffer using exceljs for reference content. */
async function buildTestBuffer() {
  const wb = new ExcelJS.Workbook()
  const ws = wb.addWorksheet('Sheet1')

  // Cell types to test
  ws.getCell('A1').value = 'Hello'           // String
  ws.getCell('B1').value = 42                 // Number (int)
  ws.getCell('C1').value = 3.14              // Number (float)
  ws.getCell('D1').value = true              // Boolean
  ws.getCell('E1').value = false             // Boolean
  ws.getCell('F1').value = null              // Null/Empty

  // Date — exceljs stores as serial date with format code
  ws.getCell('A2').value = new Date(2025, 0, 1) // Jan 1, 2025
  ws.getCell('A2').numFmt = 'yyyy-mm-dd'

  // Formula
  ws.getCell('A3').value = 10
  ws.getCell('B3').value = 20
  ws.getCell('C3').value = { formula: 'SUM(A3:B3)' }

  // Second sheet
  const ws2 = wb.addWorksheet('Data')
  ws2.getCell('A1').value = 'Name'
  ws2.getCell('B1').value = 'Age'

  return wb.xlsx.writeBuffer()
}

// ---------------------------------------------------------------------------
// wb.xlsx.read — full round-trip
// ---------------------------------------------------------------------------

test('readXlsxBuffer reads all sheets and cell types', async () => {
  const buf = await buildTestBuffer()
  const wb = new Workbook()
  await wb.xlsx.read(buf as never)

  expect(wb.worksheetCount).toBe(2)

  // --- Sheet 1: Sheet1 ---
  const ws = wb.getWorksheet('Sheet1')!
  expect(ws).not.toBeNull()

  // String
  const a1 = ws.getCell('A1')
  expect(a1.value.valueType).toBe('String')
  expect(a1.value.string).toBe('Hello')

  // Number (int)
  const b1 = ws.getCell('B1')
  expect(b1.value.valueType).toBe('Number')
  expect(b1.value.number).toBe(42)

  // Number (float)
  const c1 = ws.getCell('C1')
  expect(c1.value.valueType).toBe('Number')
  expect(c1.value.number).toBeCloseTo(3.14, 2)

  // Boolean true
  const d1 = ws.getCell('D1')
  expect(d1.value.valueType).toBe('Boolean')
  expect(d1.value.boolean).toBe(true)

  // Boolean false
  const e1 = ws.getCell('E1')
  expect(e1.value.valueType).toBe('Boolean')
  expect(e1.value.boolean).toBe(false)

  // Null
  const f1 = ws.getCell('F1')
  expect(f1.value.valueType).toBe('Null')

  // Date — now reads as Date type (v0.13.0)
  const a2 = ws.getCell('A2')
  expect(a2.value.valueType).toBe('Date')
  expect(a2.value.dateSerial).toBeGreaterThan(0)
  expect(a2.date).toBeInstanceOf(Date)

  // Formula cell — verify formula string is preserved
  const c3 = ws.getCell('C3')
  expect(c3.formula).toBeTruthy()
  // calamine may return 'SUM(A3:B3)' without '=' prefix or with it
  const formulaStr: string = c3.formula || ''
  expect(formulaStr.toUpperCase()).toContain('SUM')
  expect(formulaStr.toUpperCase()).toContain('A3')

  // --- Sheet 2: Data ---
  const ws2 = wb.getWorksheet('Data')!
  expect(ws2).not.toBeNull()
  expect(ws2.getCell('A1').value.string).toBe('Name')
  expect(ws2.getCell('B1').value.string).toBe('Age')
})

// ---------------------------------------------------------------------------
// wb.xlsx.read — empty workbook
// ---------------------------------------------------------------------------

test('readXlsxBuffer handles empty worksheets', async () => {
  const wbjs = new ExcelJS.Workbook()
  wbjs.addWorksheet('Empty')
  const buf = await wbjs.xlsx.writeBuffer()

  const wb = new Workbook()
  await wb.xlsx.read(buf as never)
  expect(wb.worksheetCount).toBe(1)
  const ws = wb.getWorksheet('Empty')!
  expect(ws.name).toBe('Empty')
  expect(ws.rowCount).toBe(0)
})

// ---------------------------------------------------------------------------
// wb.xlsx.read — invalid data
// ---------------------------------------------------------------------------

test('readXlsxBuffer throws on invalid data', async () => {
  const wb = new Workbook()
  await expect(wb.xlsx.read(Buffer.from('not an xlsx'))).rejects.toThrow()
})

// ---------------------------------------------------------------------------
// wb.xlsx.readFile — requires an actual file on disk
// ---------------------------------------------------------------------------

test('readXlsxFile reads from file path', async () => {
  const wbjs = new ExcelJS.Workbook()
  const ws = wbjs.addWorksheet('FileTest')
  ws.getCell('A1').value = 99
  const buf = await wbjs.xlsx.writeBuffer()

  // Write to temp file
  const fs = await import('fs')
  const os = await import('os')
  const path = await import('path')
  const tmpFile = path.join(os.tmpdir(), `excelrs-test-${Date.now()}.xlsx`)
  fs.writeFileSync(tmpFile, buf as never)

  const wb = new Workbook()
  try {
    await wb.xlsx.readFile(tmpFile)
    expect(wb.worksheetCount).toBe(1)
    expect(wb.getWorksheet('FileTest')!.getCell('A1').value.number).toBe(99)
  } finally {
    try { fs.unlinkSync(tmpFile) } catch { /* ignore */ }
  }
})

// ---------------------------------------------------------------------------
// Edge cases
// ---------------------------------------------------------------------------

test('readXlsxBuffer handles multi-cell formula references', async () => {
  const wbjs = new ExcelJS.Workbook()
  const ws = wbjs.addWorksheet('FormulaTest')

  // Create cells with a range formula
  for (let i = 1; i <= 5; i++) {
    ws.getCell(`A${i}`).value = i * 10
  }
  ws.getCell('A6').value = { formula: 'SUM(A1:A5)' }

  const buf = await wbjs.xlsx.writeBuffer()
  const wb = new Workbook()
  await wb.xlsx.read(buf as never)

  const cell = wb.getWorksheet('FormulaTest')!.getCell('A6')
  expect(cell.formula).toBeTruthy()
  const f = (cell.formula || '').toUpperCase()
  expect(f).toContain('SUM')
  expect(f).toContain('A1')
})

test('readXlsxBuffer handles long strings', async () => {
  const longStr = 'A'.repeat(1000)
  const wbjs = new ExcelJS.Workbook()
  const ws = wbjs.addWorksheet('LongStr')
  ws.getCell('A1').value = longStr

  const buf = await wbjs.xlsx.writeBuffer()
  const wb = new Workbook()
  await wb.xlsx.read(buf as never)

  const cell = wb.getWorksheet('LongStr')!.getCell('A1')
  expect(cell.value.valueType).toBe('String')
  expect(cell.value.string).toBe(longStr)
})
