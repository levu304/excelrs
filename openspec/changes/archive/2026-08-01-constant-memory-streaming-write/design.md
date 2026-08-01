## Context

See `proposal.md` — Why. The current `stream_write` in `src/stream.rs:889` is a
two-pass implementation: pass 1 collects `sheet_emits: Vec<StreamSheet>` (every
cell's XML emit), `cell_styles`/`row_styles` (one entry per cell/row), and the
shared-strings table; pass 2 writes everything to `ZipWriter<Cursor<Vec<u8>>>`.

The `streaming-xlsx` spec (`openspec/specs/streaming-xlsx/spec.md`) already
promises "without buffering the whole workbook in memory" — the current
implementation violates this.

**Constraints**:

- `xl/sharedStrings.xml` and `xl/styles.xml` must appear after all sheets (they
  aggregate cross-sheet data). Zip format allows parts in any order —
  `ZipWriter::finish()` writes the central directory last, mapping names →
  offsets regardless of entry order.
- `zip` crate v7.2.0 (in `Cargo.lock`):
  - `ZipWriter::new(W: Write + Seek)` + `set_flush_on_finish_file(true)` for
    file-backed incremental flushing (`File` implements `Read + Write + Seek`).
  - `ZipWriter::new_stream(W: Write)` for non-seekable forward-only writing
    (channel/pipe output).
- `build_style_table` (`src/writer/styles.rs:83`) → `StyleTable` with fields
  `fonts`, `fills`, `borders`, `num_fmts`, `alignments`, `cell_xfs`,
  `cell_indices`, `dxfs`. `emit_styles_xml` (`src/writer/styles.rs:~245`)
  consumes all fields EXCEPT `cell_indices`. So an incremental `StyleAccumulator`
  that produces `cell_xfs` and sub-tables inline, then a `StyleTable` at
  finalize, is sufficient.
- napi-rs v3.10.3: `ReadableStream::create_with_stream_bytes` bridges a Rust
  `Stream<Item = Result<B>>` to JS `ReadableStream`. `WriteableStream` only
  exposes `ready()`, `close()`, `abort()` — no `write()` — so JS `WritableStream`
  cannot be the direct sink from Rust; the `ReadableStream` push model is the
  supported path.

## Goals / Non-Goals

**Goals:**

- Eliminate `sheet_emits` double-buffering in the `finalize()` → `Buffer` path.
- Add `finalizeToFile(path)` as a true constant-memory disk writer.
- Add `finalizeToReadable()` as a true constant-memory JS `ReadableStream` writer.
- Produce byte-identical output to the whole-workbook writer (`workbook_to_bytes`).
- Keep existing round-trip tests green.

**Non-Goals:**

- Accepting a JS `WritableStream` as the direct Rust sink (napi-rs limitation; defer
  unless user explicitly needs it).
- Fixing `WorkbookStreamXlsx.read()` eager materialization (separate change — see
  proposal).
- 1904-date-system handling (pre-existing P2 issue, orthogonal).

## Decisions

### Decision 1: `StyleAccumulator` mirrors `build_style_table` exactly

The incremental style accumulator uses the SAME dedup maps (`BTreeMap<canonical_key, u32>`)
and the SAME canonical key function. `CellXf` derives `Ord` (via `#[derive(Ord, Eq, PartialOrd, PartialEq)]`), so `BTreeMap<CellXf, u32>` produces identical xf ID assignment. `cell_xfs` Vec grows by first-occurrence in document order — identical to batch.

- *Alternative*: Keep `build_style_table` batch, post-process → rejected. Still
  needs all styles in memory, defeats constant-memory.
- *Risk*: If `CellXf` didn't derive `Ord`, we'd need `BTreeMap<String, u32>`
  keyed by canonical JSON of the tuple — verified it derives `Ord` correctly.

### Decision 2: Part reordering — sheets first, metadata last

Zip parts written in order: `[Content_Types].xml`, `_rels/.rels`,
`xl/workbook.xml`, `xl/worksheets/sheetN.xml` (per sheet, streamed), then
`xl/sharedStrings.xml`, `xl/styles.xml`, `xl/_rels/workbook.xml.rels`,
`xl/theme/theme1.xml`, media, etc. — then `ZipWriter::finish()` writes central
directory.

- *Why not stream central directory at end with `new_stream`*: The zip end-of-
  central-directory record is written by `finish()`. For `new_stream` (non-seekable),
  `finish_into_stream()` consumes the writer and returns remaining data.
- *Alternative*: Use `zip` crate's `write::ZipWriter` with `Cursor<Vec<u8>>`
  for the Buffer path (seekable, allows any order). Confirmed `set_flush_on_finish_file`
  only on `impl A: Read + Write + Seek`, not on `new_stream` variant — but file
  path doesn't need it since `File` is seekable.

### Decision 3: `spawn_blocking` for all output paths

`ZipWriter` writing is synchronous CPU + I/O. `#[napi]` async functions run on
napi-rs's tokio worker threads. For `finalizeToFile` and `finalizeToReadable`,
the zip writing (which can take seconds for large workbooks) MUST run on
`tokio::task::spawn_blocking` to avoid starving the JS event loop.

```
finalizeToFile(path) ─→ spawn_blocking ─→ ZipWriter::new(File) ─→ write parts ─→ finish()
finalizeToReadable() ─→ spawn_blocking ─→ ZipWriter::new_stream(ChannelWriter) ─→ write parts ─→ finish_into_stream
```

### Decision 4: `ReadableStream` output via bounded channel (not `WritableStream`)

Since `WriteableStream` has no `write()`, we emit a JS `ReadableStream`:

1. `tokio::sync::mpsc::channel::<Result<Vec<u8>, Error>>(16)` — bounded for backpressure.
2. `ChannelWriter<W>` implements `std::io::Write`, calling `sender.blocking_send()`
   on each `write()` call (blocks `spawn_blocking` thread until JS consumer pulls).
3. `tokio_stream::wrappers::ReceiverStream` (or manual `futures_core::Stream` impl)
   wraps the receiver.
4. `ReadableStream::create_with_stream_bytes(env, stream)` bridges to JS.

- *Alternative*: Add `tokio-stream` dependency (35KB, zero-risk) vs. manual
  `Stream` impl over `tokio::sync::mpsc::Receiver::poll_recv` (avoids dep, ~15 lines).
  **Decision**: manual impl — `futures_core::Stream` is already available via
  napi-rs, `tokio::sync::mpsc::Receiver::poll_recv` is stable. No new dependency.

### Decision 5: `finalize()` → `Buffer` keeps `ZipWriter<Cursor<Vec<u8>>>`

The Buffer-return path can't avoid materializing the full output (that's the
contract of returning a `Buffer`). But it eliminates the intermediate `sheet_emits`
double-buffer: sheets are written directly to `ZipWriter<Cursor>` during
iteration, no collection step.

## Risks / Trade-offs

| Risk | Mitigation |
| --- | --- |
| **Style table output drift** — incremental `StyleAccumulator` produces different `cellXfs` or sub-table ordering than batch `build_style_table` | BTreeMap dedup is deterministic (key-sorted), `cell_xfs` by first-occurrence — verified identical. Existing round-trip tests must pass unchanged. |
| **`spawn_blocking` + tokio channel deadlock** — if JS consumer of `ReadableStream` is slow and channel fills, `spawn_blocking` thread blocks on `blocking_send`. JS event loop can't pull because it's... actually JS event loop is free to pull (napi-rs handles pulls on async worker threads, not blocking). No deadlock. | Bounded channel (16 entries) provides natural backpressure. Channel capacity tunes memory ceiling. |
| **`set_flush_on_finish_file` not on `new_stream`** — ChannelWriter path can't use it | `new_stream` mode is inherently forward-only (seeks return error), so data flows to channel immediately. No flush needed — channel send IS the flush point. |
| **Existing JS `writeToWritable` API breaks** | `writeToWritable` in `src/stream-bridge.ts` currently buffers full Buffer then pipes. Update it to either: (a) call `finalizeToFile` then stream the file, or (b) call `finalizeToReadable` and pipe to the writable. Option (b) is cleaner — no intermediate file on disk. |
| **`WorkbookStreamXlsx.read()` eager materialization** | Deferred to separate change. Documented in proposal. |

## Migration Plan

1. **Refactor `stream_write` → `StreamSession`** (Level 1): Extract the two-pass
   body into a `StreamSession` struct with `write_sheet()` (incremental) +
   `finalize_to_bytes()` (Buffer), `finalize_to_file(path)`, `finalize_to_readable()`.
   Existing `stream_write(&[StreamSheet]) -> Vec<u8>` delegates to
   `StreamSession::from_sheets()` + `finalize_to_bytes()`.

2. **Add `StyleAccumulator`** in `src/writer/styles.rs`: struct with same fields
   as the intermediate state of `build_style_table`, plus `register(&style) -> u32`
   method. `finalize()` produces `StyleTable`.

3. **Add `#[napi]` methods on `StreamWriter`** (`src/stream_handle.rs`):
   `finalize_to_file(path: String)` and `finalize_to_readable() -> ReadableStream`.

4. **Update `stream-bridge.ts`**: `writeToWritable` pipes to `finalizeToReadable()`
   result instead of buffering then piping.

5. **Test**: All existing round-trip tests must pass. New tests:
   `finalizeToFile` round-trip, `finalizeToReadable` pipe-to-file round-trip,
   memory bounding test (large workbook peak < one sheet + tables).

**Rollback**: All changes are additive — `finalize()` → `Buffer` path unchanged.
No on-disk format change (output is byte-identical).

## Open Questions

- **Should `finalizeToReadable` use `tokio-stream` or manual `Stream` impl?**
  (see Decision 4 — leaning manual, needs confirmation.)
- **Should `finalizeToFile` return the file path or `void`?** Leaning `void`
  (simpler; JS caller knows the path they passed).
- **Channel capacity for `finalizeToReadable`?** 16 chunks (~64KB) is a reasonable
  default backpressure window; could be configurable later.
