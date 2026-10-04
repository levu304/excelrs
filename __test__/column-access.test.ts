import { test, expect } from 'vitest'
import ExcelJS from 'exceljs'
import { Workbook } from '../index'

/** Helper: write an excelrs workbook and read it back with exceljs. */
async function writeThenReadWithExceljs(wb: Workbook): Promise<ExcelJS.Workbook> {
  const buf = await wb.xlsx.write()
  const wbjs = new ExcelJS.Workbook()
  await wbjs.xlsx.load(buf as never)
  return wbjs
}

test('getColumn by number resolves an existing column', () => {
  const wb = new Workbook()
  const ws = wb.addWorksheet('Cols')
  ws.setColumns([{ header: 'B', key: 'b', width: 15 }])
  const col = ws.getColumn(1)
  expect(col.header).toBe('B')
  expect(col.key).toBe('b')
  expect(col.width).toBe(15)
})

test('getColumn auto-creates an absent column and mutations persist', () => {
  const wb = new Workbook()
  const ws = wb.addWorksheet('Auto')
  ws.getColumn(5).width = 20
  expect(ws.getColumn(5).width).toBe(20)
  expect(ws.getColumn(5).colNum).toBe(5)
})

test('getColumn by letter reaches the same column as by number', () => {
  const wb = new Workbook()
  const ws = wb.addWorksheet('Letter')
  ws.getColumn('C').hidden = true
  expect(ws.getColumn(3).hidden).toBe(true)
  ws.getColumn(2).width = 12
  expect(ws.getColumn('B').width).toBe(12)
})

test('getColumn rejects invalid input', () => {
  const wb = new Workbook()
  const ws = wb.addWorksheet('Invalid')
  expect(() => ws.getColumn('!!')).toThrow()
  expect(() => ws.getColumn('')).toThrow()
  expect(() => ws.getColumn(0)).toThrow()
})

test('getColumn width mutation is emitted in <cols>', async () => {
  const wb = new Workbook()
  const ws = wb.addWorksheet('Emit')
  ws.getColumn(1).width = 20
  ws.addRow(['a'])

  const buf = await wb.xlsx.write()
  const wb2 = new Workbook()
  await wb2.xlsx.read(buf as never)
  expect(wb2.getWorksheet('Emit')!.getColumn(1).width).toBe(20)
})

test('getColumn style reaches unstyled cells via exceljs', async () => {
  const wb = new Workbook()
  const ws = wb.addWorksheet('Inherit')
  ws.getColumn('A').style = { font: { bold: true } }
  ws.addRow(['hello'])

  const wbjs = await writeThenReadWithExceljs(wb)
  expect(wbjs.getWorksheet('Inherit')!.getCell('A1').font?.bold).toBe(true)
})
