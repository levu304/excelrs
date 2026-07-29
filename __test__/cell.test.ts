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
