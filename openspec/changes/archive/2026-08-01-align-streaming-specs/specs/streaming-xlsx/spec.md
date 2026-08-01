## MODIFIED Requirements

### Requirement: Streaming writer emits a workbook to a byte stream

The streaming writer SHALL emit a valid `.xlsx` to a writable byte stream. The
writer operates in two phases:

1. **Input phase** — sheets pushed via `writeSheet()` are accumulated in the
   `StreamWriter` handle (`sheets: Vec<StreamSheet>`) before any zip entry is
   written. Peak memory for this phase is **O(all sheets)**, NOT constant.
   True incremental `writeSheet()` (each sheet's XML written to the zip as it
   arrives, before `finalize`) is **deferred** — see
   `openspec/specs/streaming-write-incremental/spec.md` and
   `docs/adr/005-streaming-write-buffering.md`.

2. **Output phase** — `finalize`, `finalizeToFile`, `finalizeToReadable` emit
   the accumulated sheets directly to the `ZipWriter`, writing each sheet's XML
   to the zip as it is produced (not collected into a per-cell emit buffer
   across all sheets first) and piping compressed bytes through a bounded mpsc
   channel (cap 16). `sharedStrings.xml`, `styles.xml`, and workbook metadata
   parts are emitted once at finalize time, after all sheet XML has been
   written. Peak memory for this phase is constant (one sheet's XML + the
   shared-strings and style accumulators).

The writer SHALL provide at least the following output targets:

- `finalize()` → `Buffer`: emits the `.xlsx` as an in-memory Buffer. This path
  is still constant-memory for intermediate buffers (no double-buffering of
  sheet emits), but the returned Buffer inherently materializes the full
  output in RAM.
- `finalizeToFile(path)`: emits the `.xlsx` directly to a file on disk with
  constant memory in the *output* phase via incremental zip file-entry
  flushing (input sheets remain buffered in the handle).
- `finalizeToReadable()`: emits the `.xlsx` as a JS `ReadableStream` of
  compressed chunk Buffers with constant memory in the *output* phase via
  bounded backpressure (input sheets remain buffered in the handle).

#### Scenario: Write rows incrementally to a stream

- **WHEN** rows are added one-by-one to the streaming writer and the output stream is consumed
- **THEN** a valid `.xlsx` is produced containing exactly the added rows with their cell values preserved

#### Scenario: Streaming write round-trips through the streaming reader

- **WHEN** a workbook produced by the streaming writer is read back by the streaming reader
- **THEN** the read-back rows and cell values match what was written

#### Scenario: finalizeToFile produces constant-memory output to disk

- **WHEN** sheets are added incrementally and `finalizeToFile(path)` is called
- **THEN** a valid `.xlsx` file is created at the path; output is written with one sheet's XML in memory at a time (constant-memory output), though input sheets were buffered in the handle (peak O(all sheets)); true incremental `writeSheet()` is deferred — see `openspec/specs/streaming-write-incremental/spec.md`

#### Scenario: finalizeToReadable produces constant-memory output as chunks

- **WHEN** sheets are added incrementally and `finalizeToReadable()` is consumed by piping to a writable
- **THEN** a valid `.xlsx` is produced as chunked output with bounded backpressure, one sheet's XML in memory at a time (constant-memory output); input sheets were buffered in the handle (peak O(all sheets)); true incremental `writeSheet()` is deferred — see `openspec/specs/streaming-write-incremental/spec.md`
