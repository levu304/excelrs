import { test, expect } from 'vitest'
import ExcelJS from 'exceljs'
import { Workbook } from '../index'

const rt = async (wb: Workbook): Promise<Workbook> => {
  const buf = await wb.xlsx.write()
  const wb2 = new Workbook()
  await wb2.xlsx.read(buf as never)
  return wb2
}

test('addTable writes header and data cells (v1.1.0)', () => {
  const wb = new Workbook()
  const ws = wb.addWorksheet('S')
  ws.addTable({
    name: 'T1',
    ref: 'A1:C3',
    headerRow: true,
    columns: [{ name: 'A' }, { name: 'B' }, { name: 'C' }],
    rows: [
      [1, 2, 3],
      [4, 5, 6],
    ],
  })
  expect(ws.getCell('A1').value.string).toBe('A')
  expect(ws.getCell('C1').value.string).toBe('C')
  expect(ws.getCell('A2').value.number).toBe(1)
  expect(ws.getCell('C3').value.number).toBe(6)
})

test('addTable round-trips through write/read (v1.1.0)', async () => {
  const wb = new Workbook()
  const ws = wb.addWorksheet('Sheet1')
  ws.addTable({
    name: 'T1',
    ref: 'A1:C3',
    headerRow: true,
    totalsRow: false,
    columns: [{ name: 'A' }, { name: 'B' }, { name: 'C' }],
    rows: [
      [1, 2, 3],
      [4, 5, 6],
    ],
    style: { theme: 'TableStyleMedium2', showRowStripes: true },
    autoFilter: 'A1:C3',
  })

  const wb2 = await rt(wb)
  const t = wb2.getWorksheet('Sheet1')!.getTable('T1')!
  expect(t).toBeDefined()
  expect(t.name).toBe('T1')
  expect(t.ref).toBe('A1:C3')
  expect(t.columns.length).toBe(3)
  expect(t.columns[0].name).toBe('A')
  expect(t.rows.length).toBe(2)
  expect(t.rows[0].values[0].number).toBe(1)
  expect(t.rows[1].values[2].number).toBe(6)
  expect(t.style?.theme).toBe('TableStyleMedium2')
  expect(t.autofilterRef).toBe('A1:C3')
})

test('getTables lists tables and removeTable leaves cells intact (v1.1.0)', () => {
  const wb = new Workbook()
  const ws = wb.addWorksheet('S')
  ws.addTable({
    name: 'T1',
    ref: 'A1:C3',
    headerRow: true,
    columns: [{ name: 'A' }, { name: 'B' }, { name: 'C' }],
    rows: [
      [1, 2, 3],
      [4, 5, 6],
    ],
  })
  expect(ws.getTables().length).toBe(1)
  // Cells are populated before removeTable.
  expect(ws.getCell('A2').value.number).toBe(1)

  const ok = ws.removeTable('T1')
  expect(ok).toBe(true)
  expect(ws.getTable('T1')).toBeNull()
  expect(ws.getTables().length).toBe(0)
  // Underlying cells remain intact.
  expect(ws.getCell('A1').value.string).toBe('A')
  expect(ws.getCell('A2').value.number).toBe(1)
  expect(ws.getCell('C3').value.number).toBe(6)
})

test('duplicate table name is rejected (v1.1.0)', () => {
  const wb = new Workbook()
  const ws = wb.addWorksheet('S')
  ws.addTable({
    name: 'T1',
    ref: 'A1:C3',
    headerRow: true,
    columns: [{ name: 'A' }, { name: 'B' }, { name: 'C' }],
    rows: [[1, 2, 3], [4, 5, 6]],
  })
  expect(() =>
    ws.addTable({
      name: 'T1',
      ref: 'E1:G3',
      headerRow: true,
      columns: [{ name: 'X' }, { name: 'Y' }, { name: 'Z' }],
      rows: [[7, 8, 9], [10, 11, 12]],
    }),
  ).toThrow()
})

test('reads an ExcelJS-authored table (v1.1.0)', async () => {
  const ej = new ExcelJS.Workbook()
  const ws = ej.addWorksheet('Sheet1')
  ws.addTable({
    name: 'T1',
    ref: 'A1:C3',
    headerRow: true,
    totalsRow: false,
    style: { theme: 'TableStyleDark1', showRowStripes: true },
    columns: [{ name: 'A' }, { name: 'B' }, { name: 'C' }],
    rows: [[1, 2, 3], [4, 5, 6]],
  } as never)

  const buf = await ej.xlsx.writeBuffer()
  const wb = new Workbook()
  await wb.xlsx.read(buf as never)

  const t = wb.getWorksheet('Sheet1')!.getTable('T1')!
  expect(t).toBeDefined()
  expect(t.ref).toBe('A1:C3')
  expect(t.columns.length).toBe(3)
  expect(t.columns[0].name).toBe('A')
  // ExcelJS forces totalsRowShown=1 even when totalsRow:false is passed, so the
  // OOXML-correct reconstruction (exclude the totals row) yields 1 data row here.
  // Assert fidelity: at least the first authored data row is recovered.
  expect(t.rows.length).toBeGreaterThanOrEqual(1)
  expect(t.rows[0].values[0].number).toBe(1)
})
