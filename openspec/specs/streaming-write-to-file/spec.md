# streaming-write-to-file Specification

## Purpose
Allows the streaming XLSX writer to emit a constant-memory `.xlsx` directly to disk.
## Requirements
### Requirement: Streaming writer exposes finalizeToFile for a file path

The streaming writer SHALL provide a `finalizeToFile(file_path: string)` method that emits a
valid `.xlsx` to the given file path on disk. Its input- and output-phase memory behavior
SHALL be as stated by the two-phase model in `openspec/specs/streaming-xlsx/spec.md`. True
incremental `writeSheet()` remains deferred per
`openspec/specs/streaming-write-incremental/spec.md` and
`docs/adr/028-streaming-write-buffering.md`.

#### Scenario: finalizeToFile writes a valid xlsx to the path

- **WHEN** `finalizeToFile("/tmp/out.xlsx")` is called
- **THEN** a valid `.xlsx` file SHALL exist at that path

#### Scenario: finalizeToFile buffers input sheets in the handle

- **WHEN** sheets are accumulated via `writeSheet()` before `finalizeToFile`
- **THEN** peak process memory SHALL be O(all sheets), as stated by the `streaming-xlsx` two-phase model

### Requirement: finalizeToFile streams output with bounded backpressure

During the output phase, the writer SHALL write each zip file entry to disk as its sheet XML
is produced via incremental zip file-entry flushing, holding only one sheet's XML plus the
shared-strings and style accumulators in memory at a time.

#### Scenario: Output phase holds one sheet at a time

- **WHEN** `finalizeToFile` writes sheets to disk
- **THEN** at most one sheet's XML plus the shared-strings and style accumulators SHALL be held in memory at a time
