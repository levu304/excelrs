// ---------------------------------------------------------------------------
// Hand-written streaming bridge — adapts native pull primitives into
// Node Readable / Writable / AsyncIterable.
//
// This file is NOT auto-generated. It re-exports the native bindings and
// provides the public streaming API functions.
// ---------------------------------------------------------------------------

import { Readable, Writable } from 'node:stream'
import type { JsStreamSheet } from '../index'

// ---------------------------------------------------------------------------
// read() — returns an AsyncIterable<JsStreamSheet>
// ---------------------------------------------------------------------------

/**
 * Read an .xlsx buffer as an async iterable of sheets.
 *
 * Each iteration yields one `JsStreamSheet` (sheet-level granularity,
 * values-only, no styles). Only one sheet is materialized at a time.
 *
 * @example
 * ```ts
 * import { read } from '@levu304/excelrs/stream-bridge'
 * for await (const sheet of read(buffer)) {
 *   console.log(sheet.name, sheet.rowCount)
 * }
 * ```
 */
export function read(buffer: Buffer): AsyncIterable<JsStreamSheet> {
  // eslint-disable-next-line @typescript-eslint/no-require-imports
  const native = require('../index') as typeof import('../index')
  const reader = new native.StreamReader(buffer)
  return reader as unknown as AsyncIterable<JsStreamSheet>
}

// ---------------------------------------------------------------------------
// write() — accepts an AsyncIterable and returns Buffer
// ---------------------------------------------------------------------------

/**
 * Write an async iterable of sheets to an .xlsx buffer.
 *
 * The caller can produce sheets incrementally — they are consumed one at a
 * time and do not all need to reside in memory simultaneously.
 *
 * @example
 * ```ts
 * import { read, write } from '@levu304/excelrs/stream-bridge'
 * const output = await write(read(inputBuffer))
 * ```
 */
export async function write(sheets: AsyncIterable<JsStreamSheet>): Promise<Buffer> {
  // eslint-disable-next-line @typescript-eslint/no-require-imports
  const native = require('../index') as typeof import('../index')
  const writer = new native.StreamWriter()
  for await (const sheet of sheets) {
    writer.writeSheet(sheet)
  }
  return writer.finalize()
}

// ---------------------------------------------------------------------------
// readAsReadable() — returns a Node Readable
// ---------------------------------------------------------------------------

/**
 * Read an .xlsx buffer as a Node `Readable` that emits `JsStreamSheet` objects.
 */
export function readAsReadable(buffer: Buffer): Readable {
  return Readable.from(read(buffer))
}

// ---------------------------------------------------------------------------
// writeToWritable() — accepts an AsyncIterable and streams to a Writable
// ---------------------------------------------------------------------------

/**
 * Write an async iterable of sheets to a Node `Writable`.
 */
export async function writeToWritable(
  sheets: AsyncIterable<JsStreamSheet>,
  writable: Writable,
): Promise<void> {
  const buf = await write(sheets)
  return new Promise((resolve, reject) => {
    writable.write(buf, (err) => {
      if (err) reject(err)
      else resolve()
    })
  })
}