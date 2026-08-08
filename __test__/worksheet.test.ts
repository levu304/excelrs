import { test, expect } from 'vitest'
import { Workbook, Worksheet } from '../index'

test('Worksheet constructor', () => {
  const ws = new Worksheet('Sheet1')
  expect(ws.name).toBe('Sheet1')
  expect(ws.id).toBe(1)
  expect(ws.rowCount).toBe(0)
  expect(ws.columnCount).toBe(0)
})

test('getCell by address (via JS glue)', () => {
  const ws = new Worksheet('Sheet1')
  const cell = ws.getCell('A1')
  expect(cell.address).toBe('A1')
  expect(cell.row).toBe(1)
  expect(cell.col).toBe(1)
  expect(cell.value).toBeNull()
  expect(cell.type).toBe('Null')
})

test('getCell by row/col (via JS glue)', () => {
  const ws = new Worksheet('Sheet1')
  const cell = ws.getCell(3, 5)
  expect(cell.address).toBe('E3')
  expect(cell.row).toBe(3)
  expect(cell.col).toBe(5)
})

test('getCellByAddress directly (Rust method)', () => {
  const ws = new Worksheet('Sheet1')
  const cell = ws.getCellByAddress('D4')
  expect(cell.address).toBe('D4')
  expect(cell.row).toBe(4)
  expect(cell.col).toBe(4)
})

test('getCellByRc directly (Rust method)', () => {
  const ws = new Worksheet('Sheet1')
  const cell = ws.getCellByRc(10, 27)
  expect(cell.address).toBe('AA10')
})

test('addRow creates row with cell values', () => {
  const ws = new Worksheet('Data')
  const row = ws.addRow(['Alice', 30, true])
  expect(row.number).toBe(1)
  expect(ws.rowCount).toBe(1)

  // Verify via getCell
  const c1 = ws.getCell('A1')
  expect(c1.value).toBe('Alice')
  expect(c1.type).toBe('String')

  const c2 = ws.getCell('B1')
  expect(c2.value).toBe(30)
  expect(c2.type).toBe('Number')

  const c3 = ws.getCell('C1')
  expect(c3.value).toBe(true)
  expect(c3.type).toBe('Boolean')
})

test('multiple addRow calls', () => {
  const ws = new Worksheet('Data')
  ws.addRow(['a', 1])
  ws.addRow(['b', 2])
  ws.addRow(['c', 3])
  expect(ws.rowCount).toBe(3)

  expect(ws.getCell('A2').value).toBe('b')
  expect(ws.getCell('B3').value).toBe(3)
})

test('getRow creates row if not exists', () => {
  const ws = new Worksheet('Sheet1')
  const row = ws.getRow(42)
  expect(row.number).toBe(42)
})

test('removeRow removes row', () => {
  const ws = new Worksheet('Sheet1')
  ws.addRow(['a'])
  ws.addRow(['b'])
  expect(ws.rowCount).toBe(2)

  ws.removeRow(1)

  // Row 1 is gone but rowCount reflects max row (still 2)
  // getCell on row 1 returns empty cell
  const cell = ws.getCell('A1')
  expect(cell.type).toBe('Null')
  expect(cell.value).toBeNull()
})

test('getRows returns range', () => {
  const ws = new Worksheet('Sheet1')
  ws.addRow(['a', 1])
  ws.addRow(['b', 2])
  ws.addRow(['c', 3])

  const rows = ws.getRows(2, 2)
  expect(rows.length).toBe(2)
  expect(rows[0].number).toBe(2)
  expect(rows[1].number).toBe(3)
})

test('rows getter returns all rows sorted', () => {
  const ws = new Worksheet('Sheet1')
  ws.addRow(['first'])
  ws.addRow(['second'])
  ws.addRow(['third'])

  const all = ws.rows
  expect(all.length).toBe(3)
  expect(all[0].number).toBe(1)
  expect(all[2].number).toBe(3)
})

test('setName on worksheet', () => {
  const ws = new Worksheet('Old')
  ws.name = 'New'
  expect(ws.name).toBe('New')
})

test('getRow().getCell().value on fresh row — string', () => {
  const ws = new Worksheet('Test')

  const row = ws.getRow(1)

  const cell = row.getCell('A')

  cell.value = 'Hello!'

  expect(ws.getCell('A1').value).toBe('Hello!')

  expect(ws.getCell('A1').type).toBe('String')

})


  test('getRow().getCell().value on fresh row — number', () => {
  const ws = new Worksheet('Test')

  ws.getRow(2).getCell('B').value = 42

  expect(ws.getCell('B2').value).toBe(42)

  expect(ws.getCell('B2').type).toBe('Number')

})


  test('getRow().getCell().value on fresh row — boolean', () => {
  const ws = new Worksheet('Test')

  ws.getRow(3).getCell('C').value = true

  expect(ws.getCell('C3').value).toBe(true)

  expect(ws.getCell('C3').type).toBe('Boolean')

})


  test('getRow().getCell().style mutation on fresh row', () => {
  const ws = new Worksheet('Test')

  ws.getRow(4).getCell('A').style = { font: { bold: true } }

  expect(ws.getCell('A4').style!.font!.bold).toBe(true)

})


  test('getRow().getCell().value on pre-existing cell (v0.4.0 reg guard)', () => {
  const ws = new Worksheet('Test')

  ws.addRow([10])

  ws.getRow(1).getCell(1).value = 99

  expect(ws.getCell('A1').value).toBe(99)

})


test('getRow().getCell().value on sparse high row number persists', () => {
    const ws = new Worksheet('Sparse')

    ws.getRow(100).getCell('A').value = 'sparse'

    expect(ws.getCell('A100').value).toBe('sparse')

})

// ---------------------------------------------------------------------------
// Worksheet-level metadata (v2.1.0): state, tabColor, default dimensions
// ---------------------------------------------------------------------------

test('addWorksheet with state and properties applies them', () => {
  const wb = new Workbook()
  const ws = wb.addWorksheet('Secret', {
    state: 'hidden',
    properties: { tabColor: 'FFFF0000', defaultRowHeight: 15, defaultColWidth: 10, outlineLevelRow: 1 },
  })
  expect(ws.state).toBe('hidden')
  expect(ws.properties.tabColor).toBe('FFFF0000')
  expect(ws.properties.defaultRowHeight).toBe(15)
  expect(ws.properties.defaultColWidth).toBe(10)
  expect(ws.properties.outlineLevelRow).toBe(1)
})

test('addWorksheet defaults to visible with no tab color', () => {
  const wb = new Workbook()
  const ws = wb.addWorksheet('Sheet1')
  expect(ws.state).toBe('visible')
  expect(ws.tabColor).toBeNull()
  expect(ws.properties.tabColor).toBeUndefined()
})

test('state setter round-trips hidden', async () => {
  const wb = new Workbook()
  const ws = wb.addWorksheet('Hidden')
  ws.state = 'hidden'
  const buf = await wb.xlsx.write()
  const wb2 = new Workbook()
  await wb2.xlsx.read(buf as never)
  expect(wb2.getWorksheet('Hidden')!.state).toBe('hidden')
})

test('state setter round-trips veryHidden', async () => {
  const wb = new Workbook()
  const ws = wb.addWorksheet('Very')
  ws.state = 'veryHidden'
  const buf = await wb.xlsx.write()
  const wb2 = new Workbook()
  await wb2.xlsx.read(buf as never)
  expect(wb2.getWorksheet('Very')!.state).toBe('veryHidden')
})

test('tabColor round-trips', async () => {
  const wb = new Workbook()
  const ws = wb.addWorksheet('Colored')
  ws.tabColor = 'FFFF0000'
  const buf = await wb.xlsx.write()
  const wb2 = new Workbook()
  await wb2.xlsx.read(buf as never)
  expect(wb2.getWorksheet('Colored')!.tabColor).toBe('FFFF0000')
})

test('default dimensions round-trip', async () => {
  const wb = new Workbook()
  const ws = wb.addWorksheet('Dims')
  ws.setProperties({ defaultRowHeight: 24, defaultColWidth: 20, outlineLevelRow: 2, outlineLevelCol: 1 })
  const buf = await wb.xlsx.write()
  const wb2 = new Workbook()
  await wb2.xlsx.read(buf as never)
  const props = wb2.getWorksheet('Dims')!.properties
  expect(props.defaultRowHeight).toBe(24)
  expect(props.defaultColWidth).toBe(20)
  expect(props.outlineLevelRow).toBe(2)
  expect(props.outlineLevelCol).toBe(1)
})

test('cross-library parity: ExcelJS hidden sheet read by excelrs stays hidden', async () => {
  const wbjs = new (require('exceljs').Workbook)()
  const wsjs = wbjs.addWorksheet('Secret')
  wsjs.state = 'hidden'
  const buf = await wbjs.xlsx.writeBuffer()

  const wb = new Workbook()
  await wb.xlsx.read(buf as never)
  expect(wb.getWorksheet('Secret')!.state).toBe('hidden')
})

