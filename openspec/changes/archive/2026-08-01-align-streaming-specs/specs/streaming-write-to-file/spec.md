## MODIFIED Requirements

### Requirement: Streaming writer can finalize directly to a file path

The streaming writer SHALL provide a `finalizeToFile(file_path: string)` method
that emits a valid `.xlsx` to the given file path on disk.

**Output is streamed with backpressure:** the writer writes each zip file entry to
disk as its sheet XML is produced (incremental zip file-entry flushing), holding
only one sheet's XML plus the shared-strings and style accumulators in memory at
a time during the *output* phase.

**Input is NOT constant-memory:** sheets provided to `writeSheet()` are
accumulated in the `StreamWriter` handle before `finalizeToFile` is called, so
peak write memory is O(all sheets). The phrase "at no point shall the process
buffer more than one sheet's XML" applies to the output phase only. True
constant-memory incremental `writeSheet()` (each sheet written to the zip as it
arrives) is deferred — see `openspec/specs/streaming-write-incremental/spec.md`
and `docs/adr/005-streaming-write-buffering.md`.

#### Scenario: Finalize to file produces valid xlsx

- **WHEN** sheets are added incrementally and the streaming writer `finalizeToFile("/tmp/out.xlsx")` is called
- **THEN** a valid `.xlsx` file is created at the path that round-trips through the whole-workbook reader

#### Scenario: Finalize to file is constant-memory

- **WHEN** a workbook with many sheets is written via `finalizeToFile`
- **THEN** output is written to disk one sheet's XML at a time (constant-memory in the output phase, bounded backpressure); however input sheets were buffered in the handle first, so peak memory is O(all sheets), not one sheet's worth — true incremental `writeSheet()` deferred, see `openspec/specs/streaming-write-incremental/spec.md`

#### Scenario: Finalize to file round-trips through streaming reader

- **WHEN** a workbook produced by `finalizeToFile` is read back by the streaming reader
- **THEN** the read-back rows and cell values match what was written

#### Scenario: Finalize to file rejects invalid paths

- **WHEN** `finalizeToFile` is called with a path in a non-existent directory or an unwritable location
- **THEN** the call rejects with an error and the file is not created (or left incomplete)
