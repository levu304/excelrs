import { test, expect } from 'vitest'

import { Cell, Workbook } from '../index'

function makeWorkbook() {
  const wb = new Workbook()
  wb.addWorksheet('S')
  return wb
}

test('cell.value = { richText: ... } writes and round-trips', async () => {
  const wb = makeWorkbook()
  const ws = wb.getWorksheet('S')!
  const cell = ws.getCell('A1')

  cell.value = {
    richText: [
      { text: 'Hello ', font: { bold: true } },
      { text: 'World' },
    ],
  }

  // write and read back
  const buf = await wb.xlsx.write()
  const wb2 = new Workbook()
  await wb2.xlsx.read(buf)
  const cell2 = wb2.getWorksheet('S')!.getCell('A1')

  expect(cell2.type).toBe('RichText')
  expect(cell2.richText).toBeDefined()
  expect(cell2.richText!.length).toBe(2)
  expect(cell2.richText![0].text).toBe('Hello ')
  expect(cell2.richText![0].font?.bold).toBe(true)
  expect(cell2.richText![1].text).toBe('World')
})

test('cell.value = { richText: ... } with full font (user reproduction)', async () => {
  const wb = makeWorkbook()
  const ws = wb.getWorksheet('S')!
  const cell = ws.getCell('A1')

  cell.value = {
    richText: [
      {
        text: 'B: (11) = (7) + (10)\n',
        font: { name: 'Times New Roman', size: 8 },
      },
      {
        text: 'S: (11) = (7) - (8) - (10)',
        font: { name: 'Times New Roman', size: 8 },
      },
    ],
  }

  const buf = await wb.xlsx.write()
  const wb2 = new Workbook()
  await wb2.xlsx.read(buf)
  const cell2 = wb2.getWorksheet('S')!.getCell('A1')

  expect(cell2.type).toBe('RichText')
  expect(cell2.richText!.length).toBe(2)
  expect(cell2.richText![0].text).toBe('B: (11) = (7) + (10)\n')
  expect(cell2.richText![0].font?.name).toBe('Times New Roman')
  expect(cell2.richText![0].font?.size).toBeCloseTo(8)
  expect(cell2.richText![1].text).toBe('S: (11) = (7) - (8) - (10)')
})

test('cell.value = { richText: ... } with full font including underline round-trips', async () => {
  const wb = makeWorkbook()
  const ws = wb.getWorksheet('S')!
  const cell = ws.getCell('A1')

  cell.value = {
    richText: [
      {
        text: 'Underline me',
        font: { name: 'Arial', size: 12, bold: true, italic: true, underline: true, color: 'FFFF0000' },
      },
    ],
  }

  const buf = await wb.xlsx.write()
  const wb2 = new Workbook()
  await wb2.xlsx.read(buf)
  const cell2 = wb2.getWorksheet('S')!.getCell('A1')

  expect(cell2.richText!.length).toBe(1)
  const font = cell2.richText![0].font!
  expect(font.name).toBe('Arial')
  expect(font.size).toBeCloseTo(12)
  expect(font.bold).toBe(true)
  expect(font.italic).toBe(true)
  expect(font.underline).toBe(true)
  expect(font.color).toBe('FFFF0000')
})

test('cell.value = { hyperlink: ... } writes and round-trips', async () => {
  const wb = makeWorkbook()
  const ws = wb.getWorksheet('S')!
  const cell = ws.getCell('A1')

  cell.value = {
    hyperlink: 'https://example.com',
    hyperlinkText: 'Example',
  }

  const buf = await wb.xlsx.write()
  const wb2 = new Workbook()
  await wb2.xlsx.read(buf)
  const cell2 = wb2.getWorksheet('S')!.getCell('A1')

  expect(cell2.type).toBe('Hyperlink')
})

test('cell.value = { formula: ... } sets value type via setter', () => {
  const wb = makeWorkbook()
  const ws = wb.getWorksheet('S')!
  const cell = ws.getCell('A1')

  cell.value = {
    formula: 'SUM(A1:B1)',
  }

  // Formula is write-only — value_type is Formula after assign, but
  // XLSX round-trip stores the computed result, not the formula string.
  expect(cell.type).toBe('Formula')
  expect(cell.formula).toBe('SUM(A1:B1)')
})

test('cell.value = { formula: ... } persists through XLSX write and read-back', async () => {
  const wb = makeWorkbook()
  const ws = wb.getWorksheet('S')!
  const cell = ws.getCell('A1')

  cell.value = { formula: 'SUM(A1:A2)' }
  expect(cell.formula).toBe('SUM(A1:A2)')

  const buf = await wb.xlsx.write()
  const wb2 = new Workbook()
  await wb2.xlsx.read(buf)
  const cell2 = wb2.getWorksheet('S')!.getCell('A1')

  expect(cell2.formula).toBe('SUM(A1:A2)')
})

test('cell.valueOf discriminated union narrows on valueType', () => {
  const wb = makeWorkbook()
  const ws = wb.getWorksheet('S')!
  const cell = ws.getCell('A1')

  cell.value = {
    richText: [{ text: 'A' }, { text: 'B' }],
  }

  // Narrow via valueType — TS prevents accessing richText on other variants
  const cv = cell.valueOf
  if (cv.valueType === 'RichText') {
    expect(cv.richText.length).toBe(2)
  } else {
    // Should never reach here for a RichText cell
    expect(true).toBe(false)
  }
})

test('unknown valueType raises an error', () => {
  const cell = new Cell('A1', 0, 0)
  expect(() => {
    ;(cell as any).value = { valueType: 'Banana', number: 5 }
  }).toThrow(/Unknown valueType/)
})

test('reassigning a primitive after formula clears the formula', () => {
  const wb = makeWorkbook()
  const ws = wb.getWorksheet('S')!
  const cell = ws.getCell('A1')

  cell.value = { formula: 'SUM(A1:A2)' }
  expect(cell.formula).toBe('SUM(A1:A2)')

  cell.value = 42
  expect(cell.formula).toBeNull()
  expect(cell.type).toBe('Number')
})

test('primitives still work (no regression)', async () => {
  const wb = makeWorkbook()
  const ws = wb.getWorksheet('S')!

  ws.getCell('A1').value = 42
  ws.getCell('A2').value = 'hello'
  ws.getCell('A3').value = true
  ws.getCell('A4').value = null

  const buf = await wb.xlsx.write()
  const wb2 = new Workbook()
  await wb2.xlsx.read(buf)

  expect(wb2.getWorksheet('S')!.getCell('A1').type).toBe('Number')
  expect(wb2.getWorksheet('S')!.getCell('A2').type).toBe('String')
  expect(wb2.getWorksheet('S')!.getCell('A3').type).toBe('Boolean')
  expect(wb2.getWorksheet('S')!.getCell('A4').type).toBe('Null')
})

test('Date primitive still works (no regression)', async () => {
  const wb = makeWorkbook()
  const ws = wb.getWorksheet('S')!
  ws.getCell('A1').value = new Date('2024-01-15')

  const buf = await wb.xlsx.write()
  const wb2 = new Workbook()
  await wb2.xlsx.read(buf)
  const cell = wb2.getWorksheet('S')!.getCell('A1')

  expect(cell.type).toBe('Date')
})

test('C1: exceljs-authored shared-strings rich text read by excelrs', async () => {
  const ExcelJS = await import('exceljs')
  const wbjs = new ExcelJS.default.Workbook()
  const ws = wbjs.addWorksheet('S')
  // ExcelJS writes rich text as shared strings (<si><r><rPr>…)
  ws.getCell('A1').value = {
    richText: [
      { text: 'Hello ', font: { name: 'Arial', size: 12, bold: true, color: { argb: 'FFFF0000' } } },
      { text: 'World', font: { name: 'Times New Roman', size: 10, italic: true } },
    ],
  }

  const raw = await wbjs.xlsx.writeBuffer()
  const buf = raw instanceof Buffer ? raw : Buffer.from(raw as never)

  const wb = new Workbook()
  await wb.xlsx.read(buf)
  const cell = wb.getWorksheet('S')!.getCell('A1')

  expect(cell.type).toBe('RichText')
  expect(cell.richText).toBeDefined()
  expect(cell.richText!.length).toBe(2)
  // Run 0: Arial 12 bold red
  expect(cell.richText![0].text).toBe('Hello ')
  const f0 = cell.richText![0].font!
  expect(f0.name).toBe('Arial')
  expect(f0.size).toBeCloseTo(12)
  expect(f0.bold).toBe(true)
  expect(f0.color).toBe('FFFF0000')
  // Run 1: Times New Roman 10 italic
  expect(cell.richText![1].text).toBe('World')
  const f1 = cell.richText![1].font!
  expect(f1.name).toBe('Times New Roman')
  expect(f1.size).toBeCloseTo(10)
  expect(f1.italic).toBe(true)
})

test('C2: exceljs round-trip then read by excelrs preserves rich text', async () => {
  const ExcelJS = await import('exceljs')

  // Write with exceljs
  const wbjs = new ExcelJS.default.Workbook()
  const wsjs = wbjs.addWorksheet('S')
  wsjs.getCell('A1').value = {
    richText: [{ text: 'Bold', font: { name: 'Arial', size: 14, bold: true } }],
  }
  const raw1 = await wbjs.xlsx.writeBuffer()
  const buf1 = raw1 instanceof Buffer ? raw1 : Buffer.from(raw1 as never)

  // Re-read into exceljs, then re-write (exceljs round-trip)
  const wbjs2 = new ExcelJS.default.Workbook()
  await wbjs2.xlsx.load(buf1 as never)
  const raw2 = await wbjs2.xlsx.writeBuffer()
  const buf2 = raw2 instanceof Buffer ? raw2 : Buffer.from(raw2 as never)

  // Read with excelrs
  const wb = new Workbook()
  await wb.xlsx.read(buf2)
  const cell = wb.getWorksheet('S')!.getCell('A1')

  expect(cell.type).toBe('RichText')
  expect(cell.richText!.length).toBe(1)
  expect(cell.richText![0].text).toBe('Bold')
  expect(cell.richText![0].font!.name).toBe('Arial')
  expect(cell.richText![0].font!.size).toBeCloseTo(14)
  expect(cell.richText![0].font!.bold).toBe(true)
})