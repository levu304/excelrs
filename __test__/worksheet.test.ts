import { test, expect } from 'vitest'
import { Worksheet, Workbook } from '../index'

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
  expect(cell.value.valueType).toBe('Null')
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
  expect(c1.value.string).toBe('Alice')
  expect(c1.value.valueType).toBe('String')

  const c2 = ws.getCell('B1')
  expect(c2.value.number).toBe(30)
  expect(c2.value.valueType).toBe('Number')

  const c3 = ws.getCell('C1')
  expect(c3.value.boolean).toBe(true)
  expect(c3.value.valueType).toBe('Boolean')
})

test('multiple addRow calls', () => {
  const ws = new Worksheet('Data')
  ws.addRow(['a', 1])
  ws.addRow(['b', 2])
  ws.addRow(['c', 3])
  expect(ws.rowCount).toBe(3)

  expect(ws.getCell('A2').value.string).toBe('b')
  expect(ws.getCell('B3').value.number).toBe(3)
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
  expect(cell.value.valueType).toBe('Null')
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
