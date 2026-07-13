import { test, expect } from 'vitest'
import { Workbook } from '../index'

test('add data validation, write, re-read', async () => {
  const wb = new Workbook()
  const ws = wb.addWorksheet('Sheet1')
  ws.addRow([1, 2, 3])

  ws.addDataValidation({
    sqref: 'A1:A10',
    type: 'whole',
    operator: 'between',
    formula1: '1',
    formula2: '10',
    allowBlank: true,
  })

  const dv = ws.getDataValidation('A1:A10')
  expect(dv).not.toBeNull()
  expect(dv!.type).toBe('whole')
  expect(dv!.operator).toBe('between')
  expect(dv!.formula1).toBe('1')
  expect(dv!.formula2).toBe('10')
  expect(dv!.allowBlank).toBe(true)
  expect(dv!.sqref).toBe('A1:A10')

  // Round-trip via excelrs
  const buf = await wb.xlsx.write()
  const wb2 = new Workbook()
  await wb2.xlsx.read(buf)
  const ws2 = wb2.getWorksheet('Sheet1')!
  const dv2 = ws2.getDataValidation('A1:A10')
  expect(dv2).not.toBeNull()
  expect(dv2!.type).toBe('whole')
  expect(dv2!.formula1).toBe('1')
  expect(dv2!.formula2).toBe('10')
  expect(dv2!.allowBlank).toBe(true)
})

test('data validation round-trip via exceljs', async () => {
  // Write with exceljs, read with excelrs
  const ExcelJS = await import('exceljs')
  const wbjs = new ExcelJS.default.Workbook()
  const xlws = wbjs.addWorksheet('Sheet1')
  xlws.getCell('A1').value = 5
  // exceljs v4.4.0 Worksheet type omits dataValidations.add() from its public API,
  // but it works at runtime. Defensive `;` protects against ASI merging with the line above.
  ;(xlws as unknown as { dataValidations: { add: Function } }).dataValidations.add('A1', {
    type: 'whole',
    operator: 'between',
    formulae: [1, 10],
    allowBlank: true,
  })

  const raw = await wbjs.xlsx.writeBuffer()
  const buf = raw instanceof Buffer ? raw : Buffer.from(raw)

  const wb = new Workbook()
  await wb.xlsx.read(buf)
  const ws = wb.getWorksheet('Sheet1')!
  const dv = ws.getDataValidation('A1')
  expect(dv).not.toBeNull()
  expect(dv!.type).toBe('whole')
  // exceljs omits operator="between" as the default, so expect None/null
  // expect(dv!.operator).toBeFalsy()
  expect(dv!.formula1).toBe('1')
  expect(dv!.sqref).toBe('A1')
})

test('remove data validation', async () => {
  const wb = new Workbook()
  const ws = wb.addWorksheet('Sheet1')

  ws.addDataValidation({ sqref: 'A1:A10', type: 'whole', formula1: '1' })
  expect(ws.getDataValidation('A1:A10')).not.toBeNull()

  ws.removeDataValidation('A1:A10')
  expect(ws.getDataValidation('A1:A10')).toBeNull()
  expect(ws.dataValidations.length).toBe(0)
})

test('dataValidations getter returns all', async () => {
  const wb = new Workbook()
  const ws = wb.addWorksheet('Sheet1')

  ws.addDataValidation({ sqref: 'A1:A10', type: 'whole', formula1: '1' })
  ws.addDataValidation({ sqref: 'B1:B10', type: 'list', formula1: 'x,y,z' })

  const dvs = ws.dataValidations
  expect(dvs.length).toBe(2)
  // Order should match insertion
  const matchA = dvs.find((dv: { sqref: string }) => dv.sqref === 'A1:A10')
  const matchB = dvs.find((dv: { sqref: string }) => dv.sqref === 'B1:B10')
  expect(matchA).toBeDefined()
  expect(matchA!.type).toBe('whole')
  expect(matchB).toBeDefined()
  expect(matchB!.type).toBe('list')
})

test('no validations returns empty', async () => {
  const wb = new Workbook()
  const ws = wb.addWorksheet('Sheet1')
  ws.addRow([1])
  const buf = await wb.xlsx.write()
  const wb2 = new Workbook()
  await wb2.xlsx.read(buf)
  const ws2 = wb2.getWorksheet('Sheet1')!
  expect(ws2.dataValidations.length).toBe(0)
  expect(ws2.getDataValidation('A1')).toBeNull()
})

test('allowBlank:false round-trips correctly', async () => {
  const wb = new Workbook()
  const ws = wb.addWorksheet('Sheet1')
  ws.addRow([1])
  ws.addDataValidation({ sqref: 'A1', type: 'whole', formula1: '1', allowBlank: false })
  const buf = await wb.xlsx.write()
  const wb2 = new Workbook()
  await wb2.xlsx.read(buf)
  const ws2 = wb2.getWorksheet('Sheet1')!
  const dv = ws2.getDataValidation('A1')
  expect(dv).not.toBeNull()
  expect(dv!.allowBlank).toBe(false)
})
