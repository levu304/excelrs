import { test, expect } from 'vitest'
import { Workbook } from '../index'

test('E1: add global defined name round-trips', async () => {
  const wb = new Workbook()
  wb.addDefinedName('TaxRate', '0.08')

  // Snapshot
  const names = wb.definedNames
  expect(names).toHaveLength(1)
  expect(names[0].name).toBe('TaxRate')
  expect(names[0].value).toBe('0.08')
  expect(names[0].sheet).toBeUndefined()

  // Write → read back via excelrs
  const ws = wb.addWorksheet('Sheet1')
  ws.addRow([1])
  const buf = await wb.xlsx.write()
  const wb2 = new Workbook()
  await wb2.xlsx.read(buf)
  const names2 = wb2.definedNames
  expect(names2).toHaveLength(1)
  expect(names2[0].name).toBe('TaxRate')
  expect(names2[0].value).toBe('0.08')
  expect(names2[0].sheet).toBeUndefined()
})

test('E2: add sheet-scoped defined name round-trips', async () => {
  const wb = new Workbook()
  wb.addWorksheet('Sheet1')
  wb.addDefinedName('LocalRef', '$A$1:$B$10', 'Sheet1')

  const names = wb.definedNames
  expect(names).toHaveLength(1)
  expect(names[0].sheet).toBe('Sheet1')

  const buf = await wb.xlsx.write()
  const wb2 = new Workbook()
  await wb2.xlsx.read(buf)
  const names2 = wb2.definedNames
  expect(names2).toHaveLength(1)
  expect(names2[0].name).toBe('LocalRef')
  expect(names2[0].sheet).toBe('Sheet1')
})

test('E3: remove defined name round-trips', async () => {
  const wb = new Workbook()
  wb.addDefinedName('X', '1')
  wb.removeDefinedName('X')
  expect(wb.definedNames).toHaveLength(0)

  const buf = await wb.xlsx.write()
  const wb2 = new Workbook()
  await wb2.xlsx.read(buf)
  expect(wb2.definedNames).toHaveLength(0)
})

test('E4: multiple names round-trip in order', async () => {
  const wb = new Workbook()
  wb.addWorksheet('Sheet1')
  wb.addDefinedName('A', '1')
  wb.addDefinedName('B', '2')
  wb.addDefinedName('Local', '$A$1', 'Sheet1')
  wb.addDefinedName('C', '3')

  expect(wb.definedNames).toHaveLength(4)
  expect(wb.definedNames.map(n => n.name)).toEqual(['A', 'B', 'Local', 'C'])

  const buf = await wb.xlsx.write()
  const wb2 = new Workbook()
  await wb2.xlsx.read(buf)
  expect(wb2.definedNames).toHaveLength(4)
  expect(wb2.definedNames.map((n: { name: string }) => n.name)).toEqual(['A', 'B', 'Local', 'C'])
})

test('E5: exceljs-defined names read by excelrs', async () => {
  const ExcelJS = await import('exceljs')

  // exceljs's definedNames.add takes (locStr, name) — note the parameter order!
  const wbjs = new ExcelJS.default.Workbook()
  wbjs.addWorksheet('Sheet1')
  wbjs.definedNames.add('Sheet1!$A$1', 'GlobalRef')

  const raw = await wbjs.xlsx.writeBuffer()
  const buf = raw instanceof Buffer ? raw : Buffer.from(raw as never)

  const wb = new Workbook()
  await wb.xlsx.read(buf)
  const names = wb.definedNames

  // exceljs writes definedNames with fully-qualified refs in the value
  // (e.g. Sheet1!$A$1), while our writer puts localSheetId in an attribute
  // and the cell ref in the value. Both formats should be readable.
  const globalName = names.find((n: { name: string }) => n.name === 'GlobalRef')
  expect(globalName).toBeDefined()
  expect(globalName!.value).toContain('$A$1')
})

test('E6: round-trip via excelrs → exceljs reads back defined names', async () => {
  const wb = new Workbook()
  wb.addWorksheet('Data')
  // Use a cell reference with $ prefix so exceljs can parse it as a range
  wb.addDefinedName('MyRange', '$A$1')

  const buf = await wb.xlsx.write()

  const ExcelJS = await import('exceljs')
  const wbjs = new ExcelJS.default.Workbook()
  await wbjs.xlsx.load(buf as never)

  // exceljs model is an array of { name, ranges } objects
  const model = wbjs.definedNames.model as Array<{ name: string; ranges: string[] }>
  const myRange = model.find((dn: { name: string }) => dn.name === 'MyRange')
  expect(myRange).toBeDefined()
  // Our writer outputs the raw value as text content; exceljs normalizes
  // the cell reference.
  expect(myRange!.ranges[0]).toContain('$A$1')
})

// Edge cases

test('F1: special XML chars in value are preserved', async () => {
  const wb = new Workbook()
  wb.addDefinedName('Special', 'a&b<c>d"e\'f')

  const buf = await wb.xlsx.write()
  const wb2 = new Workbook()
  await wb2.xlsx.read(buf)
  expect(wb2.definedNames[0].value).toBe('a&b<c>d"e\'f')
})

test('F2: empty value', async () => {
  const wb = new Workbook()
  wb.addDefinedName('Empty', '')

  const buf = await wb.xlsx.write()
  const wb2 = new Workbook()
  await wb2.xlsx.read(buf)
  expect(wb2.definedNames[0].value).toBe('')
})

test('F3: upsert updates value', async () => {
  const wb = new Workbook()
  wb.addDefinedName('X', '1')
  wb.addDefinedName('X', '2')
  expect(wb.definedNames).toHaveLength(1)
  expect(wb.definedNames[0].value).toBe('2')
})

test('F4: remove absent is no-op', () => {
  const wb = new Workbook()
  wb.removeDefinedName('NonExistent')
  expect(wb.definedNames).toHaveLength(0)
})

test('F5: getDefinedName returns null for missing', () => {
  const wb = new Workbook()
  expect(wb.getDefinedName('Missing')).toBeNull()
})
