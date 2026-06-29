import { test, expect } from 'vitest'
import { Workbook } from '../index'

test('Workbook constructor creates empty workbook', () => {
  const wb = new Workbook()
  expect(wb.worksheetCount).toBe(0)
  expect(wb.worksheets).toHaveLength(0)
})

test('addWorksheet adds and returns worksheet', () => {
  const wb = new Workbook()
  const ws = wb.addWorksheet('Sheet1')
  expect(ws.name).toBe('Sheet1')
  expect(ws.id).toBe(1)
  expect(wb.worksheetCount).toBe(1)
})

test('addWorksheet increments id', () => {
  const wb = new Workbook()
  wb.addWorksheet('First')
  wb.addWorksheet('Second')
  wb.addWorksheet('Third')

  expect(wb.worksheetCount).toBe(3)
  expect(wb.worksheets[0].name).toBe('First')
  expect(wb.worksheets[1].name).toBe('Second')
  expect(wb.worksheets[2].name).toBe('Third')
})

test('getWorksheet by name', () => {
  const wb = new Workbook()
  wb.addWorksheet('Sheet1')
  wb.addWorksheet('Data')

  const ws = wb.getWorksheet('Data')
  expect(ws).not.toBeNull()
  expect(ws!.name).toBe('Data')

  const missing = wb.getWorksheet('NonExistent')
  expect(missing).toBeNull()
})

test('getWorksheet by index (1-based)', () => {
  const wb = new Workbook()
  wb.addWorksheet('First')
  wb.addWorksheet('Second')

  const ws = wb.getWorksheet(2)
  expect(ws).not.toBeNull()
  expect(ws!.name).toBe('Second')

  const outOfRange = wb.getWorksheet(99)
  expect(outOfRange).toBeNull()
})

test('workbook round-trip: create workbook, add sheet, read worksheets', () => {
  const wb = new Workbook()
  wb.addWorksheet('Data')
  wb.addWorksheet('Summary')

  const sheets = wb.worksheets
  expect(sheets).toHaveLength(2)
  expect(sheets[0].name).toBe('Data')
  expect(sheets[1].name).toBe('Summary')
  expect(wb.worksheetCount).toBe(2)
})
