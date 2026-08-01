## Why

The streaming XLSX writer (`StreamWriter` in `src/stream_handle.rs`, `stream_write` in `src/stream.rs`) is documented (via `openspec/specs/streaming-xlsx/spec.md`) as producing a valid `.xlsx` "without buffering the whole workbook in memory." But the current `stream_write` implementation collects **all** sheet data into `sheet_emits: Vec<StreamSheet>` and **all** per-cell styles into `cell_styles`/`row_styles` before writing a single byte — a two-pass design that materializes the entire workbook's cell data in RAM. For large workbooks this violates the streaming contract and risks OOM.

The goal: make the streaming writer truly constant-memory — write each sheet's XML directly to the zip writer as it arrives, interning strings and styles inline, then emit `sharedStrings.xml`, `styles.xml`, and metadata parts once at `finalize()`. This requires an incremental `StyleAccumulator` (verified byte-identical to the batch `build_style_table`), reordered zip parts (sheets first, metadata last — valid since zip central directory maps by name), and `zip` crate v7.2.0 streaming APIs (`ZipWriter::new_stream` for non-seekable, `set_flush_on_finish_file` for file-backed incremental flush).

## What Changes

### Level 1: Incremental sheet writing (eliminate `sheet_emits` + `cell_styles`/`row_styles` buffers)

- Refactor `stream_write` into a `StreamSession` struct that holds:
  - Incremental string interning (`string_table: Vec<String>` + `string_indices: HashMap`) — kept inline per-cell during sheet streaming (no algorithm change; already structured this way in pass 1).
  - Incremental `StyleAccumulator` replacing `build_style_table(&[Option<Style>)` — assigns xf IDs inline via `BTreeMap<CellXf, u32>` dedup, verified to produce byte-identical `cell_xfs`/`fonts`/`fills`/`borders`/`num_fmts`/`alignments` ordering.
- Reorder zip parts: write `xl/worksheets/sheetN.xml` directly during sheet iteration (before sharedStrings/styles), emit `sharedStrings.xml`, `styles.xml`, `workbook.xml`, and metadata at finalize. This is valid per zip format — `ZipWriter::finish()` writes the central directory last, mapping names → offsets regardless of part order.
- **Eliminates** the `sheet_emits: Vec` intermediate buffer entirely. The `finalize()` → `Buffer` API still materializes the full output `Vec<u8>` (inherent to returning a `Buffer`), but no longer holds a second full copy of cell emits alongside it.

### Level 2-A: `finalizeToFile(path)` — true constant-memory disk output

- New `#[napi]` method on `StreamWriter`: `finalizeToFile(file_path: String) -> Result<()>`.
- Uses `ZipWriter::new(std::fs::File)` + `set_flush_on_finish_file(true)` — flushes each zip file entry to disk immediately, so only zip internal buffers (~KB) + string/style accumulators + one sheet's XML reside in RAM.
- Runs on `tokio::task::spawn_blocking` since `ZipWriter` is synchronous CPU+I/O (avoids blocking the JS event loop). Same tokio runtime already used by existing `#[napi]` async functions.
- Same part reordering and incremental tables as Level 1.

### Level 2-B: `finalizeToReadable()` — constant-memory JS `ReadableStream`

- New `#[napi]` method: `finalizeToReadable() -> ReadableStream<BufferSlice>`.
- Uses `ZipWriter::new_stream(ChannelWriter)` (non-seekable, forward-only) where `ChannelWriter: Write` calls `tokio::sync::mpsc::Sender::blocking_send` to a bounded channel (`[16]`).
- The tokio channel receiver is bridged to a JS `ReadableStream` via napi-rs's `ReadableStream::create_with_stream_bytes` — JS pulls chunks, Rust provides them, backpressure via bounded channel.
- **Constraint**: napi-rs v3.10.3's `WriteableStream` only exposes `ready()`, `close()`, `abort()` — no `write()`. So we cannot accept a JS `WritableStream` directly; instead we emit a JS `ReadableStream` that the user pipes externally (`readable.pipeTo(writable)`). This is the idiomatic JS pattern (ExcelJS does the same with `archiver`).
- Runs on `spawn_blocking`; the channel receiver is converted to a `futures_core::Stream` (manual impl over `tokio::sync::mpsc::Receiver`, since `tokio-stream` is not a direct dependency — or add `tokio-stream` dep).

### Fix `WorkbookStreamXlsx.read()` eager materialization

- `WorkbookStreamXlsx.read()` currently calls `stream_read()` which eagerly builds `Vec<StreamSheet>` (all sheets in RAM) — contradicts the streaming reader's own spec. Investigate aligning `read()` with the true async-iterator `StreamReader` pattern (per-sheet `next()`). **Tentative** — may defer to separate change if scope creeps.

## Capabilities

### New Capabilities

- **`streaming-write-to-file`**: The streaming writer SHALL accept sheets incrementally via `writeSheet()` and, on `finalizeToFile(path)`, emit a valid `.xlsx` to a file on disk with constant memory (no buffered cell emits, no buffered output). The file is written incrementally via zip file-entry flushing; at no point does the process hold the full workbook XML in memory.
- **`streaming-write-to-readable`**: The streaming writer SHALL, via `finalizeToReadable()`, produce a JS `ReadableStream` of compressed zip chunks with constant memory. The stream is consumed by JS piping (`readable.pipeTo(writable)` or `readable.getReader()`); backpressure is enforced by a bounded channel between the Rust zip writer thread and the JS event loop.
- **`streaming-write-incremental`**: The streaming writer SHALL generate each sheet's XML directly into the zip writer as `writeSheet()` is called, interning shared strings and accumulating styles inline, rather than collecting all sheet data before writing. `sharedStrings.xml`, `styles.xml`, and workbook metadata are emitted once at finalize, after all sheets.

### Modified Capabilities

- **`streaming-xlsx`**: The existing requirement "without buffering the whole workbook in memory" (currently aspirational/aspirational in spec, violated by current implementation) becomes actually true. The `finalize()` → `Buffer` path retains the same API (returns a `Buffer`) but eliminates the intermediate `sheet_emits` double-buffering. New `finalizeToFile`/`finalizeToReadable` APIs extend the streaming writer's surface with constant-memory output targets. The existing round-trip scenarios (write → read back → values match) remain unchanged.

## Impact

- **`src/stream.rs`** — `stream_write` (line 889): refactor two-pass body into `StreamSession` with per-sheet write + finalize phases. Add incremental `StyleAccumulator` usage. Keep existing `stream_read` round-trip tests passing (identical output).
- **`src/writer/styles.rs`** — Add `StyleAccumulator` struct mirroring `build_style_table`'s dedup logic (BTreeMap-based, same canonical keys). `emit_styles_xml` already consumes `StyleTable` fields; `StyleAccumulator` produces a compatible `StyleTable` at finalize.
- **`src/stream_handle.rs`** — `StreamWriter`: add `finalize_to_file(path)` and `finalize_to_readable()` `#[napi]` methods. `write_sheet()` now writes sheet XML directly during iteration instead of pushing to `self.sheets`. `finalize()` continues returning a `Buffer` (calls refactored `stream_write`).
- **`src/stream-bridge.ts`** — Update `writeToWritable` to use the new streaming write path (write sheets incrementally) instead of buffering the full Buffer then piping.
- **`Cargo.lock`** — May add `tokio-stream` if opting into `ReceiverStream` wrapper for Level 2-B (or implement `futures_core::Stream` manually).
- **Tests** — Existing round-trip tests in `__test__/streaming-bridge.test.ts` and `src/stream.rs:1220+` verify output is byte-identical. Add tests for `finalizeToFile` (file output round-trip) and `finalizeToReadable` (ReadableStream consumption round-trip).
