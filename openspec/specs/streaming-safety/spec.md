# streaming-safety Specification

## Purpose

Safety and correctness guarantees for the streaming XLSX bridge: zip-bomb
rejection on untrusted input, termination of destination streams, and per-sheet
constant-memory reading. (Created by archiving change streaming-hardening.)

## Requirements

### Requirement: Streaming reader rejects zip-bomb inputs

The streaming reader (`StreamReader` and the batch `stream_read`) SHALL reject a
`.xlsx` whose parsed zip central directory exceeds a bounded entry count or total
byte size, returning an error instead of exhausting process memory. The check SHALL
occur after `ZipArchive::new` succeeds and before any worksheet content is read.

#### Scenario: Many-entry archive is rejected

- **WHEN** a `.xlsx` is supplied whose zip contains more than `MAX_ARCHIVE_ENTRIES` (10,000) entries
- **THEN** the streaming reader returns an error and does not allocate the full central directory's worth of memory

#### Scenario: Oversized archive is rejected

- **WHEN** a `.xlsx` is supplied whose total byte size exceeds `MAX_ARCHIVE_BYTES` (256 MB)
- **THEN** the streaming reader returns an error rather than materializing the input

#### Scenario: Legitimate workbook is accepted

- **WHEN** a `.xlsx` has fewer than `MAX_ARCHIVE_ENTRIES` entries and is under `MAX_ARCHIVE_BYTES`
- **THEN** the streaming reader parses it normally and yields its sheets

### Requirement: Streaming write terminates the destination stream

`writeToWritable(sheets, writable)` SHALL call `writable.end()` after writing the
output buffer so that piped consumers (HTTP responses, `pipeline`, filesystem
streams) receive `finish` and the promise resolves only after the stream closes.

#### Scenario: Piping to a PassThrough completes

- **WHEN** `writeToWritable` writes to a `PassThrough` (or any `Writable`)
- **THEN** the `Writable` emits `finish` and the returned promise resolves (it does not hang)

### Requirement: Streaming reader avoids per-sheet full re-materialization

The streaming reader SHALL NOT clone the entire input file or re-parse the zip
central directory for each yielded sheet. Peak memory across `next()` calls SHALL
stay bounded by a single sheet's content plus a constant overhead, independent of
the number of sheets in the workbook.

#### Scenario: Multi-sheet read keeps memory bounded

- **WHEN** a workbook with N sheets is stream-read
- **THEN** the reader opens the archive once and reuses it per sheet, allocating at most one sheet's worth of rows at a time (no N× file clone / re-parse)
