## Why

Three shipped streaming specs promise constant-memory *write* I/O:

- `streaming-xlsx` — "without buffering the whole workbook in memory"
- `streaming-write-to-file` — "At no point shall the process buffer more than one sheet's XML worth of cell data..."
- `streaming-write-to-readable` — same "at no point shall the process buffer more than one sheet's XML worth of cell data..."

The shipped implementation does **not** honor that for the *input* phase. `StreamWriter::write_sheet` accumulates every sheet into `sheets: Vec<StreamSheet>` before any zip entry is written (`src/stream_handle.rs`). Only the *output* phase is constant-memory (cap-16 bounded mpsc channel, one sheet's XML + shared-strings/style accumulators at a time), per the v2.6.0 design and `docs/adr/005-streaming-write-buffering.md`.

This mismatch is a real footgun: a reader of `streaming-write-to-file/spec.md` piping a 2 GB workbook through `writeToWritable` hits O(all sheets) memory in the handle, not the "one sheet's XML at any point" the spec guarantees. Honest signals already exist in `index.d.ts` (StreamWriter doc-comments), `src/stream_handle.rs`, `src/stream-bridge.ts`, ADR-005, the deferred `streaming-write-incremental` spec, and the v2.6.0 CHANGELOG — only three spec files lag.

## What Changes

Tighten the writer memory-model wording in the three streaming specs to the
already-shipped two-phase reality, and cross-reference the deferred
`streaming-write-incremental` target. **No code, no behavior, no API
change** — the implementation is already correct and honest.

## Capabilities

### Modified Capabilities

Requirements (not implementation) change in three existing specs:

- `streaming-xlsx` — requirement "Streaming writer emits a workbook to a byte
  stream": phrase the two-phase model (input buffered O(all sheets); output
  streamed with backpressure); defer true incremental `writeSheet()` to
  `streaming-write-incremental`.
- `streaming-write-to-file` — requirement "Streaming writer finalize directly
  to file path": qualify the constant-memory claim to the output phase; note
  input is buffered O(all sheets).
- `streaming-write-to-readable` — requirement "Streaming writer can produce a
  JS ReadableStream of chunks": same two-phase qualification + cross-reference.

No new capabilities.

## Impact

- Docs only. `openspec/specs/{streaming-xlsx,streaming-write-to-file,streaming-write-to-readable}/spec.md`
  requirement wording tightened to match `index.d.ts` / ADR-005.
- No source, no tests, no type changes (`index.d.ts` already states the
  input-buffered reality).
- Follow-up: archive merges the MODIFIED deltas into the main specs via
  `openspec archive`; re-run the streaming test suite to confirm zero
  behavioral change.
