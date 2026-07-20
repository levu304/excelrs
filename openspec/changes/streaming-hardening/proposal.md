## Why

A code review of PR #34 (`streaming-node-bridge`) surfaced confirmed, High-impact
defects in the streaming XLSX bridge. The feature is branded "constant-memory
streaming," but three of the issues contradict that promise or break correctness
on untrusted input:

- **`writeToWritable` never ends the stream** (`src/stream-bridge.ts:87`): it calls
  `writable.write(buf, cb)` but never `writable.end()`, so piped consumers (HTTP
  response, `pipeline`, fs) never receive `finish` and hang.
- **No zip-bomb protection** (`src/stream_handle.rs:265`): `ZipArchive::new` parses
  the entire central directory into memory with no entry-count or total-size cap. A
  crafted `.xlsx` with millions of tiny entries OOMs the process before any content
  is read — a DoS on untrusted input.
- **Read path re-materializes per sheet** (`src/stream_handle.rs:320` clones the full
  file, `:331` re-parses the central directory on every `next()`): peak memory and
  CPU scale with `sheets × file_size`, undermining the constant-memory claim.

Secondary gaps: the TS bridge functions (`read`/`write`/`readAsReadable`/
`writeToWritable`) have **zero tests**, and README/comments falsely claim the write
path streams (it buffers every sheet into `StreamWriter.sheets` then builds the
whole archive in RAM).

These are correctness/safety hardening, not new features. They should land as a
follow-up to PR #34 before the streaming feature is advertised as production-ready.

## Changes

- Fix `writeToWritable` to call `writable.end(buf, cb)` so destination streams
  terminate.
- Add zip-bomb protection: cap zip entry count and total byte size in `StreamReader`
  (and `stream_read`) before/at `ZipArchive::new`.
- Tighten read memory: hold file bytes as `Arc<[u8]>`, open the zip archive **once**
  in the constructor, and reuse it across `next()` calls; wrap `shared`/`style_table`
  in `Arc` to avoid per-sheet clones.
- Add tests for the TS bridge functions and a zip-bomb entry-count regression test.
- Correct README/comments that claim the write path streams.

## Capabilities

### New Capabilities

- `streaming-safety`: safety and correctness guarantees for the streaming XLSX
  bridge — zip-bomb rejection on untrusted input, termination of destination
  streams, and per-sheet constant-memory reading.

### Modified Capabilities

<!-- none — existing streaming-xlsx behavior is unchanged; this change adds safety guarantees -->

## Impact

- **Files touched**: `src/stream_handle.rs` (`StreamReader` ctor + `next`),
  `src/stream.rs` (`stream_read` cap, new rust test), `src/stream-bridge.ts`
  (`writeToWritable`), `README.md`, `__test__/streaming-bridge.test.ts`,
  `__test__/streaming-bridge.test.ts` (or a new bridge test file).
- **Public API unchanged**: `StreamReader` constructor, `StreamWriter`, and
  `writeToWritable` keep their signatures; only internal memory handling and the
  `end()` call change. No napi type changes → glue script unaffected.
- **Dependencies**: none added. Uses existing `zip` v7 and `std::sync::Arc`.
- **Out of scope (separate change)**: true constant-memory *write* to a `Writable`
  (requires a streaming zip encoder that does not need `Seek` + inline-string
  cells). The current write path remains buffered-by-design; docs will state this.
