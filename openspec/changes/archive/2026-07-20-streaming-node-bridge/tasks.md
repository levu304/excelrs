## 1. Rust core refactor (sheet-level)

- [x] 1.1 Refactor `stream_read` (`src/stream.rs`) from "collect `Vec<StreamSheet>`" to produce one sheet at a time — persist SAX parse state across calls so each `next()` yields exactly one `StreamSheet` (sheet-level, no full-workbook accumulation).
- [x] 1.2 Add incremental `stream_write` entry emission to a `Write` sink (`ZipWriter` streaming with data descriptors) so output is written sheet-by-sheet instead of building one `Buffer`.

## 2. napi bridge primitives

- [x] 2.1 Add `StreamReader` napi class (`src/stream_handle.rs`): constructor accepts `Buffer` or file path; `async next() -> Option<JsStreamSheet>`; holds parse state in `Arc<Mutex<...>>`; drives parse via `spawn_blocking` so it stays off the event loop.
- [x] 2.2 Add `StreamWriter` napi class: constructor accepts `Buffer` / `Writable` / file path; `async write_sheet(sheet)`; `async finalize()/close()`; streams entries via the incremental `stream_write` sink.
- [x] 2.3 Spike: confirm the cleanest napi-rs expression of the pull primitive (next()-class + JS `[Symbol.asyncIterator]` vs generated async generator) before the bulk of the Rust work.

## 3. JS adapter layer

- [x] 3.1 Add a hand-written wrapper module (imports `./native.js`) implementing `[Symbol.asyncIterator]` on `StreamReader`, `Readable.from(iter)` for read, and a `Writable` that drains an `AsyncIterable<StreamSheet>` for write.
- [x] 3.2 Point the package `main`/public entry at the wrapper; leave generated `index.js` / `native.js` untouched.
- [x] 3.3 Regenerate `index.d.ts` and update types for the new `read` (AsyncIterable + Readable) and `write` (AsyncIterable input, Writable/Buffer output) signatures.

## 4. Spec & docs

- [x] 4.1 Add `specs/streaming-node-bridge/spec.md` with requirements: `read` returns `AsyncIterable<StreamSheet>` (+ `Readable`), `write` accepts `AsyncIterable<StreamSheet>` and streams to `Writable`/Buffer, sheet-level granularity, seekable source (`Buffer` + file path), values-only.
- [x] 4.2 Update README / ROADMAP streaming section and close GitHub issue #25.1.

## 5. Tests

- [x] 5.1 Round-trip: write via `AsyncIterable` → read via `AsyncIterable`; assert cell values match per streamed sheet.
- [x] 5.2 Constant-memory assertion: verify not all sheets are held at once (e.g. stream a multi-sheet workbook and observe one sheet materialized at a time).
- [x] 5.3 Backpressure: a slow consumer still completes correctly (pull paces producer).
- [x] 5.4 Mid-stream error propagation: hostile input hitting #25.3 caps aborts the iterator with a clean JS error (no partial/leaked state).
- [x] 5.5 Integration: file-path read + `Writable`/file-path write round-trips through the streaming reader.
