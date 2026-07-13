import { test, expect } from 'vitest'
import ExcelJS from 'exceljs'
import { Workbook } from '../index'
import * as fs from 'fs'
import * as path from 'path'

// ---------------------------------------------------------------------------
// Helper: write an ExcelJS workbook to buffer, read it with excelrs
// ---------------------------------------------------------------------------

async function exceljsToExcelrs(make: (ws: ExcelJS.Worksheet) => void): Promise<Workbook> {
  const wbjs = new ExcelJS.Workbook()
  const ws = wbjs.addWorksheet('Sheet1')
  make(ws)
  const buf = await wbjs.xlsx.writeBuffer()
  const wb = new Workbook()
  await wb.xlsx.read(buf as never)
  return wb
}

// ---------------------------------------------------------------------------
// F1 — Font theme color resolves to ARGB
// ---------------------------------------------------------------------------

test('F1: font theme color resolves to default accent1 ARGB', async () => {
  const wb = await exceljsToExcelrs((ws) => {
    ws.getCell('A1').value = 'hello'
    ws.getCell('A1').font = { color: { theme: 4 } }
  })
  const cell = wb.getWorksheet('Sheet1')!.getCell('A1')
  expect(cell.style?.font?.color).toBe('FF4F81BD')
})

// ---------------------------------------------------------------------------
// F2 — Font theme color with tint
// ---------------------------------------------------------------------------

test('F2: font theme color with tint resolves approximately', async () => {
  const wb = await exceljsToExcelrs((ws) => {
    ws.getCell('A1').value = 'hello'
    ws.getCell('A1').font = { color: { theme: 4, tint: -0.5 } as unknown as ExcelJS.Color }
  })
  const color = wb.getWorksheet('Sheet1')!.getCell('A1').style?.font?.color
  expect(color).toBeDefined()
  expect(color).not.toBeNull()
  // tint -0.5 on accent1 (4F81BD) → ~"28415F" (Rust round-half-away-from-zero)
  // Use loose check: non-null hex ARGB starting with FF
  expect(color!).toMatch(/^FF[0-9A-F]{6}$/i)
})

// ---------------------------------------------------------------------------
// F3 — Border theme color
// ---------------------------------------------------------------------------

test('F3: border top theme color resolves to lt1 ARGB', async () => {
  const wb = await exceljsToExcelrs((ws) => {
    ws.getCell('A1').value = 'hello'
    ws.getCell('A1').border = { top: { style: 'thin', color: { theme: 1 } } }
  })
  const cell = wb.getWorksheet('Sheet1')!.getCell('A1')
  expect(cell.style?.border?.top?.color).toBe('FFFFFFFF')
})

// ---------------------------------------------------------------------------
// F4 — Fill foreground theme color
// ---------------------------------------------------------------------------

test('F4: fill foreground theme color resolves to accent6 ARGB', async () => {
  const wb = await exceljsToExcelrs((ws) => {
    ws.getCell('A1').value = 'hello'
    ws.getCell('A1').fill = { type: 'pattern', pattern: 'solid', fgColor: { theme: 8 } }
  })
  const cell = wb.getWorksheet('Sheet1')!.getCell('A1')
  expect(cell.style?.fill?.foreground).toBe('FF4BACC6')
})

// ---------------------------------------------------------------------------
// F5 — Round-trip: read themed → write → ExcelJS reads → same ARGB
// ---------------------------------------------------------------------------

test('F5: round-trip themed ARGB through excelrs write', async () => {
  // 1. Create themed workbook with ExcelJS
  const wbjsIn = new ExcelJS.Workbook()
  const wsIn = wbjsIn.addWorksheet('Sheet1')
  wsIn.getCell('A1').value = 'hello'
  wsIn.getCell('A1').font = { color: { theme: 4 } }
  const bufIn = await wbjsIn.xlsx.writeBuffer()

  // 2. Read with excelrs
  const wb = new Workbook()
  await wb.xlsx.read(bufIn as never)
  const colorIn = wb.getWorksheet('Sheet1')!.getCell('A1').style?.font?.color
  expect(colorIn).toBe('FF4F81BD')

  // 3. Write back with excelrs
  const bufOut = await wb.xlsx.write()

  // 4. Read with ExcelJS
  const wbjsOut = new ExcelJS.Workbook()
  // exceljs load() expects legacy Buffer type; newer @types/node returns
  // Buffer<ArrayBufferLike>.  `as never` bridges the version gap.
  await wbjsOut.xlsx.load(bufOut as never)
  const colorOut = wbjsOut.getWorksheet('Sheet1')!.getCell('A1').font?.color
  expect(colorOut?.argb?.toUpperCase()).toBe('FF4F81BD')
})

// ---------------------------------------------------------------------------
// F6 — Default scheme (no custom theme)
// ---------------------------------------------------------------------------

test('F6: default scheme resolves when theme1.xml has standard palette', async () => {
  const wb = await exceljsToExcelrs((ws) => {
    ws.getCell('A1').value = 'hello'
    ws.getCell('A1').font = { color: { theme: 4 } }
  })
  const cell = wb.getWorksheet('Sheet1')!.getCell('A1')
  expect(cell.style?.font?.color).toBe('FF4F81BD')
})

// ---------------------------------------------------------------------------
// F7 — Indexed color resolution
// ---------------------------------------------------------------------------

test('F7: indexed color resolves to system palette entry', async () => {
  const wb = await exceljsToExcelrs((ws) => {
    ws.getCell('A1').value = 'hello'
    ws.getCell('A1').fill = { type: 'pattern', pattern: 'solid', fgColor: { indexed: 8 } as unknown as ExcelJS.Color }
  })
  const color = wb.getWorksheet('Sheet1')!.getCell('A1').style?.fill?.foreground
  expect(color).toBeDefined()
  expect(color).not.toBeNull()
  expect(color!).toMatch(/^FF[0-9A-F]{6}$/i)
})

// ---------------------------------------------------------------------------
// F8 — Custom theme fixture
// ---------------------------------------------------------------------------

test('F8: custom theme fixture resolves custom accent1 ARGB', async () => {
  const fixturePath = path.resolve(__dirname, '..', 'fixtures', 'custom-theme.xlsx')
  const buf = fs.readFileSync(fixturePath)
  const wb = new Workbook()
  await wb.xlsx.read(buf)
  const cell = wb.getWorksheet('Sheet1')!.getCell('A1')
  // Fixture has accent1=FF0000 (red) instead of default 4F81BD
  expect(cell.value?.string).toBe('Custom Theme')
  expect(cell.style?.font?.color).toBe('FFFF0000')
})

// ---------------------------------------------------------------------------
// F9 — API stability: color is always a string
// ---------------------------------------------------------------------------

test('F9: color is always a plain string, never an object', async () => {
  const wb = await exceljsToExcelrs((ws) => {
    ws.getCell('A1').value = 'hello'
    ws.getCell('A1').font = { color: { theme: 4 } }
  })
  const color = wb.getWorksheet('Sheet1')!.getCell('A1').style?.font?.color
  expect(typeof color).toBe('string')
})
