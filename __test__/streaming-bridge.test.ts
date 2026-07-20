import { test, expect } from 'vitest'
import ExcelJS from 'exceljs'
import { StreamReader, StreamWriter, JsStreamSheet } from '../index'
import { PassThrough } from 'node:stream'
import { read, write, readAsReadable, writeToWritable } from '../src/stream-bridge'

// ---------------------------------------------------------------------------
// 5.1 Round-trip: write → read via AsyncIterable, assert cell values match
// ---------------------------------------------------------------------------

test('5.1 round-trip preserves cell values through StreamReader/StreamWriter', async () => {
  // Create workbook with ExcelJS
  const wbjs = new ExcelJS.Workbook()
  const ws1 = wbjs.addWorksheet('Data')
  ws1.getCell('A1').value = 'hello'
  ws1.getCell('B1').value = 42
  ws1.getCell('A2').value = true
  ws1.getCell('B2').value = 3.14
  const ws2 = wbjs.addWorksheet('Empty')
  ws2.getCell('A1').value = null
  const buf = await wbjs.xlsx.writeBuffer()

  // Read via StreamReader
  const reader = new StreamReader(Buffer.from(buf as ArrayBuffer))
  const writer = new StreamWriter()

  for await (const sheet of reader as unknown as AsyncIterable<JsStreamSheet>) {
    writer.writeSheet(sheet)
  }

  const out = writer.finalize()
  expect(out).toBeInstanceOf(Buffer)

  // Read back and verify cell values
  const reader2 = new StreamReader(out)
  const sheets: JsStreamSheet[] = []
  for await (const sheet of reader2 as unknown as AsyncIterable<JsStreamSheet>) {
    sheets.push(sheet)
  }

  expect(sheets).toHaveLength(2)
  expect(sheets[0].name).toBe('Data')
  expect(sheets[1].name).toBe('Empty')

  // Streaming sheet shape: { name, rows: [{ r, cells: [{ col, value }] }] }
  const dataSheet = sheets[0]
  // Streaming cell values are wrapped: { text }, { number }, { boolean }
  expect(dataSheet.rows[0].cells[0].value.text).toBe('hello')
  expect(dataSheet.rows[0].cells[1].value.number).toBe(42)
  expect(dataSheet.rows[1].cells[0].value.boolean).toBe(true)
  expect(dataSheet.rows[1].cells[1].value.number).toBeCloseTo(3.14)
})

// ---------------------------------------------------------------------------
// 5.2 Constant-memory: observe one sheet materialized at a time
// ---------------------------------------------------------------------------

test('5.2 streams sheets one at a time (not all at once)', async () => {
  const wbjs = new ExcelJS.Workbook()
  for (let i = 0; i < 5; i++) {
    const ws = wbjs.addWorksheet(`Sheet${i}`)
    ws.getCell('A1').value = `data-${i}`
  }
  const buf = await wbjs.xlsx.writeBuffer()

  const reader = new StreamReader(Buffer.from(buf as ArrayBuffer))
  const observed: string[] = []
  let concurrentCount = 0
  let maxConcurrent = 0

  for await (const sheet of reader as unknown as AsyncIterable<JsStreamSheet>) {
    concurrentCount++
    maxConcurrent = Math.max(maxConcurrent, concurrentCount)
    observed.push(sheet.name)
    concurrentCount--
  }

  // Sheets were observed one at a time
  expect(observed).toEqual(['Sheet0', 'Sheet1', 'Sheet2', 'Sheet3', 'Sheet4'])
  expect(maxConcurrent).toBe(1)
})

// ---------------------------------------------------------------------------
// 5.3 Backpressure: slow consumer still completes correctly
// ---------------------------------------------------------------------------

test('5.3 slow consumer still completes correctly', async () => {
  const wbjs = new ExcelJS.Workbook()
  for (let i = 0; i < 5; i++) {
    wbjs.addWorksheet(`Sheet${i}`)
  }
  const buf = await wbjs.xlsx.writeBuffer()

  const reader = new StreamReader(Buffer.from(buf as ArrayBuffer))
  const collected: number[] = []

  for await (const sheet of reader as unknown as AsyncIterable<JsStreamSheet>) {
    // Simulate slow processing
    await new Promise((resolve) => setTimeout(resolve, 5))
    collected.push(sheet.rows.length)
  }

  expect(collected).toHaveLength(5)
})

// ---------------------------------------------------------------------------
// 5.4 Mid-stream error: invalid zip aborts cleanly
// ---------------------------------------------------------------------------

test('5.4 invalid input aborts with clean error in constructor', async () => {
  const garbage = Buffer.from('this is not an xlsx file')
  expect(() => new StreamReader(garbage)).toThrow()
})

// ---------------------------------------------------------------------------
// 5.5 Empty workbook round-trip
// ---------------------------------------------------------------------------

test('5.5 empty workbook round-trips through streaming', async () => {
  const wbjs = new ExcelJS.Workbook()
  wbjs.addWorksheet('Empty')
  const buf = await wbjs.xlsx.writeBuffer()

  const reader = new StreamReader(Buffer.from(buf as ArrayBuffer))
  const sheets: JsStreamSheet[] = []
  for await (const sheet of reader as unknown as AsyncIterable<JsStreamSheet>) {
    sheets.push(sheet)
  }

  expect(sheets).toHaveLength(1)
  expect(sheets[0].name).toBe('Empty')
  // Empty sheet has no rows
  expect(sheets[0].rows.length).toBe(0)
})

// ---------------------------------------------------------------------------
// 4.1 Bridge functions (TS wrapper) — round-trip + stream termination
// ---------------------------------------------------------------------------

test('4.1 read/write bridge round-trips cell values', async () => {
  const wbjs = new ExcelJS.Workbook()
  const ws = wbjs.addWorksheet('Data')
  ws.getCell('A1').value = 'bridge'
  ws.getCell('B1').value = 7
  const buf = Buffer.from(await wbjs.xlsx.writeBuffer())

  const out = await write(read(buf))
  expect(out).toBeInstanceOf(Buffer)

  const reader = new StreamReader(out)
  const sheets: JsStreamSheet[] = []
  for await (const s of reader as unknown as AsyncIterable<JsStreamSheet>) sheets.push(s)
  expect(sheets).toHaveLength(1)
  expect(sheets[0].rows[0].cells[0].value.text).toBe('bridge')
  expect(sheets[0].rows[0].cells[1].value.number).toBe(7)
})

test('4.1 readAsReadable emits sheets', async () => {
  const wbjs = new ExcelJS.Workbook()
  wbjs.addWorksheet('Data').getCell('A1').value = 'r'
  const buf = Buffer.from(await wbjs.xlsx.writeBuffer())

  const readable = readAsReadable(buf)
  const sheets: JsStreamSheet[] = []
  for await (const s of readable as unknown as AsyncIterable<JsStreamSheet>) sheets.push(s)
  expect(sheets).toHaveLength(1)
  expect(sheets[0].name).toBe('Data')
})

test('4.1 writeToWritable ends the destination stream', async () => {
  const wbjs = new ExcelJS.Workbook()
  wbjs.addWorksheet('Data').getCell('A1').value = 'hi'
  const buf = Buffer.from(await wbjs.xlsx.writeBuffer())

  const pass = new PassThrough()
  const chunks: Buffer[] = []
  let finished = false
  pass.on('data', (c: Buffer) => chunks.push(c))
  pass.on('finish', () => { finished = true })

  await writeToWritable(read(buf), pass)

  expect(finished).toBe(true)
  const out = Buffer.concat(chunks)
  expect(out.length).toBeGreaterThan(0)
  const reader = new StreamReader(out)
  const sheets: JsStreamSheet[] = []
  for await (const s of reader as unknown as AsyncIterable<JsStreamSheet>) sheets.push(s)
  expect(sheets).toHaveLength(1)
  expect(sheets[0].name).toBe('Data')
})