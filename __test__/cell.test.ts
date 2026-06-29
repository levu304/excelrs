import { test, expect } from 'vitest'
import { Cell } from '../index'

test('Cell constructor sets address, row, col', () => {
  const cell = new Cell('A1', 1, 1)
  expect(cell.address).toBe('A1')
  expect(cell.row).toBe(1)
  expect(cell.col).toBe(1)
  expect(cell.value.valueType).toBe('Null')
  expect(cell.formula).toBeNull()
})

test('CellValue setter dispatches Number', () => {
  const cell = new Cell('B2', 2, 2)
  cell.value = 42
  expect(cell.value.valueType).toBe('Number')
  expect(cell.value.number).toBe(42)
})

test('CellValue setter dispatches String', () => {
  const cell = new Cell('C3', 3, 3)
  cell.value = 'hello'
  expect(cell.value.valueType).toBe('String')
  expect(cell.value.string).toBe('hello')
})

test('CellValue setter dispatches Boolean', () => {
  const cell = new Cell('D4', 4, 4)
  cell.value = true
  expect(cell.value.valueType).toBe('Boolean')
  expect(cell.value.boolean).toBe(true)
})

test('CellValue setter handles null', () => {
  const cell = new Cell('E5', 5, 5)
  cell.value = null
  expect(cell.value.valueType).toBe('Null')
})

test('CellValue setter throws on undefined (napi-rs constraint)', () => {
  const cell = new Cell('F6', 6, 6)
  // napi-rs does not convert JS `undefined` to serde_json::Value — throws instead.
  // Use `null` explicitly to set a Null value.
  expect(() => { cell.value = undefined }).toThrow()
})

test('readonly fields are not writable from JS', () => {
  const cell = new Cell('A1', 1, 1)
  // These should be readonly — TS would catch at compile time, but at runtime
  // the assignment is silently ignored or throws in strict mode
  expect(() => { (cell as any).address = 'B2' }).toThrow()
})

test('serial get/set round-trip preserves value', () => {
  const cell = new Cell('A1', 1, 1)
  cell.value = 42
  const v = cell.value
  expect(v.valueType).toBe('Number')
  expect(v.number).toBe(42)

  cell.value = 'test'
  const v2 = cell.value
  expect(v2.valueType).toBe('String')
  expect(v2.string).toBe('test')
})
