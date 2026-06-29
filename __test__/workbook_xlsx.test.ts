import { test, expect } from 'vitest'
import ExcelJS from 'exceljs'
import { Workbook } from '../index'

// ---------------------------------------------------------------------------
// Workbook.xlsx read — end-to-end (Phase 2 regression)
// ---------------------------------------------------------------------------

test('wb.xlsx.read populates workbook from buffer', async () => {
  const wbjs = new ExcelJS.Workbook()
  wbjs.addWorksheet('TestSheet')
  wbjs.getWorksheet('TestSheet')!.getCell('A1').value = 42
  const buf: Buffer = await wbjs.xlsx.writeBuffer() as Buffer

  const wb = new Workbook()
  await wb.xlsx.read(buf)

  expect(wb.worksheetCount).toBe(1)
  expect(wb.getWorksheet('TestSheet')).toBeDefined()
  expect(wb.getWorksheet('TestSheet')!.getCell('A1').value.number).toBe(42)
})

test('wb.xlsx.readFile populates workbook from file', async () => {
  const wbjs = new ExcelJS.Workbook()
  wbjs.addWorksheet('FileSheet')
  wbjs.getWorksheet('FileSheet')!.getCell('B2').value = 'file test'
  const buf: Buffer = await wbjs.xlsx.writeBuffer() as Buffer

  const fs = await import('fs')
  const os = await import('os')
  const path = await import('path')
  const tmpFile = path.join(os.tmpdir(), `excelrs-xlsx-test-${Date.now()}.xlsx`)
  fs.writeFileSync(tmpFile, buf)

  const wb = new Workbook()
  try {
    await wb.xlsx.readFile(tmpFile)
    expect(wb.worksheetCount).toBe(1)
    expect(wb.getWorksheet('FileSheet')!.getCell('B2').value.string).toBe('file test')
  } finally {
    try { fs.unlinkSync(tmpFile) } catch { /* ignore */ }
  }
})

// ---------------------------------------------------------------------------
// Workbook.xlsx write — now implemented
// ---------------------------------------------------------------------------

test('wb.xlsx.write returns a non-empty buffer', async () => {
  const wb = new Workbook()
  wb.addWorksheet('Test')
  const buf = await wb.xlsx.write()
  expect(buf).toBeInstanceOf(Buffer)
  expect(buf.length).toBeGreaterThan(200)
})

test('wb.xlsx.writeFile writes to disk', async () => {
  const wb = new Workbook()
  wb.addWorksheet('SheetA')
  const fs = await import('fs')
  const os = await import('os')
  const path = await import('path')
  const tmpFile = path.join(os.tmpdir(), `excelrs-write-${Date.now()}.xlsx`)

  await wb.xlsx.writeFile(tmpFile)
  expect(fs.existsSync(tmpFile)).toBe(true)
  expect(fs.statSync(tmpFile).size).toBeGreaterThan(200)

  try { fs.unlinkSync(tmpFile) } catch { /* ignore */ }
})

// ---------------------------------------------------------------------------
// Round-trip: excelrs -> write -> read with same excelrs
// ---------------------------------------------------------------------------

test('round-trip excelrs write then read matches data', async () => {
  const wb = new Workbook()
  const ws = wb.addWorksheet('Data')

  // Populate with addRow (mutates worksheet directly, unlike clone-on-read getCell)
  ws.addRow([42, 'hello', true])
  ws.addRow([3.14, 'world'])

  // Write to buffer
  const buf = await wb.xlsx.write()

  // Read back with a new workbook
  const wb2 = new Workbook()
  await wb2.xlsx.read(buf)

  expect(wb2.worksheetCount).toBe(1)
  const ws2 = wb2.getWorksheet('Data')!
  expect(ws2).toBeDefined()

  // Check cell values
  const a1 = ws2.getCell('A1')
  expect(a1.value.valueType).toBe('Number')
  expect(a1.value.number).toBe(42)

  const b1 = ws2.getCell('B1')
  expect(b1.value.valueType).toBe('String')
  expect(b1.value.string).toBe('hello')

  const c1 = ws2.getCell('C1')
  expect(c1.value.valueType).toBe('Boolean')
  expect(c1.value.boolean).toBe(true)

  const a2 = ws2.getCell('A2')
  expect(a2.value.valueType).toBe('Number')
  expect(a2.value.number).toBeCloseTo(3.14, 2)

  const b2 = ws2.getCell('B2')
  expect(b2.value.valueType).toBe('String')
  expect(b2.value.string).toBe('world')
})

// ---------------------------------------------------------------------------
// Round-trip: excelrs -> write -> read with exceljs
// ---------------------------------------------------------------------------

test('round-trip excelrs write then exceljs read matches', async () => {
  const wb = new Workbook()
  const ws = wb.addWorksheet('Trip')
  ws.addRow([100, 'test', false])

  const buf = await wb.xlsx.write()

  // Read with exceljs
  const wbjs = new ExcelJS.Workbook()
  await wbjs.xlsx.load(buf)

  expect(wbjs.worksheets.length).toBe(1)
  const wsjs = wbjs.getWorksheet('Trip')!

  const vA1 = (wsjs.getCell('A1').value as any)
  if (typeof vA1 === 'object' && vA1 !== null && 'result' in vA1) {
    expect((vA1 as any).result).toBe(100)
  } else {
    expect(vA1).toBe(100)
  }

  const vB1 = wsjs.getCell('B1').value
  expect(vB1).toBe('test')

  const vC1 = wsjs.getCell('C1').value
  expect(vC1).toBe(false)
})

// ---------------------------------------------------------------------------
// Round-trip: exceljs -> write -> read with excelrs (Phase 2 regression)
// ---------------------------------------------------------------------------

test('round-trip exceljs write then excelrs read matches', async () => {
  const wbjs = new ExcelJS.Workbook()
  const wsjs = wbjs.addWorksheet('Regression')
  wsjs.getCell('A1').value = 99
  wsjs.getCell('B1').value = 'keep'

  const buf: Buffer = await wbjs.xlsx.writeBuffer() as Buffer

  const wb = new Workbook()
  await wb.xlsx.read(buf)

  expect(wb.worksheetCount).toBe(1)
  const ws = wb.getWorksheet('Regression')!
  expect(ws.getCell('A1').value.number).toBe(99)
  expect(ws.getCell('B1').value.string).toBe('keep')
})

// ---------------------------------------------------------------------------
// Multi-sheet round-trip
// ---------------------------------------------------------------------------

test('write then read multiple sheets', async () => {
  const wb = new Workbook()
  wb.addWorksheet('First')
  const ws2 = wb.addWorksheet('Second')
  ws2.addRow(['hello'])

  const buf = await wb.xlsx.write()

  const wb2 = new Workbook()
  await wb2.xlsx.read(buf)

  expect(wb2.worksheetCount).toBe(2)
  expect(wb2.getWorksheet('First')).toBeDefined()
  expect(wb2.getWorksheet('Second')).toBeDefined()
  expect(wb2.getWorksheet('Second')!.getCell('A1').value.string).toBe('hello')
})

// ---------------------------------------------------------------------------
// Shared state — WorkbookXlsx handle mutates the same Workbook
// ---------------------------------------------------------------------------

test('wb.xlsx.read mutates the workbook in place', async () => {
  const wb = new Workbook()
  // Initially empty
  expect(wb.worksheetCount).toBe(0)

  // Build a buffer with one sheet
  const wbjs = new ExcelJS.Workbook()
  wbjs.addWorksheet('Shared')
  const buf: Buffer = await wbjs.xlsx.writeBuffer() as Buffer

  // Read via xlsx handle — should mutate the original workbook
  await wb.xlsx.read(buf)
  expect(wb.worksheetCount).toBe(1)
  expect(wb.getWorksheet('Shared')).toBeDefined()
})

test('wb.xlsx getter returns a new handle each time but shares state', async () => {
  const wb = new Workbook()
  const h1 = wb.xlsx
  const h2 = wb.xlsx

  // They are different JS objects
  expect(h1).not.toBe(h2)

  // But they share the same underlying state
  const wbjs = new ExcelJS.Workbook()
  wbjs.addWorksheet('SharedState')
  const buf: Buffer = await wbjs.xlsx.writeBuffer() as Buffer
  await h1.read(buf)

  // h2 should see the same data because they share the inner Arc
  expect(h2).toBeDefined()
  // The workbook itself should also reflect the change
  expect(wb.worksheetCount).toBe(1)
})
