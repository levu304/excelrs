import { test, expect } from 'vitest'
import { tmpdir } from 'node:os'
import path from 'node:path'
import { Workbook } from '../index'

test('csv write produces correct output', async () => {
  const wb = new Workbook()
  const ws = wb.addWorksheet('Sheet1')
  ws.addRow([42, 'hello'])
  ws.addRow([3.14, 'world'])

  const buf = await wb.csv.write()
  const text = buf.toString()

  expect(text).toContain('42,hello')
  expect(text).toContain('3.14,world')
})

test('csv round-trip preserves values', async () => {
  const wb = new Workbook()
  const ws = wb.addWorksheet('Sheet1')
  ws.addRow([1, 'a', true])
  ws.addRow([2, 'b', false])

  const buf = await wb.csv.write()

  const wb2 = new Workbook()
  await wb2.csv.read(buf)
  const ws2 = wb2.getWorksheet('Sheet1')!
  expect(ws2.rowCount).toBe(2)

  // Row 1
  const a1Cell = ws2.getCellByAddress('A1')
  expect(a1Cell.type).toBe('Number')
  expect(a1Cell.value).toBe(1)

  const b1Cell = ws2.getCellByAddress('B1')
  expect(b1Cell.type).toBe('String')
  expect(b1Cell.value).toBe('a')

  // Row 2
  const a2Cell = ws2.getCellByAddress('A2')
  expect(a2Cell.type).toBe('Number')
  expect(a2Cell.value).toBe(2)

  const b2Cell = ws2.getCellByAddress('B2')
  expect(b2Cell.type).toBe('String')
  expect(b2Cell.value).toBe('b')
})

test('csv numeric inference on read', async () => {
  const wb = new Workbook()
  await wb.csv.read(Buffer.from('42,hello\n3.14,world'))

  const ws = wb.getWorksheet('Sheet1')!
  const a1Cell = ws.getCellByAddress('A1')
  expect(a1Cell.type).toBe('Number')
  expect(a1Cell.value).toBe(42)

  const b1Cell = ws.getCellByAddress('B1')
  expect(b1Cell.type).toBe('String')

  const a2Cell = ws.getCellByAddress('A2')
  expect(a2Cell.type).toBe('Number')
  expect(a2Cell.value).toBe(3.14)
})

test('csv custom delimiter', async () => {
  const wb = new Workbook()
  await wb.csv.read(Buffer.from('a;b\n1;2'), ';')

  const ws = wb.getWorksheet('Sheet1')!
  const a1Cell = ws.getCellByAddress('A1')
  expect(a1Cell.value).toBe('a')

  const b2Cell = ws.getCellByAddress('B2')
  expect(b2Cell.value).toBe(2)

  // write with same delimiter
  const out = await wb.csv.write(';', false)
  expect(out.toString()).toBe('a;b\n1;2\n')
})

test('csv write with BOM', async () => {
  const wb = new Workbook()
  wb.addWorksheet('Sheet1').addRow([99])

  const buf = await wb.csv.write(',', true)
  // Check BOM bytes
  expect(buf[0]).toBe(0xEF)
  expect(buf[1]).toBe(0xBB)
  expect(buf[2]).toBe(0xBF)
  // BOM-roundtrip: BOM-stripped on read
  const wb2 = new Workbook()
  await wb2.csv.read(buf)
  const ws = wb2.getWorksheet('Sheet1')!
  expect(ws.getCellByAddress('A1').value).toBe(99)
})

test('csv write empty workbook produces empty buffer', async () => {
  const wb = new Workbook()
  const buf = await wb.csv.write()
  expect(buf.length).toBe(0)
})

test('csv write with no worksheets produces empty buffer', async () => {
  const wb = new Workbook()
  wb.addWorksheet('Sheet1') // no rows
  const buf = await wb.csv.write()
  expect(buf.length).toBe(0)
})

test('csv quoted fields on write', async () => {
  const wb = new Workbook()
  const ws = wb.addWorksheet('Sheet1')
  ws.addRow(['hello,world', 'line1\nline2'])

  const buf = await wb.csv.write()
  const text = buf.toString()

  expect(text).toContain('"hello,world"')
  expect(text).toContain('"line1\nline2"')
})

test('csv read file and write file round-trip', async () => {
  const wb = new Workbook()
  wb.addWorksheet('Sheet1').addRow([1, 2, 3])

  const tmpFile = path.join(tmpdir(), `excelrs_test_csv_${Date.now()}.csv`)
  await wb.csv.writeFile(tmpFile)

  const wb2 = new Workbook()
  await wb2.csv.readFile(tmpFile)

  const ws = wb2.getWorksheet('Sheet1')!
  expect(ws.rowCount).toBe(1)
  expect(ws.getCellByAddress('A1').value).toBe(1)

  // clean up
  const fs = await import('fs')
  fs.unlinkSync(tmpFile)
})
