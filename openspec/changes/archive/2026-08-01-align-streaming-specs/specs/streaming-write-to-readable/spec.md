## MODIFIED Requirements

### Requirement: Streaming writer can produce a JS ReadableStream of chunks

The streaming writer SHALL provide a `finalizeToReadable()` method that returns a
JS `ReadableStream` yielding compressed zip chunk `Buffer`s.

**Output is streamed with backpressure:** the writer writes each zip file entry as
its sheet XML is produced and pipes the compressed bytes through the stream via a
bounded mpsc channel (cap 16), so that the *output-emission* phase never holds
more than one sheet's XML worth of cell data plus the shared-strings and style
accumulators in memory at a time.

**Input is NOT constant-memory:** sheets provided to `writeSheet()` are
accumulated in the `StreamWriter` handle before `finalizeToReadable` is called,
so peak memory is O(all sheets). The phrase "at no point shall the process
buffer more than one sheet's XML worth of cell data" applies to the output phase
only; the input phase buffers all sheets. True constant-memory incremental
`writeSheet()` (each sheet written to the zip as it arrives, before `finalize`)
is deferred — see `openspec/specs/streaming-write-incremental/spec.md` and
`docs/adr/005-streaming-write-buffering.md`.

#### Scenario: ReadableStream yields valid xlsx when piped to a file

- **WHEN** sheets are added incrementally and `finalizeToReadable()` is consumed by piping to a file writable (`readable.pipeTo(fileHandle.createWriteStream())` or equivalent)
- **THEN** a valid `.xlsx` file is produced that round-trips through the whole-workbook reader

#### Scenario: ReadableStream is constant-memory

- **WHEN** a workbook with many sheets is written via `finalizeToReadable()`
- **THEN** the output phase is constant-memory (one sheet's XML + accumulators at a time, with cap-16 backpressure); however input sheets were buffered in the handle first, so peak process memory is O(all sheets), not one sheet's worth — true incremental `writeSheet()` deferred, see `openspec/specs/streaming-write-incremental/spec.md`

#### Scenario: Backpressure is honored

- **WHEN** the JS consumer reads chunks slower than the writer produces them
- **THEN** the writer's production is paused (bounded channel backpressure) until the consumer catches up — the writer does not run ahead and buffer unbounded output

#### Scenario: ReadableStream closes on completion

- **WHEN** all sheets have been streamed and the `finalize`-equivalent metadata parts (sharedStrings, styles, central directory) have been emitted
- **THEN** the returned `ReadableStream` closes cleanly (the JS `reader.read()` loop receives `{ done: true }`)

#### Scenario: ReadableStream propagates write errors

- **WHEN** an error occurs during zip writing (e.g., I/O failure on the internal channel)
- **THEN** the returned `ReadableStream` errors (the JS `reader.read()` promise rejects) and the stream terminates
