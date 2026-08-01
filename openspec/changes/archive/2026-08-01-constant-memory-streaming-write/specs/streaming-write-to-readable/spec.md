## Purpose

Lets the streaming XLSX writer emit a constant-memory `.xlsx` as a JS ReadableStream.

## ADDED Requirements

### Requirement: Streaming writer can produce a JS ReadableStream of chunks

The streaming writer SHALL provide a `finalizeToReadable()` method that returns a
JS `ReadableStream` yielding compressed zip chunk `Buffer`s. The writer SHALL
write each zip file entry as its sheet XML is produced and pipe the compressed
bytes through the stream with bounded backpressure, so that the process does not
hold the full workbook XML or full compressed archive in memory at any point.
At no point shall the process buffer more than one sheet's XML worth of cell data
plus the shared-strings and style accumulators.

#### Scenario: ReadableStream yields valid xlsx when piped to a file

- **WHEN** sheets are added incrementally and `finalizeToReadable()` is consumed by piping to a file writable (`readable.pipeTo(fileHandle.createWriteStream())` or equivalent)
- **THEN** a valid `.xlsx` file is produced that round-trips through the whole-workbook reader

#### Scenario: ReadableStream is constant-memory

- **WHEN** a workbook with many sheets is written via `finalizeToReadable()`
- **THEN** peak process memory stays bounded by one sheet's cell data plus the shared-strings and style tables, regardless of total workbook size

#### Scenario: Backpressure is honored

- **WHEN** the JS consumer reads chunks slower than the writer produces them
- **THEN** the writer's production is paused (bounded channel backpressure) until the consumer catches up — the writer does not run ahead and buffer unbounded output

#### Scenario: ReadableStream closes on completion

- **WHEN** all sheets have been streamed and `finalize`-equivalent metadata parts emitted
- **THEN** the returned `ReadableStream` closes cleanly (the JS `reader.read()` loop receives `{ done: true }`)

#### Scenario: ReadableStream propagates write errors

- **WHEN** an error occurs during zip writing (e.g., I/O failure on the internal channel)
- **THEN** the returned `ReadableStream` errors (the JS `reader.read()` promise rejects) and the stream terminates
