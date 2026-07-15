import { test, expect } from 'vitest'
import ExcelJS from 'exceljs'
import { Workbook } from '../index'

// ---------------------------------------------------------------------------
// Async contract — read/write return Promise, state only swapped after await
// ---------------------------------------------------------------------------

test('wb.xlsx.read returns Promise (async contract)', async () => {
  const wbjs = new ExcelJS.Workbook()
  wbjs.addWorksheet('Sheet1')
  wbjs.getWorksheet('Sheet1')!.getCell('A1').value = 42
  const buf = await wbjs.xlsx.writeBuffer()

  const wb = new Workbook()
  const result = wb.xlsx.read(buf as never)

  expect(result).toBeInstanceOf(Promise)
  await result
})

test('wb.xlsx.write returns Promise (async contract)', async () => {
  const wb = new Workbook()
  wb.addWorksheet('Test')

  const result = wb.xlsx.write()

  expect(result).toBeInstanceOf(Promise)
  await result
})

test('wb.xlsx.read without await leaves worksheetCount stale, resolves', async () => {
  // Build a buffer with one sheet via exceljs
  const wbjs = new ExcelJS.Workbook()
  wbjs.addWorksheet('Stale')
  wbjs.getWorksheet('Stale')!.getCell('A1').value = 'stale-await-test'
  const buf = await wbjs.xlsx.writeBuffer()

  const wb = new Workbook()
  // Sanity: initially empty
  expect(wb.worksheetCount).toBe(0)

  // Call read but do NOT await yet
  const p = wb.xlsx.read(buf as never)

  // State is still the old value because the Promise hasn't resolved
  // (characterization of required-await semantics)
  expect(wb.worksheetCount).toBe(0)

  // Now await — state swaps
  await p
  expect(wb.worksheetCount).toBe(1)
  expect(wb.getWorksheet('Stale')).toBeDefined()
})

test('async read/write round-trip preserves worksheetCount + getWorksheet + Date', async () => {
  const wb = new Workbook()
  const ws = wb.addWorksheet('Dates')
  ws.getCell('A1').value = new Date('2024-06-15T12:00:00Z')

  const buf = await wb.xlsx.write()

  const wb2 = new Workbook()
  await wb2.xlsx.read(buf)

  expect(wb2.worksheetCount).toBe(1)
  expect(wb2.worksheets.length).toBe(1)

  const cellVal = wb2.getWorksheet('Dates')!.getCell('A1').value
  expect(cellVal).toBeInstanceOf(Date)
  if (cellVal instanceof Date) {
    expect(cellVal.toISOString()).toBe('2024-06-15T12:00:00.000Z')
  }
})
