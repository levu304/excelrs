import { test, expect } from 'vitest'
import { Cell } from '../index'

test('Cell constructor sets address, row, col', () => {
  const cell = new Cell('A1', 1, 1)
  expect(cell.address).toBe('A1')
  expect(cell.row).toBe(1)
  expect(cell.col).toBe(1)
  expect(cell.value).toBeNull()
  expect(cell.type).toBe('Null')
  expect(cell.formula).toBeNull()
})

test('CellValue setter dispatches Number', () => {
  const cell = new Cell('B2', 2, 2)
  cell.value = 42
  expect(cell.type).toBe('Number')
  expect(cell.value).toBe(42)
})

test('CellValue setter dispatches String', () => {
  const cell = new Cell('C3', 3, 3)
  cell.value = 'hello'
  expect(cell.type).toBe('String')
  expect(cell.value).toBe('hello')
})

test('CellValue setter dispatches Boolean', () => {
  const cell = new Cell('D4', 4, 4)
  cell.value = true
  expect(cell.type).toBe('Boolean')
  expect(cell.value).toBe(true)
})

test('CellValue setter handles null', () => {
  const cell = new Cell('E5', 5, 5)
  cell.value = null
  expect(cell.type).toBe('Null')
})

test('CellValue setter throws on undefined (napi-rs constraint)', () => {
  const cell = new Cell('F6', 6, 6)
  // napi-rs does not convert JS `undefined` to serde_json::Value — throws instead.
  // Use `null` explicitly to set a Null value.
  expect(() => { cell.value = undefined as never }).toThrow()
})

test('CellValue setter dispatches Date and round-trips via UTC milliseconds', () => {
  const cell = new Cell('G7', 7, 7)
  const utcDate = new Date(Date.UTC(2026, 0, 15))
  cell.value = utcDate
  expect(cell.type).toBe('Date')
  expect((cell.value as Date).getTime()).toBe(utcDate.getTime())
  expect((cell.value as Date).toISOString()).toBe('2026-01-15T00:00:00.000Z')
})

test('readonly fields are not writable from JS', () => {
  const cell = new Cell('A1', 1, 1)
  // These should be readonly — TS would catch at compile time, but at runtime
  // the assignment is silently ignored or throws in strict mode
  // Cast through unknown to set readonly property at runtime
  expect(() => { (cell as unknown as { address: string }).address = 'B2' }).toThrow()
})

test('serial get/set round-trip preserves value', () => {
  const cell = new Cell('A1', 1, 1)
  cell.value = 42
  expect(cell.type).toBe('Number')
  expect(cell.value).toBe(42)

  cell.value = 'test'
  expect(cell.type).toBe('String')
  expect(cell.value).toBe('test')
})

test('set value with a CellValue object literal dispatches by valueType', () => {
  const cell = new Cell('A1', 0, 0)
  cell.value = { valueType: 'Number', number: 5 } as any
  expect(cell.type).toBe('Number')
  expect(cell.value).toBe(5)
})

test('cell.valueOf returns CellValue discriminated union for RichText cells', () => {
  const cell = new Cell('A1', 0, 0)
  cell.value = {
    richText: [{ text: 'Hello' }],
  }
  expect(cell.valueOf).toBeDefined()
  expect(cell.valueOf.valueType).toBe('RichText')
  expect(cell.richText).toBeDefined()
  expect(cell.richText!.length).toBe(1)
})

test('cell.valueOf vs cell.value — primitive vs CellValue object', () => {
  const cell = new Cell('A1', 0, 0)
  cell.value = 42
  // value unwraps primitives; valueOf returns the full discriminated union
  expect(cell.value).toBe(42)
  expect(cell.valueOf.valueType).toBe('Number')
  const cv = cell.valueOf
  if (cv.valueType === 'Number') {
    expect(cv.number).toBe(42)
  }
})

test('cell.richText returns null for a Number cell', () => {
  const cell = new Cell('A1', 0, 0)
  cell.value = 42
  expect(cell.type).toBe('Number')
  expect(cell.richText).toBeNull()
})

test('cell.valueOf returns Null variant for a freshly constructed cell', () => {
  const cell = new Cell('A1', 0, 0)
  expect(cell.valueOf.valueType).toBe('Null')
})
