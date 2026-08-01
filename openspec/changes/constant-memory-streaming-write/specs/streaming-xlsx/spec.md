## MODIFIED Requirements

### Requirement: Streaming writer emits a workbook to a byte stream

The streaming writer SHALL accept rows incrementally and emit a valid `.xlsx`
to a writable byte stream without buffering the whole workbook in memory. The
writer SHALL write each sheet's XML directly to the zip writer as that sheet is
provided (not collected into a per-cell emit buffer across all sheets first).
`sharedStrings.xml`, `styles.xml`, and workbook metadata parts SHALL be emitted
once at finalize time, after all sheet XML has been written.

The writer SHALL provide at least the following output targets:

- `finalize()` → `Buffer`: emits the `.xlsx` as an in-memory Buffer. This path
  is still constant-memory for intermediate buffers (no double-buffering of
  sheet emits), but the returned Buffer inherently materializes the full
  output in RAM.
- `finalizeToFile(path)`: emits the `.xlsx` directly to a file on disk with
  constant memory via incremental zip file-entry flushing.
- `finalizeToReadable()`: emits the `.xlsx` as a JS `ReadableStream` of
  compressed chunk Buffers with constant memory via bounded backpressure.

#### Scenario: Write rows incrementally to a stream

- **WHEN** rows are added one-by-one to the streaming writer and the output stream is consumed
- **THEN** a valid `.xlsx` is produced containing exactly the added rows with their cell values preserved

#### Scenario: Streaming write round-trips through the streaming reader

- **WHEN** a workbook produced by the streaming writer is read back by the streaming reader
- **THEN** the read-back rows and cell values match what was written

#### Scenario: finalizeToFile produces constant-memory output to disk

- **WHEN** sheets are added incrementally and `finalizeToFile(path)` is called
- **THEN** a valid `.xlsx` file is created at `path` without buffering the full workbook in memory

#### Scenario: finalizeToReadable produces constant-memory output as chunks

- **WHEN** sheets are added incrementally and `finalizeToReadable()` is consumed by piping to a writable
- **THEN** a valid `.xlsx` is produced as chunked output with bounded backpressure, without buffering the full workbook in memory
