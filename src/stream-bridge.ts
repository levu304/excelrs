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
 *   console.log(sheet.name, sheet.rows.length)
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
 * The caller can produce sheets incrementally. Note: every sheet is buffered
 * in memory until `finalize()` builds the full archive, so the write path is
 * **not** constant-memory.
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
 *
 * Uses `finalizeToReadable()` to emit the archive with backpressure (a
 * bounded channel drained by the consumer), then pipes chunks into the
 * `Writable`. Input sheets are still buffered in the writer handle first, so
 * the write path is not fully constant-memory — see `finalizeToReadable` and
 * `openspec/specs/streaming-write-incremental/spec.md`.
 */
export async function writeToWritable(
  sheets: AsyncIterable<JsStreamSheet>,
  writable: Writable,
): Promise<void> {
  const native = require('../index') as typeof import('../index')
  const writer = new native.StreamWriter()
  for await (const sheet of sheets) {
    writer.writeSheet(sheet)
  }
  if (typeof writer.finalizeToReadable === 'function') {
    const readable = writer.finalizeToReadable()
    // `finalizeToReadable()` yields a Web `ReadableStream` of compressed zip
    // chunks. Drain it via the reader API and feed chunks into the Node
    // `Writable`, honoring backpressure (pause when `write` returns false).
    return new Promise<void>((resolve, reject) => {
      const reader = readable.getReader()
      const ignoreErr = () => undefined
      const cleanup = () => {
        // Abort the Web ReadableStream so the native Rust stream source drops
        // promptly (without this it lingers until JS GC — the ~55-60s leak).
        // Per the Web Streams spec: cancel() throws "Invalid state: locked"
        // while a reader holds the lock, and releaseLock() throws while a
        // read() is in flight. Both are best-effort here: in the happy path a
        // read is already settled, so releaseLock() succeeds and cancel()
        // drops the source promptly (B2). If a read is in flight (abandon
        // mid-read), both throw → fall back to GC, and the Rust layer's
        // try_send + is_closed reaps the worker once collected (B1, no
        // park-forever). Idempotent across double-cleanup.
        try { reader.releaseLock() } catch { ignoreErr() }
        try { void readable.cancel().catch(ignoreErr) } catch { ignoreErr() }
      }
      const pump = () => {
        reader.read().then(({ done, value }) => {
          if (done) {
            writable.end()
            return
          }
          if (!writable.write(value)) {
            writable.once('drain', pump)
          } else {
            pump()
          }
        }, (err) => {
          cleanup()
          reject(err)
        })
      }
      writable.once('error', (err) => { cleanup(); reject(err) })
      writable.once('close', () => { cleanup(); resolve() })
      pump()
    })
  }
  const buf = await writer.finalize()
  return new Promise((resolve, reject) => {
    writable.once('error', reject)
    writable.once('finish', resolve)
    writable.end(buf)
  })
}